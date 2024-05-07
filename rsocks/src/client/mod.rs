mod client;
mod connector;
mod input;
mod listener;

pub(self) type StreamCollection = std::collections::HashMap<u32, connector::StreamConnector>;

pub use client::TcpClient;

#[cfg(test)]
mod test_utils {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        sync::{Mutex, OnceLock},
    };

    use log::info;

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
}
