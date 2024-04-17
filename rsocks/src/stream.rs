use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use crate::Sendable;

/// A stream of data received from a socket.
pub struct Stream<T>
where
    T: Sendable,
{
    data: Arc<Mutex<Vec<T>>>,
}

impl<T> Stream<T>
where
    T: Sendable + 'static,
{
    pub(crate) fn new() -> Self {
        Stream {
            data: Arc::new(Mutex::new(vec![])),
        }
    }
    /// Gets one item from the stream.
    pub fn get(&mut self) -> Option<T> {
        self.data.lock().unwrap().pop()
    }
    /// Gets the count of items in the stream.
    pub fn len(&self) -> usize {
        self.data.lock().unwrap().len()
    }
    /// Gets a pointer to the underlying buffer.
    pub fn get_vec(&mut self) -> Arc<Mutex<Vec<T>>> {
        self.data.clone()
    }
    /// Gets the type id of T
    pub(crate) fn get_type_id(self) -> u32 {
        crate::hash_type_id::<T>()
    }
}

#[cfg(test)]
mod test {
    use super::Stream;

    #[test]
    fn test_new_stream() {
        let mut t: Stream<u32> = Stream::new();
        assert_eq!(t.len(), 0);
    }
    #[test]
    fn test_pop_stream() {
        let mut stream = Stream::<u32>::new();
        let binding = stream.get_vec();
        let mut stream_input = binding.lock().unwrap();
        stream_input.push(9);
        drop(stream_input);
        assert_eq!(stream.get().unwrap(), 9);
        assert_eq!(stream.len(), 0);
    }
}
