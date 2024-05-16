use std::{
    fmt::Debug,
    io::{self, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::{Arc, Mutex},
};

use log::trace;

use crate::{hash_type_id, stream::Stream, ArcMutex, Result, Sendable};

use super::{connector::StreamConnector, input, listener::SocketListener, StreamCollection};

pub struct Client {
    socket: ArcMutex<TcpStream>,
    streams: ArcMutex<StreamCollection>,
    listener: Option<SocketListener>,
}

impl Client {
    pub fn from_stream(stream: TcpStream) -> Self {
        Client {
            socket: Arc::new(Mutex::new(stream)),
            streams: Default::default(),
            listener: None,
        }
    }

    pub fn from_arcmutex_socket(stream: ArcMutex<TcpStream>) -> Self {
        Client {
            socket: stream,
            streams: Default::default(),
            listener: None,
        }
    }

    pub(crate) fn with_streams(mut self, streams: ArcMutex<StreamCollection>) -> Self {
        self.streams = streams;
        self
    }

    pub fn new<T: ToSocketAddrs>(addr: T) -> Result<Client> {
        let stream = addr.to_socket_addrs()?;
        for addr in stream {
            match TcpStream::connect(addr) {
                Ok(stream) => {
                    return Ok(Self::from_stream(stream));
                }
                Err(_) => continue,
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "No available addresses",
        ))
    }

    /// Sends data to the socket.
    #[inline]
    pub fn send<T>(&mut self, data: &T) -> Result<()>
    where
        T: Sendable + 'static + Debug,
    {
        let bytes = data.send();
        trace!("Sending data: {:?}", bytes);
        let mut p_header = data.header();
        p_header.calculate_checksum(&bytes);
        let mut socket = self.socket.lock().unwrap();
        socket.write_all(&p_header.to_bytes())?;
        socket.write_all(&bytes)?;
        Ok(())
    }
    /// Receives data from the socket.
    /// This is blocking, and for now, manual.
    pub fn recv(&mut self) -> Result<()> {
        if self.listener.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Cannot receive data while listening. If you want to stop listening, call stop_listening() first.",
            ));
        }
        let header = input::input_header(&mut self.socket.lock().unwrap())?;
        trace!("Received header: {:?}", header);
        let data = input::input_data(&mut self.socket.lock().unwrap(), &header)?;
        trace!("Received data: {:?}", data);
        input::verify_checksum(&header, &data)?;
        trace!("Checksum verified");
        let mut stream = self.streams.lock().unwrap();
        if let Some(info) = stream.get_mut(&header.id()) {
            info.push(data, header)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Stream not found for data",
            ));
        }
        Ok(())
    }

    pub fn stream<T>(&mut self) -> Stream<T>
    where
        T: Sendable + 'static,
    {
        let stream: Stream<T> = Stream::new();
        let info = StreamConnector::new(&stream);
        self.streams
            .lock()
            .unwrap()
            .insert(hash_type_id::<T>(), info);
        stream
    }

    pub fn listen(&mut self) -> Result<()> {
        let listener = SocketListener::new(self.socket.clone(), self.streams.clone());
        self.listener = Some(listener);
        self.listener.as_mut().unwrap().run()?;
        Ok(())
    }

    pub fn stop_listening(&mut self) {
        if let Some(listener) = &mut self.listener {
            println!("Stopping listener...");
            listener.stop().unwrap();
        }
    }

    pub fn error(&self) -> Option<io::Error> {
        self.listener.as_ref().and_then(|l| l.error())
    }

    pub fn is_connected(&self) -> bool {
        self.socket.lock().unwrap().peer_addr().is_ok()
    }

    pub fn peer_addr(&self) -> io::Result<std::net::SocketAddr> {
        self.socket.lock().unwrap().peer_addr()
    }
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.socket.lock().unwrap().set_nonblocking(nonblocking)
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use crate::{client::test_utils::make_client_server_pair, stream::Stream, Result, Sendable};

    use super::StreamConnector;

    macro_rules! test_send_recv_num {
        ($name: ident, $type: ty, $val: expr) => {
            #[test]
            fn $name() {
                let (mut client, mut server) = make_client_server_pair();
                let mut stream = client.stream::<$type>();
                server.send(&($val as $type)).unwrap();
                client.recv().unwrap();
                assert_eq!(stream.get().unwrap(), $val);
            }
        };
        // handle multiple sets
        ($($name: ident, $type: ty, $val: expr), *) => {
            $(test_send_recv_num!($name, $type, $val);)*
        };
    }
    mod send_recv_num_tests {
        use super::*;
        test_send_recv_num!(
            test_send_u8,
            u8,
            1,
            test_send_u16,
            u16,
            2,
            test_send_u32,
            u32,
            3,
            test_send_u64,
            u64,
            4,
            test_send_u128,
            u128,
            5,
            test_send_i8,
            i8,
            -1,
            test_send_i16,
            i16,
            -2,
            test_send_i32,
            i32,
            -3,
            test_send_i64,
            i64,
            -4,
            test_send_i128,
            i128,
            -5,
            test_send_f32,
            f32,
            1.0,
            test_send_f64,
            f64,
            2.0
        );
    }

    #[test]
    fn test_send_recv_string() {
        crate::init_logging();
        let (mut client, mut server) = make_client_server_pair();
        let mut stream = client.stream::<String>();
        server.send(&"Hello, world!".to_string()).unwrap();
        client.recv().unwrap();
        let data = stream.get().unwrap();
        assert_eq!(data, "Hello, world!".to_string());
    }
    #[test]
    fn test_send_recv_vec() {
        crate::init_logging();
        let data: Vec<u8> = vec![0, 1, 2, 3, 4, 5];
        let (mut client, mut server) = make_client_server_pair();
        let mut stream = client.stream::<Vec<u8>>();
        server.send(&data).unwrap();
        client.recv().unwrap();
        assert_eq!(stream.get().unwrap(), data);
    }
    #[derive(Debug)]
    struct TestStruct {
        a: u32,
        b: u32,
    }

    impl Sendable for TestStruct {
        fn send(&self) -> Vec<u8> {
            let mut buf = Vec::new();
            buf.extend(self.a.send());
            buf.extend(self.b.send());
            buf
        }
        fn recv(data: &mut dyn std::io::prelude::Read) -> Result<Self> {
            let a = u32::recv(data)?;
            let b = u32::recv(data)?;
            Ok(TestStruct { a, b })
        }
    }

    #[test]
    fn test_stream_data_struct() {
        let mut stream: Stream<TestStruct> = Stream::new();
        let mut data = StreamConnector::new(&stream);
        unsafe {
            data.push_raw(TestStruct { a: 30, b: 40 }.send().into())
                .unwrap()
        };
        let x = stream.get().unwrap();
        assert_eq!(x.a, 30);
        assert_eq!(x.b, 40);
    }
}
