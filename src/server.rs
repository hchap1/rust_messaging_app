use std::net::{UdpSocket, TcpListener, TcpStream};
use std::io::{Read, Write};
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::mem::replace;
type AMV<T> = Arc<Mutex<Vec<T>>>;

pub fn sync_vec<T>(item: Vec<T>) -> AMV<T> {
    Arc::new(Mutex::new(item))
}

pub fn get_localaddr() -> Option<String> {
    let mut interfaces: Vec<String> = vec![];
    let network_interfaces = NetworkInterface::show().unwrap();
    for itf in network_interfaces.into_iter() {
        let addr = itf.addr;
        let data = match addr.first() {
            Some(data) => data.ip(),
            None => continue
        };

        if data.to_string() == "127.0.0.1" {
            continue;
        }

        interfaces.push(data.to_string());
    }

    interfaces.first().cloned()
}

fn broadcast_server_address(server_address: String, frequency: f64) -> Result<String, String> {
    println!("Attempting to create UDP Socket");
    let udp_socket = match UdpSocket::bind("0.0.0.0:12345") {
        Ok(udp_socket) => udp_socket,
        Err(e) => {
            println!("UDP Creation failure: {e:?}");
            return Err(format!("Failed to broadcast server address: {e:?}"))
        }
    };

    println!("Successfully created UDP Socket");

    let _ = udp_socket.set_broadcast(true);
    let sleep_duration: Duration = Duration::from_millis(((1f64 / frequency) * 1000f64) as u64);

    loop {
        match udp_socket.send_to(server_address.as_bytes(), "255.255.255.255:12345") {
            Ok(_) => {},
            Err(_) => return Err(format!("Failed to send address over UDP socket."))
        }
        sleep(sleep_duration);
    }
}

#[derive(Clone)]
pub struct Message {
    pub author: String,
    pub content: String
}

pub struct Server {
    incoming_messages: AMV<Message>,
    outgoing_messages: AMV<Message>,
    message_agents: AMV<TcpStream>,
    listen_thread: Option<JoinHandle<()>>,
    ip_address: String
}

impl Message {
    pub fn serialise(&self) -> String {
        format!("{}|{}\n", self.author, self.content)
    }
}

pub fn deserialise(serialised: String) -> Vec<Message> {
    // Split messages apart from malformed packets
    let messages: Vec<String> = serialised.split('\n').map(|x| x.to_string()).collect::<Vec<String>>();
    let mut message_objects: Vec<Message> = vec![];
    for message in messages {
        if message.is_empty() { continue; }
        if !message.contains('|') { continue; }
        // Assume serialised protocol: NAME|CONTENT
        let components = message.split("|").map(|x| x.to_string()).collect::<Vec<String>>();
        message_objects.push(Message {
            author: match components.get(0) {
                Some(author) => author.clone(),
                None => String::new()
            },
            content: match components.get(1) {
                Some(content) => content.clone(),
                None => String::new()
            }
        });
    }
    message_objects
}

impl Server {
    pub fn new(port: usize) -> Self {
        let ip_address = match get_localaddr() {
            Some(addr) => addr,
            None => {
                panic!("Cannot start server, as no localaddr was found.");
            }
        } + &format!(":{port}");

        let mut server = Self {
            incoming_messages: sync_vec(vec![]),
            outgoing_messages: sync_vec(vec![]),
            message_agents: sync_vec(vec![]),
            listen_thread: None,
            ip_address: ip_address.clone()
        };


        println!("Server created with IP: {ip_address}");

        let client_access_incoming_messages = Arc::clone(&server.incoming_messages);
        let client_access_message_agents = Arc::clone(&server.message_agents);

        server.listen_thread = Some(spawn(move || {
            listen(client_access_incoming_messages, client_access_message_agents, ip_address);
        }));

        server
    }

    pub fn get_messages(&self) -> Option<Vec<Message>> {
        let mut messages = self.incoming_messages.lock().unwrap(); 
        if messages.len() == 0 { return None; }
        Some(replace(&mut messages, vec![]))
    }

    pub fn distribute(&mut self) {
        let mut incoming_messages = self.incoming_messages.lock().unwrap();
        let mut outgoing_messages = self.outgoing_messages.lock().unwrap();
        let mut streams = self.message_agents.lock().unwrap();
        outgoing_messages.append(&mut incoming_messages);
        let mut packet: String = String::new();
        for outgoing_message in outgoing_messages.iter() {
            packet += &outgoing_message.serialise();
        }
        outgoing_messages.clear();
        for stream in streams.iter_mut() {
            let _ = stream.write_all(packet.as_bytes());
        }
    }

    pub fn clone_addr(&self) -> String {
        self.ip_address.clone()
    }
}

fn listen(incoming_messages: AMV<Message>, message_agents: AMV<TcpStream>, address: String) {
    let tcp_listener = match TcpListener::bind(&address) {
        Ok(tcp_listener) => tcp_listener,
        Err(e) => {
            eprintln!("Failed to bind to IP: {e:?}");
            return;
        }
    };

    let server_address = tcp_listener.local_addr().unwrap().to_string();

    let mut thread_dump: Vec<JoinHandle<()>> = vec![];

    spawn(move || broadcast_server_address(server_address, 1f64));

    for stream in tcp_listener.incoming() {
        match stream {
            Ok(stream) => {
                let outgoing_stream: TcpStream = stream.try_clone().unwrap();
                {
                    let mut message_agents = message_agents.lock().unwrap();
                    message_agents.push(outgoing_stream);
                }
                println!("Client connected: {}", stream.peer_addr().unwrap());
                let client_specific_message_dump: AMV<Message> = Arc::clone(&incoming_messages);
                thread_dump.push(spawn(move || read_incoming_messages_from_client(stream, client_specific_message_dump)));
            }
            Err(_) => return
        }
    }
}

pub fn read_incoming_messages_from_client(mut stream: TcpStream, message_dump: AMV<Message>) {
    loop {
        let mut buffer = [0; 512];
        let _ = stream.read(&mut buffer);
        let message: String = String::from_utf8_lossy(&buffer).to_string();
        if message.is_empty() {
            eprintln!("Client disconnected unexpectedly.");
            return;
        }

        if message.chars().nth(0).unwrap() == '\0' {
            eprintln!("Client disconnected unexpectedly.");
            return;
        }

        let mut message_dump = message_dump.lock().unwrap();
        message_dump.append(&mut deserialise(message));
    }
}
