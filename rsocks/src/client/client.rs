use std::{
    fmt::Debug,
    io::{self, Read, Write},
    mem,
    net::{TcpStream, ToSocketAddrs},
    sync::{Arc, Mutex},
};

use crate::{
    hash_type_id, stream::Stream, ArcMutex, IOResult, PacketHeader, Sendable, UnknownType,
};

use super::{connector::StreamConnector, input, listener::SocketListener, StreamCollection};

pub struct TcpClient {
    socket: ArcMutex<TcpStream>,
    streams: ArcMutex<StreamCollection>,
    listener: Option<SocketListener>,
}

impl TcpClient {
    pub fn from_stream(stream: TcpStream) -> Self {
        TcpClient {
            socket: Arc::new(Mutex::new(stream)),
            streams: Default::default(),
            listener: None,
        }
    }

    pub fn from_arcmutex_socket(stream: ArcMutex<TcpStream>) -> Self {
        TcpClient {
            socket: stream,
            streams: Default::default(),
            listener: None,
        }
    }

    pub(crate) fn with_streams(mut self, streams: ArcMutex<StreamCollection>) -> Self {
        self.streams = streams;
        self
    }

    pub fn new<T: ToSocketAddrs>(addr: T) -> IOResult<TcpClient> {
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
    pub fn send<T>(&mut self, data: &T) -> IOResult<()>
    where
        T: Sendable + 'static + Debug,
    {
        let bytes = data.send();
        let mut p_header = data.header();
        p_header.calculate_checksum(&bytes);
        let mut socket = self.socket.lock().unwrap();
        socket.write_all(&p_header.to_bytes())?;
        socket.write_all(&bytes)?;
        Ok(())
    }
    /// Receives data from the socket.
    /// This is blocking, and for now, manual.
    pub fn recv(&mut self) -> IOResult<()> {
        if self.listener.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Cannot receive data while listening. If you want to stop listening, call stop_listening() first.",
            ));
        }
        let header = input::input_header(&mut self.socket.lock().unwrap())?;
        let data = input::input_data(&mut self.socket.lock().unwrap(), &header)?;
        input::verify_checksum(&header, &data)?;
        let mut stream = self.streams.lock().unwrap();
        if let Some(info) = stream.get_mut(&header.id()) {
            info.push(data, header);
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

    pub fn listen(&mut self) {
        let mut listener = SocketListener::new(self.socket.clone(), self.streams.clone());
        self.listener = Some(listener);
        self.listener.as_mut().unwrap().run();
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
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
        thread,
        time::Duration,
    };

    use crate::{client::test_utils::make_client_server_pair, stream::Stream, IOResult, Sendable};

    use super::{StreamConnector, TcpClient};

    #[test]
    fn test_send_recv() {
        let (mut client, mut server) = make_client_server_pair();
        let mut stream = client.stream::<u32>();
        server.send(&30u32).unwrap();
        client.recv().unwrap();
        assert_eq!(stream.get().unwrap(), 30);
    }
    #[test]
    fn test_send_recv_string() {
        let (mut client, mut server) = make_client_server_pair();
        let mut stream = client.stream::<String>();
        server.send(&"Hello, world!".to_string()).unwrap();
        client.recv().unwrap();
        assert_eq!(stream.get().unwrap(), "Hello, world!".to_string());
    }
    #[test]
    fn test_send_recv_vec() {
        let (mut client, mut server) = make_client_server_pair();
        let mut stream = client.stream::<Vec<u8>>();
        server.send(&vec![0, 1, 2, 3, 4, 5]).unwrap();
        client.recv().unwrap();
        assert_eq!(stream.get().unwrap(), vec![0, 1, 2, 3, 4, 5]);
    }
    // Believe it or not, commenting out failing tests is bad practice.
    // It's fine here though, as this is a debug test which requires an already hosted server.

    // #[test]
    // fn test_send() {
    //     let mut client =
    //         TcpClient::new((Ipv4Addr::LOCALHOST, 13131)).expect("Unable to make socket");
    //     client.send(&"sent data".to_owned()).unwrap();
    //     client.send(&0xFFFFu16).unwrap();
    //     thread::sleep(Duration::from_secs(1));
    //     client.send(&0xFFFFu16).unwrap();
    //     client.send(&"sending u32".to_owned()).unwrap();
    //     client.send(&0xFFFFFFFFu32).unwrap();
    // }
    struct TestStruct {
        a: u32,
        b: u32,
    }

    impl Sendable for TestStruct {
        const SIZE_CONST: bool = true;
        fn send(&self) -> Vec<u8> {
            let mut buf = Vec::new();
            buf.extend(self.a.send());
            buf.extend(self.b.send());
            buf
        }
        fn recv(data: &mut dyn std::io::prelude::Read) -> IOResult<Self> {
            let a = u32::recv(data)?;
            let b = u32::recv(data)?;
            Ok(TestStruct { a, b })
        }
    }

    #[test]
    fn test_stream_data_struct() {
        let mut stream: Stream<TestStruct> = Stream::new();
        let mut data = StreamConnector::new(&stream);
        unsafe { data.push_raw(TestStruct { a: 30, b: 40 }.send()).unwrap() };
        let x = stream.get().unwrap();
        assert_eq!(x.a, 30);
        assert_eq!(x.b, 40);
    }
}
