#![forbid(unsafe_code)]

use crate::data::{PeerMessage, VerifiedPeerMessage};

use anyhow::{bail, Context, Result};
use crossbeam::channel::{self, unbounded, Receiver, Sender};
use log::*;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use std::{
    collections::HashMap,
    fmt::{self, Display},
    io::{self, BufRead, BufReader, ErrorKind, Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    thread,
    time::Duration,
};

////////////////////////////////////////////////////////////////////////////////

const BUF_SIZE: usize = 65536;
const MAX_RETRIES: usize = 5;

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
        self.init_tcp_read(stream_arc.clone(), session_id);

        let (comm_kind_snd, comm_kind_recv) = unbounded();
        self.init_tcp_write(stream_arc.clone(), comm_kind_recv);

        self.peers
            .write()
            .unwrap()
            .insert(session_id, comm_kind_snd);

        self.peer_event_sender
            .send(PeerEvent {
                session_id,
                event_kind: PeerEventKind::Connected,
            })
            .expect(
                format!(
                    "couldn't send connected event for {}",
                    stream_arc.local_addr().unwrap()
                )
                .as_str(),
            );
    }

    fn init_tcp_read(&self, stream_arc: Arc<TcpStream>, session_id: SessionId) {
        let event_sender = self.peer_event_sender.clone();
        thread::spawn(move || {
            let mut r_socket = BufReader::with_capacity(BUF_SIZE, stream_arc.as_ref());
            let mut message = Vec::with_capacity(BUF_SIZE);
            loop {
                message.clear();

                let mut filled_buf = r_socket.fill_buf().expect("error while reading a socket");
                let read_bytes = filled_buf.read_until(0u8, &mut message).unwrap();
                r_socket.consume(read_bytes);

                if read_bytes == 0 {
                    continue;
                }

                if message.last().unwrap() == &0u8 {
                    let message = message.split_last().unwrap().1;
                    let str_json = String::from_utf8(message.into())
                        .expect("failed to convert bytes array to valid utf_8");
                    debug!("New json has come: {}", str_json);

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
                                        stream_arc.local_addr().unwrap()
                                    )
                                    .as_str(),
                                );
                            continue;
                        }
                    } else {
                        error!("couldn't deserialize the msg");
                    }
                } else {
                    error!(
                        "the incoming message from {} was too large",
                        stream_arc.local_addr().unwrap()
                    );
                }

                event_sender
                    .send(PeerEvent {
                        session_id,
                        event_kind: PeerEventKind::Disconnected,
                    })
                    .expect(
                        format!(
                            "couldn't send Disconnected event for {}",
                            stream_arc.local_addr().unwrap()
                        )
                        .as_str(),
                    )
            }
        });
    }

    fn init_tcp_write(
        &self,
        stream_arc: Arc<TcpStream>,
        comm_kind_receiver: Receiver<PeerCommandKind>,
    ) {
        thread::spawn(move || loop {
            let command_kind = comm_kind_receiver
                .recv()
                .expect("couldn't receive a command kind");
            let mut stream_ref = stream_arc.as_ref();
            match command_kind {
                PeerCommandKind::SendMessage(verified_msg) => {
                    debug!("new message to send: {:?}", verified_msg);

                    serde_json::to_writer::<_, PeerMessage>(&mut stream_ref, &verified_msg.into())
                        .expect("failed to serialize object ot json");
                    stream_ref
                        .write_all(b"\0")
                        .expect("failed to write trailing zero");
                }
                PeerCommandKind::Drop => {
                    debug!(
                        "connection with {:?} is dropped",
                        stream_ref.local_addr().unwrap()
                    );
                    stream_ref
                        .shutdown(Shutdown::Both)
                        .expect("shutdown call failed");
                    break;
                }
            }
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

            let send_err = peers
                .read()
                .expect("failed to take read lock on peers map")
                .get(&session_id)
                .unwrap()
                .send(command_kind);
            if let Err(e) = send_err {
                error!("error while trying to send a command_kind: {e}");
            }
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
}
