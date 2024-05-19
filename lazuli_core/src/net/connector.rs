//! Contains the StreamConnector struct, which allows for the pushing of data into a Stream.

use std::{
    fmt::Debug,
    io::Read,
    mem::{self, ManuallyDrop},
};

use log::trace;

use crate::{sendable, stream::Stream, ArcMutex, PacketHeader, Result, Sendable, UnknownType};

/// A single byte type that is used to store the raw data.
#[repr(transparent)]
struct Unknown(u8);

/// The various data required to store a stream.
/// More specifically, this un-types streams, while keeping needed data.
// HACK: like this *whole* setup is a hack. I don't know if there is a better way to do this, but there probably is.
pub struct StreamConnector {
    raw_data: ArcMutex<Vec<Unknown>>,
    vec_ptr: ArcMutex<*mut Unknown>,
    size: usize,
    grew: ArcMutex<usize>,
    conversion_fn: fn(&mut dyn Read) -> Result<Box<[u8]>>,
    type_name: &'static str,
}

impl StreamConnector {
    /// Creates a new StreamConnector from a Stream.
    pub fn new<T: 'static + Sendable>(stream: &Stream<T>) -> Self {
        StreamConnector {
            raw_data: unsafe { mem::transmute(stream.get_vec()) },
            vec_ptr: unsafe { mem::transmute(stream.get_ptr()) },
            size: mem::size_of::<T>(),
            grew: stream.get_grow_by(),
            conversion_fn: sendable::as_conversion_fn::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }
    /// Pushes data to the stream.
    /// Data is the raw data received from the socket.
    /// # Safety
    /// The caller must ensure that the data is the correct size for the type, and valid.
    pub unsafe fn push_raw(&mut self, data: Box<[u8]>) -> Result<()> {
        let mut v = self.raw_data.lock().unwrap();
        // We don't need to do any pointer magic if the type is a ZST
        if data.len() == 0 && self.size == 0 {
            let len = v.len();
            unsafe { v.set_len(len + 1) };
            return Ok(());
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
        // Check size.
        assert!(
            data.len() % self.size == 0,
            "Data is not the correct size for the type. Expected {}, got {}",
            self.size,
            data.len()
        );
        // Add the bytes to the array
        vec.append(data.to_vec().as_mut());
        // Update the vector pointer in case it gets changed.
        *self.vec_ptr.lock().unwrap() = vec.as_mut_ptr() as *mut Unknown;
        // Forget the array we created to prevent any double-frees.
        let _ = ManuallyDrop::new(vec);
        // Increment how much the vec grew.
        *self.grew.lock().unwrap() += 1;
        Ok(())
    }

    pub fn push(&mut self, data: Vec<u8>, header: PacketHeader<UnknownType>) -> Result<()> {
        debug_assert_eq!(header.payload_size as usize, data.len());
        // Create a cursor from the data.
        let mut cursor = std::io::Cursor::new(data);
        let converted = (self.conversion_fn)(&mut cursor)?;
        trace!("Converted data: {:?}", converted);
        assert!(
            converted.len() == self.size,
            "Data is not the correct size for the type."
        );
        unsafe { self.push_raw(converted)? };
        Ok(())
    }
    /// Returns the type name of the stream. This is mainly used for the debug implementation.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}

impl Debug for StreamConnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamConnector")
            .field("type_name", &self.type_name)
            .field("size", &self.size)
            .finish()
    }
}

/// Everything in StreamConnector is behind a mutex, besides the size. The size should never change.
unsafe impl Send for StreamConnector {}
unsafe impl Sync for StreamConnector {}

#[cfg(test)]
mod tests {
    use core::slice;

    use super::*;

    #[test]
    fn test_stream_connector() {
        let mut stream = Stream::<u32>::new();
        let mut connector = StreamConnector::new(&stream);
        let data = vec![0, 0, 0, 0];
        unsafe { connector.push_raw(data.into()).unwrap() };
        assert_eq!(stream.get().unwrap(), 0);
    }

    #[test]
    fn test_string() {
        let mut stream = Stream::<String>::new();
        let mut connector = StreamConnector::new(&stream);
        let data = "Hello, world!".to_owned();
        unsafe {
            connector
                .push_raw(
                    slice::from_raw_parts(
                        &data as *const String as *const u8,
                        mem::size_of::<String>(),
                    )
                    .into(),
                )
                .unwrap();
            mem::forget(data);
        };
        assert_eq!(stream.get().unwrap(), "Hello, world!".to_string());
    }

    #[test]
    fn test_stream_connector_zst() {
        let mut stream = Stream::<()>::new();
        let mut connector = StreamConnector::new(&stream);
        let data = vec![];
        unsafe { connector.push_raw(data.into()).unwrap() };
        assert_eq!(stream.get().unwrap(), ());
    }
}
