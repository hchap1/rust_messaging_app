use std::net::{UdpSocket, TcpListener, TcpStream};
use std::io::{Read, Write};
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;
use std::sync::Arc;
use crate::utility::{sync, sync_vec, AM, AMV};

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

fn broadcast_server_address(running: AM<bool>, server_address: String, frequency: f64) -> Result<String, String> {
    println!("Attempting to create UDP Socket on address : {server_address}");
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
        let running = running.lock().unwrap();
        if !*running {
            return Ok(Default::default());
        }
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
    ip_address: String,
    running: AM<bool>,
    tcp_listener: AM<Option<TcpListener>>
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
            ip_address: ip_address.clone(),
            running: sync(true),
            tcp_listener: sync(None)
        };


        println!("Server created with IP: {ip_address}");

        let client_access_incoming_messages = Arc::clone(&server.incoming_messages);
        let client_access_message_agents = Arc::clone(&server.message_agents);
        let listen_running = Arc::clone(&server.running);
        let client_access_tcp_dump = Arc::clone(&server.tcp_listener);

        server.listen_thread = Some(spawn(move || {
            listen(client_access_incoming_messages, client_access_message_agents, ip_address, listen_running, client_access_tcp_dump);
        }));

        server
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
    
    pub fn kill(&mut self) {
        let message_agents = self.message_agents.lock().unwrap();
        let mut running = self.running.lock().unwrap();
        *running = false;
        for ma in message_agents.iter() {
            let _ = ma.set_nonblocking(true);
            let _ = ma.shutdown(std::net::Shutdown::Both);
        }
        let tcp_listener = self.tcp_listener.lock().unwrap();
        match &*tcp_listener {
            Some(tcp) => {
                let _ = tcp.set_nonblocking(true);
                match self.listen_thread.take() {
                    Some(handle) => { let _ = handle.join(); }
                    None => {}
                }
            }
            None => {}
        }
    }

    pub fn clone_addr(&self) -> String {
        self.ip_address.clone()
    }
}

fn listen(incoming_messages: AMV<Message>, message_agents: AMV<TcpStream>, address: String, running: AM<bool>, tcp_listener_return: AM<Option<TcpListener>>) {
    let tcp_listener = match TcpListener::bind(&address) {
        Ok(tcp_listener) => tcp_listener,
        Err(e) => {
            eprintln!("Failed to bind to IP: {e:?}");
            return;
        }
    };

    {
        let mut tcp_listener_return = tcp_listener_return.lock().unwrap();
        *tcp_listener_return = Some(tcp_listener.try_clone().unwrap());
    }

    let server_address = tcp_listener.local_addr().unwrap().to_string();
    let mut thread_dump: Vec<JoinHandle<()>> = vec![];
    spawn(move || broadcast_server_address(running, server_address, 1f64));

    for stream in tcp_listener.incoming() {
        match stream {
            Ok(stream) => {
                let outgoing_stream: TcpStream = stream.try_clone().unwrap();
                let address: String;
                {
                    address = match stream.peer_addr() {
                        Ok(address) => address.ip().to_string(),
                        Err(_) => String::from("<UNKNOWN CLIENT>")
                    };
                    let mut message_agents = message_agents.lock().unwrap();
                    message_agents.push(outgoing_stream);
                    let packet: String = format!("SERVER|{address} connected.");
                    for ma in message_agents.iter_mut() {
                        let _ = ma.write_all(packet.as_bytes());
                    }
                }
                let client_specific_message_dump: AMV<Message> = Arc::clone(&incoming_messages);
                thread_dump.push(spawn(move || read_incoming_messages_from_client(stream, client_specific_message_dump, address)));
            }
            // Shutdown occurs by setting set_nonblocking to true, causing this to error when there is no client joining.
            Err(_) => return
        }
    }
}

pub fn read_incoming_messages_from_client(mut stream: TcpStream, message_dump: AMV<Message>, client: String) {
    loop {
        let mut buffer = [0; 512];
        let _ = stream.read(&mut buffer);
        let message: String = String::from_utf8_lossy(&buffer).to_string();
        if message.is_empty() {
            let mut message_dump = message_dump.lock().unwrap();
            message_dump.push(Message { author: String::from("SERVER"), content: format!("{client} disconnected.") });
            return;
        }

        if message.chars().nth(0).unwrap() == '\0' {
            let mut message_dump = message_dump.lock().unwrap();
            message_dump.push(Message { author: String::from("SERVER"), content: format!("{client} disconnected.") });
            return;
        }

        let mut message_dump = message_dump.lock().unwrap();
        message_dump.append(&mut deserialise(message));
    }
}
