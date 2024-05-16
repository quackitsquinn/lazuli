use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use rsock::{Client, Server};

const ADDRESS: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080));

pub fn main() {
    // Initialize the server and client. The server will accept a connection from the client.
    let mut server = Server::new(ADDRESS).unwrap();
    let mut client = Client::new(ADDRESS).unwrap();

    // Accept the connection from the client. This returns a Client object that can be used to communicate with the client.
    // The client object is wrapped in an Arc<Mutex<Client>> to allow for thread-safe access.
    let server_client = server.accept().unwrap();

    // Streams are how data is received.
    // You create a stream for the type you want to receive, and then call recv() on the client or server to get the data.
    let mut stream = server_client.lock().unwrap().stream::<String>();
    // Send a message from the client to the server.
    // client.send can send any type that implements Sendable. This includes all primitive types.
    // There is a derive macro for Sendable that can be used to implement it for custom types.
    client.send(&"Hello, world!".to_owned()).unwrap();
    // Receive the message from the client. This will block unless it is made with nonblocking set to true.
    server_client.lock().unwrap().recv().unwrap();
    // Get the message from the stream.
    assert_eq!(stream.get(), Some("Hello, world!".to_owned()));
}
