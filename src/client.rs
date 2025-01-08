use std::net::{UdpSocket, TcpStream};
use std::io::{Read, Write};
use std::time::Duration;
use std::thread::{sleep, spawn, JoinHandle};
use crate::server::{Server, Message, sync_vec};
use std::sync::{Arc, Mutex};
type AMV<T> = Arc<Mutex<Vec<T>>>;

fn receive(message_dump: AMV<Message>, mut read_stream: TcpStream) {
    loop {
        let mut buffer = [0; 512];
        match read_stream.read(&mut buffer) {
            Ok(size) => {
                let message = String::from_utf8_lossy(&buffer[..size]).to_string();
                if !message.is_empty() {
                    let mut message_dump = message_dump.lock().unwrap();
                    message_dump.push(message.into());
                }
            }
            Err(_) => {
                eprintln!("Server closed. Connection terminated.");
                return;
            }
        }
    } 
}

pub struct Client {
    incoming_messages: AMV<Message>,
    listen_thread: Option<JoinHandle<()>>,
    write_stream: TcpStream
}

impl Client {
    pub fn new() -> Result<(Self, Option<Server>), String> {
        // Find a UDP broadcast, on failure create server    
        let udp_socket = match UdpSocket::bind("0.0.0.0:12345") {
            Ok(udp_socket) => udp_socket,
            Err(e) => return Err(format!("Failed to bind UdpSocket: {e:?}"))
        };


        println!("Succesfully created UDP socket.");

        let _ = udp_socket.set_read_timeout(Some(Duration::new(5, 0)));
        let mut buffer = [0; 512];
        let mut server: Option<Server> = None;

        let server_address: String = match udp_socket.recv_from(&mut buffer) {
            Ok((size, _)) => String::from_utf8_lossy(&buffer[..size]).to_string(),
            Err(_) => {
                std::mem::drop(udp_socket);
                server = Some(Server::new(String::from("0.0.0.0:7878")));
                sleep(Duration::from_millis(100));
                format!("127.0.0.1:7878")
            }
        };

        println!("Either received or timed out. {server_address}");

        let write_stream = match TcpStream::connect(server_address) {
            Ok(write_stream) => write_stream,
            Err(e) => return Err(format!("Failed to bind TcpSocket: {e:?}"))
        };

        let read_stream = write_stream.try_clone().unwrap();
        let mut client = Client { incoming_messages: sync_vec(vec![]), listen_thread: None, write_stream };
        let listen_thread_owned_dump: AMV<Message> = Arc::clone(&client.incoming_messages);
        client.listen_thread = Some(spawn(move || receive(listen_thread_owned_dump, read_stream)));

        Ok((client, server))
    }

    pub fn send(&mut self, message: &Message) -> Result<usize, String> {
        match self.write_stream.write_all(message.serialise().as_bytes()) {
            Ok(_) => Ok(1),
            Err(e) => Err(format!("Error: {e:?}"))
        }
    }
}
