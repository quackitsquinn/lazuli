mod client;
mod connector;
mod listener;

pub(self) type StreamCollection = std::collections::HashMap<u32, connector::StreamConnector>;

pub use client::TcpClient;

#[cfg(test)]
mod test_utils {
    use std::{
        io::{Read, Write},
        net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    };

    use super::*;
    static PORTS: [u16; 3] = [13131, 13132, 13133];
    static ADDRESSES: [SocketAddr; 3] = [
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[0]),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[1]),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[2]),
    ];

    /// Creates a client and server pair.
    /// (client, server)
    pub(super) fn make_client_server_pair() -> (TcpClient, TcpClient) {
        use std::net::TcpListener;
        let server = TcpListener::bind(ADDRESSES.as_slice()).expect("Unable to make socket");

        let client = TcpClient::new(server.local_addr().unwrap()).unwrap();
        let server = server.accept().unwrap().0;

        (client, TcpClient::from_stream(server))
    }
}
