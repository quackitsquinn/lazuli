mod client;
mod connector;
mod input;
mod listener;
mod server;

pub(self) type StreamCollection = std::collections::HashMap<u32, connector::StreamConnector>;

pub use client::TcpClient;

#[cfg(test)]
/// Test utilities for the client module.
mod test_utils {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        sync::{Mutex, OnceLock},
    };

    use log::info;

    use crate::Sendable;

    use self::server::Server;

    use super::*;
    static PORTS_BASE: u16 = 5000;
    static PORT_ACTIVE_BASE: Mutex<u16> = Mutex::new(PORTS_BASE);

    /// Creates a client and server pair.
    /// (client, server)
    pub(super) fn make_client_server_pair() -> (TcpClient, TcpClient) {
        use std::net::TcpListener;
        let server = TcpListener::bind((Ipv4Addr::LOCALHOST, *PORT_ACTIVE_BASE.lock().unwrap()));

        *PORT_ACTIVE_BASE.lock().unwrap() += 1;

        if let Err(e) = server {
            // If the port is in use, try again.
            if e.kind() == std::io::ErrorKind::AddrInUse {
                return make_client_server_pair();
            } else {
                panic!("Failed to bind server: {}", e);
            }
        }

        let server = server.unwrap();

        let client = TcpClient::new(server.local_addr().unwrap()).unwrap();
        let server = server.accept().unwrap().0;

        (client, TcpClient::from_stream(server))
    }

    pub(super) fn make_server() -> Server {
        let server = Server::new((Ipv4Addr::LOCALHOST, *PORT_ACTIVE_BASE.lock().unwrap()));

        if let Err(e) = server {
            // If the port is in use, try again.
            if e.kind() == std::io::ErrorKind::AddrInUse {
                return make_server();
            } else {
                panic!("Failed to bind server: {}", e);
            }
        }

        *PORT_ACTIVE_BASE.lock().unwrap() += 1;

        server.unwrap()
    }

    /// Tests sending and receiving data. Convenience function for testing.
    pub(super) fn test_send_recv<T>(client: &mut TcpClient, server: &mut TcpClient, data: T)
    where
        T: Sendable + 'static + PartialEq,
    {
        let mut stream = client.stream::<T>();
        server.send(&data).unwrap();
        client.recv().unwrap();
        assert_eq!(stream.get().unwrap(), data);
    }
}
