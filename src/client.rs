use std::net::{UdpSocket, TcpStream};
use std::io::{Read, Write};
use std::time::Duration;
use std::thread::{sleep, spawn, JoinHandle};
use crate::server::{Server, Message};
use std::sync::{Arc, Mutex};
type AMV<T> = Arc<Mutex<Vec<T>>>;

fn receive(message_dump: AMV<Message>, read_stream: TcpStream) {
    loop {
        let mut buffer = [0; 512];
        match read_stream.read(&mut buffer) {
            Ok(_) => {}
        }
    } 
}

struct Client {

}

impl Client {
    pub fn new() -> Result<(Self, Option<Server>), String> {
        // Find a UDP broadcast, on failure create server    
        let udp_socket = UdpSocket::bind("0.0.0.0:34254")?;
        udp_socket.set_read_timeout(Some(Duration::new(5, 0)))?;
        let mut buffer = [0; 512];
        let mut server: Option<Server> = None;

        let server_address: String = match udp_socket.recv_from(&mut buffer) {
            Ok((size, src)) => String::from_utf8_lossy(&buffer[..size]).to_string(),
            Err(e) => {
                server = Some(Server::new(String::from("0.0.0.0:7878")));
                sleep(Duration::from_millis(100));
                format!("127.0.0.1:7878")
            }
        };

        let write_stream = TcpStream::connect(server_address)?;
        let read_stream = write_stream.try_clone().unwrap();

        Some((Client { write_stream, read_stream }, server))
    }
}
