mod client;
mod connector;
mod input;
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
    static PORTS: [u16; 10] = [
        13131, 13132, 13133, 13134, 13135, 13136, 13137, 13138, 13139, 13140,
    ];
    static ADDRESSES: [SocketAddr; 10] = [
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[0]),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[1]),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[2]),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[3]),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[4]),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[5]),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[6]),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[7]),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[8]),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[9]),
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
