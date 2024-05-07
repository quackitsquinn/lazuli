use std::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{Arc, Mutex},
};

use crate::{ArcMutex, IOResult, Sendable, TcpClient};

pub struct Server {
    listener: TcpListener,
    streams: Vec<ArcMutex<TcpClient>>,
}
/// TODO: down the road, add a tokio feature flag and use tokio for various async operations.
impl Server {
    pub fn new<T: ToSocketAddrs>(addrs: T) -> IOResult<Self> {
        let listener = TcpListener::bind(addrs)?;
        Ok(Server {
            listener,
            streams: vec![],
        })
    }
    /// Accepts a connection.
    pub fn accept(&mut self) -> IOResult<ArcMutex<TcpClient>> {
        let stream = self.listener.accept()?.0;
        let stream = TcpClient::from_stream(stream);
        let stream = Arc::new(Mutex::new(stream));
        self.streams.push(stream.clone());
        Ok(stream)
    }

    /// Accepts n connections.
    pub fn accept_n(&mut self, n: usize) -> IOResult<Vec<ArcMutex<TcpClient>>> {
        let mut streams = vec![];
        for _ in 0..n {
            streams.push(self.accept()?);
        }
        Ok(streams)
    }
}

impl Server {
    /// Sends a message to all clients.
    pub fn broadcast<T: Sendable + 'static>(&self, data: &T) -> IOResult<()> {
        for stream in &self.streams {
            let mut stream = stream.lock().unwrap();
            stream.send(data)?;
        }
        Ok(())
    }

    pub fn local_addr(&self) -> IOResult<std::net::SocketAddr> {
        self.listener.local_addr()
    }
}

#[cfg(test)]
mod test {
    use crate::client::test_utils::make_server;

    use super::*;

    #[test]
    fn test_server() -> IOResult<()> {
        let mut server = make_server();
        let addr = server.local_addr()?;
        let mut client = TcpClient::new(addr)?;
        let mut u32_stream = client.stream::<u32>();
        let stream = server.accept()?;
        stream.lock().unwrap().send(&1234u32)?;
        client.recv()?;
        assert_eq!(u32_stream.get(), Some(1234));
        Ok(())
    }
}
