mod server;
mod client;

use server::Message;
use std::time::Duration;
use std::thread::sleep;
use client::Client;

fn main() {
    let (mut client, server) = match Client::new() {
        Ok((client, server)) => (client, server),
        Err(e) => panic!("Error: {e:?}")
    };

    if let Some(server) = server {
        println!("Server was started.");
        loop {
            sleep(Duration::from_secs(1));
            println!("Current server message list: {:?}", server.get_messages());
        }
    }

    let test_message: Message = String::from("TestAuthor|this is a test message").into();

    loop {
        let _ = client.send(&test_message);
        sleep(Duration::from_secs(1));
    }
}
