mod server;

use server::Server;

fn main() {
    let server: Server = Server::new("0.0.0.0:7878")
}
