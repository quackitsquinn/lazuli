//! Contains the StreamConnector struct, which allows for the pushing of data into a Stream.

use std::{
    io::Read,
    mem::{self, ManuallyDrop},
};

use crate::{stream::Stream, ArcMutex, Sendable};

/// A single byte type that is used to store the raw data.
#[repr(transparent)]
struct Unknown(u8);

/// The various data required to store a stream.
/// More specifically, this un-types streams, while keeping some type information.
/// This is used to store streams in a hashmap without knowing the type.
pub struct StreamConnector {
    raw_data: ArcMutex<Vec<Unknown>>,
    vec_ptr: ArcMutex<*mut Unknown>,
    size: usize,
    grew: ArcMutex<usize>,
    conversion_fn: fn(&mut dyn Read) -> Vec<u8>,
}

impl StreamConnector {
    /// Creates a new StreamConnector from a Stream.
    pub fn new<T: 'static + Sendable>(stream: &Stream<T>) -> Self {
        StreamConnector {
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
    pub unsafe fn push(&mut self, data: Vec<u8>) {
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
/// TODO: figure out if this is *actually* safe.
/// Im fairly certain mostly everything in StreamConnector is locked behind an ArcMutex, so it should be safe.
unsafe impl Send for StreamConnector {}
unsafe impl Sync for StreamConnector {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_connector() {
        let mut stream = Stream::<u32>::new();
        let mut connector = StreamConnector::new(&stream);
        let data = vec![0, 0, 0, 0];
        unsafe { connector.push(data) };
        assert_eq!(stream.get().unwrap(), 0);
    }

    #[test]
    fn test_stream_connector_zst() {
        let mut stream = Stream::<()>::new();
        let mut connector = StreamConnector::new(&stream);
        let data = vec![];
        unsafe { connector.push(data) };
        assert_eq!(stream.get().unwrap(), ());
    }
}
