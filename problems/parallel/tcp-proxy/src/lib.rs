#![forbid(unsafe_code)]

use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use log::info;
use std::io::copy;

pub fn run_proxy(port: u32, destination: String) {
    let listener =
        TcpListener::bind(format!("127.0.0.1:{}", port)).expect("Unable to bind proxy addr");
    for stream in listener.incoming() {
        let from_stream = stream.unwrap();
        let to_stream = TcpStream::connect(destination.as_str()).unwrap();
        copy_duplex_stream(from_stream, to_stream);
    }
    info!("Proxying from localhost:{} to {}", port, destination);
}

fn copy_duplex_stream(from_stream: TcpStream, to_stream: TcpStream) {
    let arc_from = Arc::new(from_stream);
    let arc_to = Arc::new(to_stream);

    copy_arc_stream(arc_from.clone(), arc_to.clone());
    copy_arc_stream(arc_to.clone(), arc_from.clone());
}

fn copy_arc_stream(
    from_stream: Arc<TcpStream>,
    to_stream: Arc<TcpStream>,
) -> JoinHandle<Result<u64, std::io::Error>> {
    thread::spawn(move || copy(&mut from_stream.as_ref(), &mut to_stream.as_ref()))
}
