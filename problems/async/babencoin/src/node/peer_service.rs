#![forbid(unsafe_code)]

use crate::data::{PeerMessage, VerifiedPeerMessage};

use anyhow::{bail, Result};
use crossbeam::channel::{unbounded, Receiver, Sender};
use log::*;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, RwLock,
    },
    thread,
    time::Duration,
};

////////////////////////////////////////////////////////////////////////////////

const BUF_SIZE: usize = 65536;
const MAX_RETRIES: usize = 5;
const MSG_DELIM: u8 = 0u8;

pub type SessionId = u64;

////////////////////////////////////////////////////////////////////////////////

#[derive(Default, Serialize, Deserialize)]
pub struct PeerServiceConfig {
    #[serde(with = "humantime_serde")]
    pub dial_cooldown: Duration,
    pub dial_addresses: Vec<String>,
    pub listen_address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PeerEvent {
    pub session_id: SessionId,
    pub event_kind: PeerEventKind,
}

#[derive(Debug, Clone)]
pub enum PeerEventKind {
    Connected,
    Disconnected,
    NewMessage(VerifiedPeerMessage),
}

#[derive(Debug, Clone)]
pub struct PeerCommand {
    pub session_id: SessionId,
    pub command_kind: PeerCommandKind,
}

#[derive(Debug, Clone)]
pub enum PeerCommandKind {
    SendMessage(VerifiedPeerMessage),
    Drop,
}

////////////////////////////////////////////////////////////////////////////////

pub struct PeerService {
    config: PeerServiceConfig,
    peer_event_sender: Sender<PeerEvent>,
    command_receiver: Receiver<PeerCommand>,
    peers: Arc<RwLock<HashMap<SessionId, Sender<PeerCommandKind>>>>,
}

impl PeerService {
    pub fn new(
        config: PeerServiceConfig,
        peer_event_sender: Sender<PeerEvent>,
        command_receiver: Receiver<PeerCommand>,
    ) -> Result<Self> {
        Ok(Self {
            config,
            peer_event_sender,
            command_receiver,
            peers: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn run(&mut self) {
        self.init_dial_conns();
        self.init_command_listener();
        self.handle_new_conns();
    }

    // fn init_connect_retry(&mut self) {
    //     thread::spawn(|| {})
    // }

    fn init_dial_conns(&mut self) {
        for node in self.config.dial_addresses.iter() {
            for _ in 0..MAX_RETRIES {
                let conn = TcpStream::connect(node);
                match conn {
                    Ok(stream) => {
                        self.add_new_stream(stream);
                    }
                    Err(_) => thread::sleep(self.config.dial_cooldown),
                }
            }
        }
    }

    fn handle_new_conns(&mut self) {
        let listener = self.get_listener();

        for stream in listener.incoming() {
            let stream = stream.expect("failed to establish a new connection");
            self.add_new_stream(stream);
        }
    }
    fn add_new_stream(&self, stream: TcpStream) {
        let session_id = self.gen_unique_session_id();

        info!(
            "new connection: {} with session_id: {}",
            stream.peer_addr().unwrap().to_string(),
            session_id
        );

        let stream_arc = Arc::new(stream);

        let (comm_kind_snd, comm_kind_recv) = unbounded();

        self.peers
            .write()
            .unwrap()
            .insert(session_id, comm_kind_snd);

        self.init_tcp_read(stream_arc.clone(), session_id);
        self.init_tcp_write(stream_arc.clone(), comm_kind_recv);

        self.peer_event_sender
            .send(PeerEvent {
                session_id,
                event_kind: PeerEventKind::Connected,
            })
            .expect(
                format!(
                    "couldn't send connected event for {}",
                    stream_arc.peer_addr().unwrap()
                )
                .as_str(),
            );
    }

    fn init_tcp_read(&self, stream_arc: Arc<TcpStream>, session_id: SessionId) {
        let event_sender = self.peer_event_sender.clone();
        thread::spawn(move || {
            let mut r_socket = BufReader::with_capacity(BUF_SIZE, stream_arc.as_ref());
            let mut message = Vec::with_capacity(BUF_SIZE);
            'read_loop: loop {
                let res_buf = r_socket.fill_buf();
                if let Err(e) = res_buf {
                    error!("error while filling a buf: {}", e);
                    break;
                }
                let buf = res_buf.unwrap();
                if buf.is_empty() {
                    continue;
                }

                for &byte in buf {
                    if byte == MSG_DELIM {
                        debug!(
                            "message from session_id: {:?}, peer_addr: {:?}",
                            session_id,
                            stream_arc.peer_addr().unwrap(),
                        );
                        // Process the complete message
                        if Self::process_the_message(
                            message.clone(),
                            &event_sender,
                            session_id,
                            stream_arc.peer_addr().unwrap(),
                        )
                        .is_err()
                        {
                            break 'read_loop;
                        }

                        // Clear the message buffer for the next message
                        message.clear();
                    } else if message.len() >= BUF_SIZE {
                        break 'read_loop;
                    } else {
                        message.push(byte);
                    }
                }
                if message.len() >= BUF_SIZE {
                    error!(
                        "the incoming message from {} was too large",
                        stream_arc.peer_addr().unwrap()
                    );
                    break;
                }

                let length = buf.len();
                r_socket.consume(length);
            }
            debug!("sent peer event Disconnected for session_id {}", session_id);

            event_sender
                .send(PeerEvent {
                    session_id,
                    event_kind: PeerEventKind::Disconnected,
                })
                .expect(
                    format!(
                        "couldn't send Disconnected event for session_id {}",
                        session_id
                    )
                    .as_str(),
                );
        });
    }

    // rewrite not to panic and return result
    fn process_the_message(
        message: Vec<u8>,
        event_sender: &Sender<PeerEvent>,
        session_id: SessionId,
        peer_addr: SocketAddr,
    ) -> Result<()> {
        let str_json =
            String::from_utf8(message).expect("failed to convert bytes array to valid utf_8");
        debug!("new json from {:?} has come: {}", peer_addr, str_json);

        if let Ok(pmsg) = serde_json::from_str::<PeerMessage>(&str_json) {
            if let Ok(verified_msg) = pmsg.verified() {
                event_sender
                    .send(PeerEvent {
                        session_id,
                        event_kind: PeerEventKind::NewMessage(verified_msg),
                    })
                    .expect(
                        format!(
                            "couldn't send NewMessage event for {} into the channel",
                            peer_addr
                        )
                        .as_str(),
                    );
                return Ok(());
            }
        } else {
            error!("couldn't deserialize the msg");
        }

        bail!("error parsing a message")
    }

    fn init_tcp_write(
        &self,
        stream_arc: Arc<TcpStream>,
        comm_kind_receiver: Receiver<PeerCommandKind>,
    ) {
        thread::spawn(move || loop {
            let command_kind_res = comm_kind_receiver.recv();
            if let Err(e) = command_kind_res {
                error!("error while receiving a command kind: {}", e);
                break;
            }
            let command_kind = command_kind_res.unwrap();
            let mut stream_ref = stream_arc.as_ref();

            match command_kind {
                PeerCommandKind::SendMessage(verified_msg) => {
                    debug!(
                        "new message for: {:?}  content: {:?} ",
                        stream_ref.peer_addr(),
                        verified_msg,
                    );
                    let peer_msg: PeerMessage = verified_msg.into();
                    let serde_write_res = Self::do_with_retry(|| {
                        serde_json::to_writer::<_, PeerMessage>(&mut stream_ref, &peer_msg)
                    });
                    if let Err(e) = serde_write_res {
                        error!("error while writing to stream with serde: {}", e);
                        continue;
                    }
                    let write_res = Self::do_with_retry(|| stream_ref.write_all(b"\0"))
                        .and_then(|_| Self::do_with_retry(|| stream_ref.flush()));

                    if let Err(e) = write_res {
                        error!("error while writing to stream: {}", e);
                    }
                }
                PeerCommandKind::Drop => {
                    debug!("connection dropped",);
                    if let Err(e) = stream_ref.shutdown(Shutdown::Both) {
                        error!("error while dropping: {e}");
                    }
                    break;
                }
            };
        });
    }

    fn init_command_listener(&self) {
        let command_receiver = self.command_receiver.clone();
        let peers = self.peers.clone();
        thread::spawn(move || loop {
            let PeerCommand {
                session_id,
                command_kind,
            } = command_receiver.recv().expect("error receiving command");

            debug!(
                "for session {} received new command {:?}",
                session_id, command_kind
            );
            let peers_rlock = peers.read().expect("failed to take read lock on peers map");

            let sender = peers_rlock.get(&session_id).unwrap();
            // debug!("sender: {:?}, with session_id: {}", sender, session_id);

            if let Err(e) = sender.send(command_kind) {
                error!("error while trying to send a command_kind: {e}");
            } else {
                continue;
            }
            // removing corrupted sender
            peers
                .write()
                .expect("failed to take read lock on peers map")
                .remove(&session_id);

            // let send_err = peers
            //     .read()
            //     .expect("failed to take read lock on peers map")
            //     .get(&session_id)
            //     .unwrap()
            //     .send(command_kind);
            // if let Err(e) = send_err {
            //     error!("error while trying to send a command_kind: {e}");
            // }
        });
    }

    fn gen_unique_session_id(&self) -> SessionId {
        let mut rng = rand::thread_rng();
        let mut session_id = rng.gen::<SessionId>();
        while self.peers.read().unwrap().contains_key(&session_id) {
            session_id = rng.gen()
        }
        session_id
    }

    fn get_listener(&self) -> TcpListener {
        for _ in 0..MAX_RETRIES {
            let listen_addr = self.config.listen_address.clone().unwrap_or_else(|| {
                let port = thread_rng().gen_range(49152..65536);
                format!("127.0.0.1:{}", port)
            });

            let listener = TcpListener::bind(listen_addr);
            match listener {
                Ok(listener) => {
                    info!(
                        "successfully created listener on: {} ",
                        listener.local_addr().unwrap().to_string()
                    );
                    return listener;
                }
                Err(e) => error!("error while trying to open a listener conn: {}", e),
            };
            thread::sleep(self.config.dial_cooldown)
        }
        panic!("can't establish a peer service listener connection");
    }

    fn do_with_retry<F, E>(mut f: F) -> Result<(), E>
    where
        F: FnMut() -> Result<(), E>,
    {
        let mut last_res = Ok(());
        for _ in 0..MAX_RETRIES {
            last_res = f();
            if last_res.is_ok() {
                return last_res;
            }
        }
        last_res
    }
}
