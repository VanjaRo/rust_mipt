#![forbid(unsafe_code)]

use crate::{
    block_forest::{self, BlockForest},
    data::{BlockHash, TransactionHash, VerifiedBlock, VerifiedPeerMessage, VerifiedTransaction},
    node::{
        mining_service::MiningInfo,
        peer_service::{PeerCommand, PeerCommandKind, PeerEvent, PeerEventKind, SessionId},
    },
};

use anyhow::{Context, Result};
use crossbeam::{
    channel::{self, never, tick, Receiver, RecvError, Sender},
    select,
};
use log::*;
use rand::{seq::SliceRandom, thread_rng};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use std::{
    collections::{HashMap, HashSet},
    thread,
    time::Duration,
};

////////////////////////////////////////////////////////////////////////////////

#[derive(Default, Serialize, Deserialize)]
pub struct GossipServiceConfig {
    #[serde(with = "humantime_serde")]
    pub eager_requests_interval: Duration,
}

pub struct GossipService {
    config: GossipServiceConfig,
    event_receiver: Receiver<PeerEvent>,
    command_sender: Sender<PeerCommand>,
    block_receiver: Receiver<VerifiedBlock>,
    mining_info_sender: Sender<MiningInfo>,
    block_forest: BlockForest,
    sessions_cache: SessionsCache,
}

#[derive(Default)]
struct SessionsCache {
    blocks: HashMap<SessionId, HashSet<BlockHash>>,
    txs: HashMap<SessionId, HashSet<TransactionHash>>,
}

impl GossipService {
    pub fn new(
        config: GossipServiceConfig,
        event_receiver: Receiver<PeerEvent>,
        command_sender: Sender<PeerCommand>,
        block_receiver: Receiver<VerifiedBlock>,
        mining_info_sender: Sender<MiningInfo>,
    ) -> Self {
        Self {
            config,
            event_receiver,
            command_sender,
            block_receiver,
            mining_info_sender,
            block_forest: BlockForest::new(),
            sessions_cache: SessionsCache::default(),
        }
    }

    pub fn run(&mut self) {
        let request_unknown_ticker = (|| {
            if self.config.eager_requests_interval.is_zero() {
                never()
            } else {
                tick(self.config.eager_requests_interval)
            }
        })();

        loop {
            self.send_mining_info();
            select! {
                recv(&self.event_receiver) -> msg => self.handle_peer_event(msg),
                recv(&self.block_receiver) -> msg => self.spread_mined_block(msg),
                recv(&request_unknown_ticker) -> _ => self.request_unknown_blocks(),
            }
        }
    }

    fn send_mining_info(&self) {
        let transactions = self
            .block_forest
            .pending_transactions()
            .values()
            .cloned()
            .collect::<Vec<VerifiedTransaction>>();

        let send_res = self.mining_info_sender.send(MiningInfo {
            block_index: self.block_forest.head().index + 1,
            prev_hash: self.block_forest.head().hash().clone(),
            max_hash: self.block_forest.next_max_hash(),
            transactions,
        });

        if let Err(e) = send_res {
            error!("error while sending mining info: {}", e);
        }
    }

    fn handle_peer_event(&mut self, peer_event_msg: Result<PeerEvent, RecvError>) {
        if let Err(e) = peer_event_msg {
            error!("unable to receive peer event msg: {}", e);
            return;
        }

        let PeerEvent {
            session_id,
            event_kind,
        } = peer_event_msg.unwrap();

        let cmds = match event_kind {
            PeerEventKind::Connected => self.new_session_cmds(session_id),
            PeerEventKind::Disconnected => self.terminate_session_cmds(session_id),
            PeerEventKind::NewMessage(msg) => self.new_message_cmds(msg, session_id),
        };
        // TODO: add logging
        cmds.into_iter().for_each(|peer_cmd| {
            self.command_sender
                .send(peer_cmd)
                .expect("unable to send peer comand")
        })
    }

    fn new_session_cmds(&mut self, session_id: SessionId) -> Vec<PeerCommand> {
        self.sessions_cache
            .blocks
            .insert(session_id, HashSet::new());
        self.sessions_cache.txs.insert(session_id, HashSet::new());

        let block_forest = &self.block_forest;
        // size = all pending + head
        let mut cmds = Vec::with_capacity(block_forest.pending_transactions().len() + 1);

        let head = block_forest.head();
        cmds.push(PeerCommand {
            session_id,
            command_kind: PeerCommandKind::SendMessage(VerifiedPeerMessage::Block(Box::new(
                head.as_ref().clone(),
            ))),
        });

        cmds.extend(
            block_forest
                .pending_transactions()
                .values()
                .map(|verif_tx| PeerCommand {
                    session_id,
                    command_kind: PeerCommandKind::SendMessage(VerifiedPeerMessage::Transaction(
                        Box::new(verif_tx.clone()),
                    )),
                }),
        );
        cmds
    }

    fn terminate_session_cmds(&mut self, session_id: SessionId) -> Vec<PeerCommand> {
        self.sessions_cache.blocks.remove(&session_id);
        self.sessions_cache.txs.remove(&session_id);

        vec![PeerCommand {
            session_id,
            command_kind: PeerCommandKind::Drop,
        }]
    }

    fn new_message_cmds(
        &mut self,
        msg: VerifiedPeerMessage,
        session_id: SessionId,
    ) -> Vec<PeerCommand> {
        match msg {
            VerifiedPeerMessage::Block(block_box) => self.new_block_cmds(block_box, session_id),
            VerifiedPeerMessage::Transaction(tx_box) => self.new_tx_cmds(tx_box, session_id),
            VerifiedPeerMessage::Request { block_hash } => {
                self.requested_block_cmd(block_hash, session_id)
            }
        }
    }

    fn add_and_spread_block_cmnds(&mut self, block_box: Box<VerifiedBlock>) -> Vec<PeerCommand> {
        match self.block_forest.add_block(*block_box.clone()) {
            Ok(()) => self
                .sessions_cache
                .blocks
                .par_iter_mut()
                .filter_map(|(session_id, known_blocks)| {
                    known_blocks
                        .insert(block_box.hash().clone())
                        .then_some(session_id)
                })
                .map(|&session_id| PeerCommand {
                    session_id,
                    command_kind: PeerCommandKind::SendMessage(VerifiedPeerMessage::Block(
                        block_box.clone(),
                    )),
                })
                .collect(),
            Err(e) => {
                error!("new block failed to add: {}", e);
                vec![]
            }
        }
    }

    fn new_block_cmds(
        &mut self,
        block_box: Box<VerifiedBlock>,
        from_session_id: SessionId,
    ) -> Vec<PeerCommand> {
        // update the source node cache
        let cache_updated = self
            .sessions_cache
            .blocks
            .get_mut(&from_session_id)
            .and_then(|known_blocks| Some(known_blocks.insert(block_box.hash().clone())));

        if cache_updated.is_none() {
            error!("new blocks from unknown session");
            return vec![];
        }

        self.add_and_spread_block_cmnds(block_box)
    }

    fn new_tx_cmds(
        &mut self,
        tx_box: Box<VerifiedTransaction>,
        from_session_id: SessionId,
    ) -> Vec<PeerCommand> {
        // update the source node cache
        let cache_updated = self
            .sessions_cache
            .txs
            .get_mut(&from_session_id)
            .and_then(|known_txs| Some(known_txs.insert(tx_box.hash().clone())));

        if cache_updated.is_none() {
            error!("new txs from unknown session");
            return vec![];
        }

        match self.block_forest.add_transaction(*tx_box.clone()) {
            Ok(()) => self
                .sessions_cache
                .txs
                .par_iter_mut()
                .filter_map(|(session_id, known_txs)| {
                    known_txs
                        .insert(tx_box.hash().clone())
                        .then_some(session_id)
                })
                .map(|&session_id| PeerCommand {
                    session_id,
                    command_kind: PeerCommandKind::SendMessage(VerifiedPeerMessage::Transaction(
                        tx_box.clone(),
                    )),
                })
                .collect(),
            Err(e) => {
                error!("new block failed to add: {}", e);
                vec![]
            }
        }
    }

    fn requested_block_cmd(
        &self,
        block_hash: BlockHash,
        session_id: SessionId,
    ) -> Vec<PeerCommand> {
        match self.block_forest.find_block(&block_hash) {
            Some(block) => vec![PeerCommand {
                session_id,
                command_kind: PeerCommandKind::SendMessage(VerifiedPeerMessage::Block(Box::new(
                    block.as_ref().clone(),
                ))),
            }],
            None => vec![],
        }
    }

    fn get_all_known_session_ids(&self) -> Vec<&SessionId> {
        self.sessions_cache
            .blocks
            .keys()
            .collect::<Vec<&SessionId>>()
    }

    fn request_unknown_blocks(&self) {
        let known_session_ids = self.get_all_known_session_ids();

        let cmds_iter = self
            .block_forest
            .unknown_block_hashes()
            .iter()
            .map(|block_hash| {
                known_session_ids.iter().map(|&session_id| PeerCommand {
                    session_id: session_id.clone(),
                    command_kind: PeerCommandKind::SendMessage(VerifiedPeerMessage::Request {
                        block_hash: block_hash.clone(),
                    }),
                })
            })
            .flatten();

        cmds_iter.for_each(|peer_cmd| {
            self.command_sender
                .send(peer_cmd)
                .expect("unable to send peer comand")
        })
    }

    fn spread_mined_block(&mut self, peer_block_msg: Result<VerifiedBlock, RecvError>) {
        if let Err(e) = peer_block_msg {
            error!("unable to receive peer event msg: {}", e);
            return;
        }
        self.add_and_spread_block_cmnds(Box::new(peer_block_msg.unwrap()));
    }
}
