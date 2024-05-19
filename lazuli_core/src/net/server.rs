use std::{
    net::{TcpListener, ToSocketAddrs},
    sync::{Arc, Mutex},
};

use crate::{ArcMutex, Client, Result, Sendable};

use super::config::{self, SocketConfig};

pub struct Server {
    listener: TcpListener,
    streams: Vec<ArcMutex<Client>>,
}
/// TODO: down the road, add a tokio feature flag and use tokio for various async operations.
impl Server {
    /// Creates a new server.
    pub fn new<T: ToSocketAddrs>(addrs: T) -> Result<Self> {
        let listener = TcpListener::bind(addrs)?;
        Ok(Server {
            listener,
            streams: vec![],
        })
    }
    /// Adds a configuration to the server.
    pub fn with_config(self, config: SocketConfig) -> Result<Self> {
        config.apply_listener(&self.listener)?;
        Ok(self)
    }
    /// Accepts a connection.
    pub fn accept(&mut self) -> Result<ArcMutex<Client>> {
        let stream = self.listener.accept()?.0;
        let stream = Client::from_stream(stream);
        let stream = Arc::new(Mutex::new(stream));
        self.streams.push(stream.clone());
        Ok(stream)
    }

    /// Accepts n connections.
    pub fn accept_n(&mut self, n: usize) -> Result<Vec<ArcMutex<Client>>> {
        let mut streams = vec![];
        for _ in 0..n {
            streams.push(self.accept()?);
        }
        Ok(streams)
    }

    pub fn incoming(&mut self) -> impl Iterator<Item = Result<ArcMutex<Client>>> + '_ {
        self.listener.incoming().map(|stream| {
            let stream = stream?;
            let stream = Client::from_stream(stream);
            let stream = Arc::new(Mutex::new(stream));
            self.streams.push(stream.clone());
            Ok(stream)
        })
    }
}

impl Server {
    /// Sends a message to all clients.
    pub fn broadcast<T: Sendable + 'static>(&self, data: &T) -> Result<()> {
        for stream in &self.streams {
            let mut stream = stream.lock().unwrap();
            stream.send(data)?;
        }
        Ok(())
    }
    /// Gets the local address of the server.
    pub fn local_addr(&self) -> Result<std::net::SocketAddr> {
        self.listener.local_addr()
    }
}

#[cfg(test)]
mod test {
    use std::net::Ipv4Addr;

    use crate::net::test_utils::{make_server, test_send_recv};

    use super::*;

    fn make_server_client_pair(server: &mut Server) -> (Client, ArcMutex<Client>) {
        let addr = server.local_addr().unwrap();
        let client = Client::connect(addr).unwrap();
        let server_client = server.accept().unwrap();
        (client, server_client)
    }

    #[test]
    fn test_server() -> Result<()> {
        let mut server = make_server();
        let (mut client, server_client) = make_server_client_pair(&mut server);
        test_send_recv(
            &mut client,
            &mut *server_client.lock().unwrap(),
            "Hello, world!".to_owned(),
        );
        Ok(())
    }

    #[test]
    fn test_broadcast() -> Result<()> {
        let mut server = make_server();
        let mut client1 = make_server_client_pair(&mut server);
        let mut client2 = make_server_client_pair(&mut server);
        let mut str_stream_1 = client1.0.stream::<String>();
        let mut str_stream_2 = client2.0.stream::<String>();
        println!("{:#?}", client1.0);
        server.broadcast(&"Hello, world!".to_owned())?;
        client1.0.recv()?;
        client2.0.recv()?;
        assert_eq!(str_stream_1.get().unwrap(), "Hello, world!".to_owned());
        assert_eq!(str_stream_2.get().unwrap(), "Hello, world!".to_owned());
        Ok(())
    }
    #[test]
    fn test_nonblocking_server() -> Result<()> {
        let mut server = Server::new((Ipv4Addr::LOCALHOST, 0))?
            .with_config(SocketConfig::new().blocking(false))?;
        assert!(server.accept().is_err());
        if let Err(e) = server.accept() {
            assert_eq!(e.kind(), std::io::ErrorKind::WouldBlock);
        }
        Ok(())
    }
}
