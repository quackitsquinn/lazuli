use std::{
    mem,
    rc::Rc,
    sync::{Arc, Mutex},
};

use crate::Sendable;

/// A stream of data received from a socket.
pub struct Stream<T> {
    data: Arc<Mutex<Vec<T>>>,
    grow_by: Arc<Mutex<usize>>,
    ptr: Arc<Mutex<*mut T>>,
}

impl<T> Stream<T>
where
    T: 'static,
{
    pub(crate) fn new() -> Self {
        Stream {
            data: Arc::new(Mutex::new(vec![])),
            grow_by: Arc::new(Mutex::new(0)),
            ptr: Arc::new(Mutex::new(std::ptr::null_mut())),
        }
    }

    fn check_vec(&mut self) {
        let mut grow_by = self.grow_by.lock().unwrap();
        println!("grow_by: {}", *grow_by);
        if *grow_by > 0 {
            let mut v = self.data.lock().unwrap();
            let (ptr, len, cap) = (v.as_mut_ptr(), v.len() + *grow_by, v.capacity() + *grow_by);
            println!("ptr: {:p}, len: {}, cap: {}", ptr, len, cap);
            // Because the way data outside the capacity of a vec is handled and how it has 0 guarantee of being valid
            // we need to do a Vec::from_raw_parts to ensure that the data is valid.
            // I don't think this process is expensive at all (from what i can tell, its just basic pointer arithmetic)
            let replaced = mem::replace(&mut *v, unsafe {
                Vec::from_raw_parts(*self.ptr.lock().unwrap(), len, cap)
            });
            // We need to forget the replaced vec, or else we will double free the data.
            mem::forget(replaced);
            *grow_by = 0;
        }
    }
    /// Gets one item from the stream.
    pub fn get(&mut self) -> Option<T> {
        self.check_vec();
        self.data.lock().unwrap().pop()
    }
    /// Gets the count of items in the stream.
    pub fn len(&self) -> usize {
        self.data.lock().unwrap().len() + *self.grow_by.lock().unwrap()
    }
    /// Gets a pointer to the underlying buffer.
    pub fn get_vec(&self) -> Arc<Mutex<Vec<T>>> {
        self.data.clone()
    }
    pub fn get_ptr(&self) -> Arc<Mutex<*mut T>> {
        self.ptr.clone()
    }
    /// Sets the amount of items to grow the buffer by.
    pub(crate) fn get_grow_by(&self) -> Arc<Mutex<usize>> {
        self.grow_by.clone()
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
