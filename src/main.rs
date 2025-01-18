mod server;
mod client;
mod application;

use server::get_localaddr;
use std::thread::{spawn, sleep};
use client::Client;
use application::{Application, execute_application};
use std::time::Duration;

fn main() {
    let (client, server) = match Client::new() {
        Ok((client, server)) => (client, server),
        Err(e) => panic!("Error: {e:?}")
    };

    let hostname: String = match get_localaddr() {
        Some(hostname) => hostname,
        None => panic!("No localaddr could be found.")
    };
    
    println!("Using {hostname} IPV4 interface.");

    let terminal = ratatui::init();
    let application: Application = Application::new();
    let application_handle = spawn(move || {
        let _ = execute_application(application, terminal, client, hostname);
    });

    match server {
        Some(mut server) => {
            while !application_handle.is_finished() {
                server.distribute();
                sleep(Duration::from_secs(1));
            }
        }
        None => { let _ = application_handle.join(); }
    }
}
