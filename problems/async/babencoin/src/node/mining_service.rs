#![forbid(unsafe_code)]

use std::{
    collections::HashSet,
};

use crate::{
    data::{
        Block, BlockAttributes, BlockHash, Transaction, TransactionHash, VerifiedBlock,
        VerifiedTransaction, WalletId, MAX_REWARD,
    },
    util::{deserialize_wallet_id, serialize_wallet_id},
};


use chrono::Utc;
use crossbeam::channel::{Receiver, Sender};

use log::*;
use rand::{thread_rng, Rng};
use rayon::{ThreadPoolBuilder};
use serde::{Deserialize, Serialize};

////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct MiningServiceConfig {
    pub thread_count: usize,
    pub max_tx_per_block: usize,

    #[serde(
        serialize_with = "serialize_wallet_id",
        deserialize_with = "deserialize_wallet_id"
    )]
    pub public_key: WalletId,
}

impl Default for MiningServiceConfig {
    fn default() -> Self {
        Self {
            thread_count: 0,
            max_tx_per_block: 0,
            public_key: WalletId::of_genesis(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct MiningInfo {
    pub block_index: u64,
    pub prev_hash: BlockHash,
    pub max_hash: BlockHash,
    pub transactions: Vec<VerifiedTransaction>,
}

// type TransactionsVecHash = BlockHash;
pub struct MiningService {
    config: MiningServiceConfig,
    info_receiver: Receiver<MiningInfo>,
    block_sender: Sender<VerifiedBlock>,
    // a assume that a node without any new incoming txs would
    // have same sequence of pending txs
    computed_txs: HashSet<TransactionHash>,
}

impl MiningService {
    pub fn new(
        config: MiningServiceConfig,
        info_receiver: Receiver<MiningInfo>,
        block_sender: Sender<VerifiedBlock>,
    ) -> Self {
        Self {
            config,
            info_receiver,
            block_sender,
            computed_txs: HashSet::new(),
        }
    }

    pub fn run(&mut self) {
        let pool = ThreadPoolBuilder::new()
            .num_threads(self.config.thread_count)
            .build()
            .unwrap();

        debug!("starting mining with config: {:?}", self.config);

        loop {
            let mining_info = self.info_receiver.recv();
            if let Err(e) = mining_info {
                error!("unable to receive mining infor msg: {}", e);
                continue;
            }

            let MiningInfo {
                block_index,
                prev_hash,
                max_hash,
                transactions,
            } = mining_info.unwrap();

            let selected_txs = transactions
                .into_iter()
                .take(self.config.max_tx_per_block)
                .map(Into::into)
                .collect::<Vec<Transaction>>();

            if selected_txs.is_empty() || !self.all_txs_are_new(&selected_txs) {
                continue;
            }

            for tx in selected_txs.iter() {
                debug!("mining tx with comments: {}", tx.comment);
            }

            let mut rng = thread_rng();
            let reward: u64 = rng.gen_range(0..=MAX_REWARD);

            let issuer = self.config.public_key.clone();

            let new_block = loop {
                let guessed_blocks: Option<VerifiedBlock> = pool
                    .broadcast(|_| {
                        let mut rng = thread_rng();
                        let attrs = BlockAttributes {
                            index: block_index,
                            reward,
                            nonce: rng.gen::<u64>(),
                            timestamp: Utc::now(),
                            issuer: issuer.clone(),
                            max_hash,
                            prev_hash,
                        };
                        Block {
                            attrs,
                            transactions: selected_txs.clone(),
                        }
                    })
                    .into_iter()
                    .filter_map(|block| block.clone().verified().ok())
                    .next();
                if let Some(block) = guessed_blocks {
                    break block;
                }
            };

            for tx in selected_txs.iter() {
                self.computed_txs.insert(tx.compute_hash());
            }
            let send_res = self.block_sender.send(new_block);
            if let Err(e) = send_res {
                error!("error while trying to send newly generated block: {}", e);
            }
        }
    }
    // Txs do not contain timestamps, so it's useless
    // fn get_selected_txs_hash(txs: &Vec<Transaction>) -> TransactionsVecHash {
    //     let mut genesis = Block::genesis();
    //     genesis.transactions = txs.clone();
    //     genesis.compute_hash()
    // }

    fn all_txs_are_new(&mut self, txs: &[Transaction]) -> bool {
        for tx in txs.iter() {
            if self.computed_txs.contains(&tx.compute_hash()) {
                return false;
            }
        }
        true
        // let txs_hash = Self::get_selected_txs_hash(txs);
        // self.computed_txs.insert(txs_hash)
    }
}
