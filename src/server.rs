use std::net::{UdpSocket, TcpListener, TcpStream};
use std::io::{Read, Write};
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;
use std::sync::{Arc, Mutex};
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
            Ok(_) => println!("BROADCASTED : {server_address}"),
            Err(_) => return Err(format!("Failed to send address over UDP socket."))
        }
        sleep(sleep_duration);
    }
}

pub struct Message {
    author: String,
    content: String
}

pub struct Server {
    incoming_messages: AMV<Message>,
    outgoing_messages: AMV<Message>,
    message_agents: AMV<TcpStream>,
    listen_thread: Option<JoinHandle<()>>
}

impl Message {
    pub fn serialise(&self) -> String {
        format!("{}|{}", self.author, self.content)
    }
}

impl From<String> for Message {
    fn from(serialised: String) -> Self {
        // Assume serialised protocol: NAME|CONTENT
        let components = serialised.split("|").map(|x| x.to_string()).collect::<Vec<String>>();
        Self {
            author: match components.get(0) {
                Some(author) => author.clone(),
                None => String::new()
            },
            content: match components.get(1) {
                Some(content) => content.clone(),
                None => String::new()
            }
        }
    }
}

impl Server {
    pub fn new(port: usize) -> Self {
        let mut server = Self {
            incoming_messages: sync_vec(vec![]),
            outgoing_messages: sync_vec(vec![]),
            message_agents: sync_vec(vec![]),
            listen_thread: None
        };

        let ip_address = match get_localaddr() {
            Some(addr) => addr,
            None => {
                panic!("Cannot start server, as no localaddr was found.");
            }
        };

        let client_access_incoming_messages = Arc::clone(&server.incoming_messages);
        let client_access_message_agents = Arc::clone(&server.message_agents);

        server.listen_thread = Some(spawn(move || {
            listen(client_access_incoming_messages, client_access_message_agents, ip_address);
        }));

        server
    }

    pub fn get_messages(&self) -> Vec<String> {
        let messages = self.incoming_messages.lock().unwrap(); 
        messages.iter().map(|x| format!("{}: {}", x.author, x.content)).collect::<Vec<String>>()
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
        println!("Received: {}", message);
        {
            let mut message_dump = message_dump.lock().unwrap();
            message_dump.push(message.into())
        }
    }
}
