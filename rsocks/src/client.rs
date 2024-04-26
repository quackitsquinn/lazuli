use std::{
    any::Any,
    collections::HashMap,
    io::{self, Read, Write},
    mem::{self, ManuallyDrop, MaybeUninit},
    net::{SocketAddr, TcpStream},
};

use crate::{stream::Stream, ArcMutex, Sendable};
#[repr(transparent)]
struct Unknown(u8);

/// The various data required to store a stream.
// TODO: This implementation is kinda clunky and relies on a lot of unsafe code.
// I don't know a better way to do this, but I'm sure there is one.
struct StreamData {
    raw_data: ArcMutex<Vec<Unknown>>,
    vec_ptr: ArcMutex<*mut Unknown>,
    size: usize,
    grew: ArcMutex<usize>,
    conversion_fn: fn(&mut dyn Read) -> Vec<u8>,
}

impl StreamData {
    fn new<T: 'static + Sendable>(stream: &Stream<T>) -> Self {
        StreamData {
            raw_data: unsafe { mem::transmute(stream.get_vec()) },
            vec_ptr: unsafe { mem::transmute(stream.get_ptr()) },
            size: mem::size_of::<T>(),
            grew: stream.get_grow_by(),
            conversion_fn: T::as_conversion_fn(),
        }
    }
    /// Pushes data to the stream.
    /// Data is the raw data received from the socket.
    /// # Safety
    /// The caller must ensure that the data is the correct size for the type, and valid.
    unsafe fn push(&mut self, data: Vec<u8>) {
        let mut v = self.raw_data.lock().unwrap();
        // We don't need to do any pointer magic if the type is a ZST
        if data.len() == 0 && self.size == 0 {
            let len = v.len();
            unsafe { v.set_len(len + 1) };
            return;
        }
        // ptr, len in bytes, cap in bytes
        // The len and capacity are converted to bytes because we only know the size of T.
        let (ptr, len, cap) = (
            v.as_mut_ptr() as *mut u8,
            v.len() * self.size,
            v.capacity() * self.size,
        );
        // Create the vec of bytes.
        // We don't transmute v because it would have an invalid length and capacity.
        let mut vec = unsafe { Vec::from_raw_parts(ptr, len, cap) };
        // Run the conversion on the input bytes.
        let mut data = (self.conversion_fn)(&mut data.as_slice());
        // Check size.
        assert!(
            data.len() % self.size == 0,
            "Data is not the correct size for the type. Expected {}, got {}",
            self.size,
            data.len()
        );
        // Add the bytes to the array
        vec.append(&mut data);
        // Update the vector pointer in case it gets changed.
        *self.vec_ptr.lock().unwrap() = vec.as_mut_ptr() as *mut Unknown;
        // Forget the array we created to prevent any double-frees.
        let _ = ManuallyDrop::new(vec);
        // Increment how much the vec grew.
        *self.grew.lock().unwrap() += 1;
    }
}

pub struct TcpClient {
    socket: TcpStream,
    streams: HashMap<u32, StreamData>,
}

impl TcpClient {
    pub fn from_stream(stream: TcpStream) -> Self {
        TcpClient {
            socket: stream,
            streams: Default::default(),
        }
    }
    pub fn new(addr: SocketAddr) -> Result<TcpClient, io::Error> {
        let stream = TcpStream::connect(addr)?;
        Ok(Self::from_stream(stream))
    }
    /// Sends data to the socket.
    #[inline]
    pub fn send<T>(&mut self, data: &T) -> Result<(), io::Error>
    where
        T: Sendable + 'static,
    {
        let bytes = data.send();
        let p_header = data.header();
        self.socket.write_all(&p_header.to_bytes())?;
        self.socket.write_all(&bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, net::Ipv4Addr};

    use crate::{client::StreamData, stream::Stream, Sendable};

    use super::TcpClient;

    #[test]
    fn test_stream_data() {
        let mut stream: Stream<u32> = Stream::new();
        let mut data = StreamData::new(&stream);
        unsafe { data.push(30u32.send()) };
        assert_eq!(stream.get().unwrap(), 30);
    }

    #[test]
    fn test_send() {
        // let mut client =
        //     TcpClient::new((Ipv4Addr::LOCALHOST, 13131).into()).expect("Unable to make socket");
        // client.send(&"sent data".to_owned()).unwrap();
        // client.send(&0xFFFFu16).unwrap();
    }
    struct TestStruct {
        a: u32,
        b: u32,
    }

    impl Sendable for TestStruct {
        type Error = std::io::Error;
        const SIZE_CONST: bool = true;
        fn send(&self) -> Vec<u8> {
            let mut buf = Vec::new();
            buf.extend(self.a.send());
            buf.extend(self.b.send());
            buf
        }
        fn recv(data: &mut dyn std::io::prelude::Read) -> Result<Self, Self::Error> {
            let a = u32::recv(data)?;
            let b = u32::recv(data)?;
            Ok(TestStruct { a, b })
        }
    }

    #[test]
    fn test_stream_data_struct() {
        let mut stream: Stream<TestStruct> = Stream::new();
        let mut data = StreamData::new(&stream);
        unsafe { data.push(TestStruct { a: 30, b: 40 }.send()) };
        let x = stream.get().unwrap();
        assert_eq!(x.a, 30);
        assert_eq!(x.b, 40);
    }
    struct ZST;
    impl Sendable for ZST {
        const SIZE_CONST: bool = true;
        type Error = Infallible;

        fn send(&self) -> Vec<u8> {
            vec![]
        }

        fn recv(_: &mut dyn std::io::prelude::Read) -> Result<Self, Self::Error> {
            Ok(ZST)
        }
    }
    #[test]
    fn test_zst() {
        let mut stream: Stream<ZST> = Stream::new();
        let mut data = StreamData::new(&stream);
        unsafe { data.push(ZST.send()) }
        assert!(stream.get().is_some())
    }
}
