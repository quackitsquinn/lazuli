use std::{
    collections::HashMap,
    fmt::Debug,
    io::{self, Read, Write},
    mem,
    net::{SocketAddr, TcpStream, ToSocketAddrs},
};

use crate::{hash_type_id, stream::Stream, PacketHeader, Sendable, UnknownType};

use super::connector::StreamConnector;

pub struct TcpClient {
    socket: TcpStream,
    streams: HashMap<u32, StreamConnector>,
}

impl TcpClient {
    pub fn from_stream(stream: TcpStream) -> Self {
        TcpClient {
            socket: stream,
            streams: Default::default(),
        }
    }

    pub fn new<T: ToSocketAddrs>(addr: T) -> Result<TcpClient, io::Error> {
        let stream = addr.to_socket_addrs()?;
        for addr in stream {
            match TcpStream::connect(addr) {
                Ok(stream) => return Ok(Self::from_stream(stream)),
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
    pub fn send<T>(&mut self, data: &T) -> Result<(), io::Error>
    where
        T: Sendable + 'static + Debug,
    {
        let bytes = data.send();
        let mut p_header = data.header();
        p_header.calculate_checksum(&bytes);
        self.socket.write_all(&p_header.to_bytes())?;
        self.socket.write_all(&bytes)?;
        Ok(())
    }
    /// Receives data from the socket.
    /// This is blocking, and for now, manual.
    pub fn recv(&mut self) -> Result<(), io::Error> {
        let mut buf: [u8; 20] = [0; mem::size_of::<PacketHeader<UnknownType>>()];
        self.socket.read_exact(&mut buf)?;
        let header = unsafe { PacketHeader::from_bytes_unchecked(&buf) };
        let mut data: Vec<u8> = vec![0; header.payload_size as usize];
        self.socket
            .read_exact(&mut data[0..header.payload_size as usize])?;
        if !header.verify_checksum(&data) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Checksum verification failed",
            ));
        }
        if let Some(info) = self.streams.get_mut(&header.id()) {
            unsafe { info.push(data) }
        }
        Ok(())
    }

    pub fn stream<T>(&mut self) -> Stream<T>
    where
        T: Sendable + 'static,
    {
        let stream: Stream<T> = Stream::new();
        let info = StreamConnector::new(&stream);
        self.streams.insert(hash_type_id::<T>(), info);
        stream
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

    use crate::{stream::Stream, Sendable};

    use super::{StreamConnector, TcpClient};

    static PORTS: [u16; 3] = [13131, 13132, 13133];
    static ADDRESSES: [SocketAddr; 3] = [
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[0]),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[1]),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORTS[2]),
    ];
    /// Creates a client and server pair.
    /// (client, server)
    fn make_client_server_pair() -> (TcpClient, TcpClient) {
        let server = TcpListener::bind(ADDRESSES.as_slice()).expect("Unable to make socket");
        let client = TcpClient::new(server.local_addr().unwrap());
        let server = server.accept().unwrap().0;
        (client.unwrap(), TcpClient::from_stream(server))
    }

    #[test]
    fn test_send_recv() {
        let (mut client, mut server) = make_client_server_pair();
        let mut stream = client.stream::<u32>();
        server.send(&30u32).unwrap();
        client.recv().unwrap();
        assert_eq!(stream.get().unwrap(), 30);
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
        fn recv(data: &mut dyn std::io::prelude::Read) -> Result<Self, io::Error> {
            let a = u32::recv(data)?;
            let b = u32::recv(data)?;
            Ok(TestStruct { a, b })
        }
    }

    #[test]
    fn test_stream_data_struct() {
        let mut stream: Stream<TestStruct> = Stream::new();
        let mut data = StreamConnector::new(&stream);
        unsafe { data.push(TestStruct { a: 30, b: 40 }.send()) };
        let x = stream.get().unwrap();
        assert_eq!(x.a, 30);
        assert_eq!(x.b, 40);
    }
}
