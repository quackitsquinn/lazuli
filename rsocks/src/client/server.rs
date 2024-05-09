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
    /// Creates a new server.
    pub fn new<T: ToSocketAddrs>(addrs: T) -> IOResult<Self> {
        let listener = TcpListener::bind(addrs)?;
        Ok(Server {
            listener,
            streams: vec![],
        })
    }
    /// Creates a new non-blocking server.
    pub fn new_nonblocking<T: ToSocketAddrs>(addrs: T) -> IOResult<Self> {
        let listener = TcpListener::bind(addrs)?;
        listener.set_nonblocking(true)?;
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
    /// Gets the local address of the server.
    pub fn local_addr(&self) -> IOResult<std::net::SocketAddr> {
        self.listener.local_addr()
    }
}

#[cfg(test)]
mod test {
    use crate::client::test_utils::{get_socket_addr, make_server, test_send_recv};

    use super::*;

    fn make_server_client_pair(server: &mut Server) -> (TcpClient, ArcMutex<TcpClient>) {
        let addr = server.local_addr().unwrap();
        let mut client = TcpClient::new(addr).unwrap();
        let server_client = server.accept().unwrap();
        (client, server_client)
    }

    #[test]
    fn test_server() -> IOResult<()> {
        let mut server = make_server();
        let addr = server.local_addr()?;
        let (mut client, server_client) = make_server_client_pair(&mut server);
        test_send_recv(
            &mut client,
            &mut *server_client.lock().unwrap(),
            "Hello, world!".to_owned(),
        );
        Ok(())
    }

    #[test]
    fn test_brodcast() -> IOResult<()> {
        let mut server = make_server();
        let mut client1 = make_server_client_pair(&mut server);
        let mut client2 = make_server_client_pair(&mut server);
        let mut str_stream_1 = client1.0.stream::<String>();
        let mut str_stream_2 = client2.0.stream::<String>();
        server.broadcast(&"Hello, world!".to_owned())?;
        client1.0.recv()?;
        client2.0.recv()?;
        assert_eq!(str_stream_1.get().unwrap(), "Hello, world!".to_owned());
        assert_eq!(str_stream_2.get().unwrap(), "Hello, world!".to_owned());
        Ok(())
    }
    #[test]
    fn test_nonblocking_server() -> IOResult<()> {
        let mut server = Server::new_nonblocking(get_socket_addr())?;
        assert!(server.accept().is_err());
        if let Err(e) = server.accept() {
            assert_eq!(e.kind(), std::io::ErrorKind::WouldBlock);
        }
        Ok(())
    }
}
