mod client;
mod connector;
mod input;
mod listener;
mod server;

pub(self) type StreamCollection = std::collections::HashMap<u32, connector::StreamConnector>;

pub use client::Client;
pub use server::Server;

#[cfg(test)]
/// Test utilities for the client module.
mod test_utils {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        sync::Mutex,
    };

    use log::debug;

    use crate::Sendable;

    use self::server::Server;

    use super::*;

    /// Creates a client and server pair.
    /// (client, server)
    pub(super) fn make_client_server_pair() -> (Client, Client) {
        use std::net::TcpListener;
        let server = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("Failed to create server!");

        let client = Client::connect(server.local_addr().unwrap()).unwrap();
        let server = server.accept().unwrap().0;

        (client, Client::from_stream(server))
    }
    /// Creates a Server. Expects the OS to assign a port.
    pub(super) fn make_server() -> Server {
        Server::new((Ipv4Addr::LOCALHOST, 0)).expect("Failed to create server!")
    }

    /// Tests sending and receiving data. Convenience function for testing.
    pub(super) fn test_send_recv<T>(client: &mut Client, server: &mut Client, data: T)
    where
        T: Sendable + 'static + PartialEq,
    {
        let mut stream = client.stream::<T>();
        server.send(&data).unwrap();
        client.recv().unwrap();
        assert_eq!(stream.get().unwrap(), data);
    }
}
