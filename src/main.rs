mod server;
mod client;

use server::{Message, get_localaddr};
use std::time::Duration;
use std::thread::sleep;
use client::Client;

fn main() {
    let (mut client, server) = match Client::new() {
        Ok((client, server)) => (client, server),
        Err(e) => panic!("Error: {e:?}")
    };

    let hostname: String = match get_localaddr() {
        Some(hostname) => hostname,
        None => {
            eprintln!("No localaddr could be determined.");
            return;
        }
    };

    if let Some(_) = server {
        println!("Server was started.");
    }

    let test_message: Message = format!("{hostname}|Test").into();

    loop {
        let _ = client.send(&test_message);
        sleep(Duration::from_secs(1));
    }
}
