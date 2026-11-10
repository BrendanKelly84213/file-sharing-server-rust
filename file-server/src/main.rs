mod filesystem;
mod server;

use server::FileServer;

fn main() {
    print!("Hello and welcome!");

    let server = FileServer::new(12345, "filesystem.dat", 10 * 128);
    // Start the file server
    server.start();
}
