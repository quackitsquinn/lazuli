use std::{
    fmt::Debug,
    io::{self, Read, Write},
    mem,
    net::{TcpStream, ToSocketAddrs},
    sync::{Arc, Mutex},
};

use crate::{hash_type_id, stream::Stream, ArcMutex, PacketHeader, Sendable, UnknownType};

use super::{connector::StreamConnector, listener::SocketListener, StreamCollection};

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

    pub fn new<T: ToSocketAddrs>(addr: T) -> Result<TcpClient, io::Error> {
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
    pub fn send<T>(&mut self, data: &T) -> Result<(), io::Error>
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
    pub fn recv(&mut self) -> Result<(), io::Error> {
        let mut buf: [u8; 20] = [0; mem::size_of::<PacketHeader<UnknownType>>()];
        let mut socket = self.socket.lock().unwrap();
        socket.read_exact(&mut buf)?;
        //dbg!("wijbnqewpiurnvqewpiovq");
        let header = unsafe { PacketHeader::from_bytes_unchecked(&buf) };
        let mut data: Vec<u8> = vec![0; header.payload_size as usize];
        // yeah ok it's this read_exact call.
        // ok i think i know whats happening.
        // this read_exact call is unable to read the data, forcing the fn to return an error.
        // then the fn is called again, with non header data, and it attempts to parse the payload as a header.
        // if the type is small, it returns at the first read_exact call. (if the sent data is bigger than mem::size_of::<PacketHeader>())
        // if the type is big, it will probably panic at PacketHeader::from_bytes_unchecked, because the RSOCK header is almost certainly not there.
        // I think a maybe solution is to figure out how to loop the read_exact call until it reads all the data.
        // I don't know how it would handle shutting down the socket though, as it would just hang forever.
        // i mean ok, my original idea was to abstract this method into a function that you just give some params to.
        // i didn't do it because i figured the other way would be easier. guess who was wrong.
        // this would probably explain why the weird debug statement was fixing the issue.
        // god threading is a mess sometimes.
        // TODO: fix this awful issue by abstracting this code. Use a modified version of the abstracted code in the listener.
        // Abstracting is probably a good idea for the long-run as well.
        // also in the future, this fn will probably intentionally not work if there is an active listener.
        // The reason this happened was because the listener was in non-blocking mode, and the socket was blocking.
        // This can have special code to handle it, but that code is for the listener.
        socket.read_exact(&mut data[0..header.payload_size as usize])?;
        println!("Received header: {:?}", buf);
        if !header.verify_checksum(&data) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Checksum verification failed",
            ));
        }
        if let Some(info) = self.streams.lock().unwrap().get_mut(&header.id()) {
            println!("Stream found for id: {}", header.id());
            unsafe { info.push(data) }
        } else {
            eprintln!("No stream found for id: {}", header.id());
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
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
        thread,
        time::Duration,
    };

    use crate::{client::test_utils::make_client_server_pair, stream::Stream, Sendable};

    use super::{StreamConnector, TcpClient};

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
