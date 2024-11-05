mod server;

use server::Server;
use std::time::Duration;
use std::thread::sleep;

fn main() {
    let server: Server = Server::new("0.0.0.0:7878".to_string());
    loop {
        sleep(Duration::from_secs(1));
        println!("{:?}", server.get_messages())
    }
}
