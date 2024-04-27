use std::{
    mem::{self, ManuallyDrop},
    sync::{Arc, Mutex},
};

/// A stream of data received from a socket.
pub struct Stream<T> {
    data: Arc<Mutex<Vec<T>>>,
    grew: Arc<Mutex<usize>>,
    ptr: Arc<Mutex<*mut T>>,
}

impl<T> Stream<T>
where
    T: 'static,
{
    pub(crate) fn new() -> Self {
        Stream {
            data: Arc::new(Mutex::new(vec![])),
            grew: Arc::new(Mutex::new(0)),
            ptr: Arc::new(Mutex::new(std::ptr::null_mut())),
        }
    }

    fn check_vec(&mut self) {
        let mut grew_by = self.grew.lock().unwrap();
        // Check if the stream was given any data.
        if *grew_by > 0 {
            // Get the size and cap from the vector.
            let mut v = self.data.lock().unwrap();
            // Grabs the pointer to the vec that has the new elements.
            let (ptr, len, cap) = (
                *self.ptr.lock().unwrap(),
                v.len() + *grew_by,
                v.capacity() + *grew_by,
            );
            // Replace the old vec with a new one with correct values.
            let replaced = mem::replace(&mut *v, unsafe { Vec::from_raw_parts(ptr, len, cap) });
            // We need to forget the replaced vec, or else we will double free the data.
            let _ = ManuallyDrop::new(replaced);
            // Reset grew_by
            *grew_by = 0;
        }
    }
    /// Gets one item from the stream.
    pub fn get(&mut self) -> Option<T> {
        self.check_vec();
        self.data.lock().unwrap().pop()
    }
    /// Gets the count of items in the stream.
    pub fn len(&self) -> usize {
        self.data.lock().unwrap().len() + *self.grew.lock().unwrap()
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
        self.grew.clone()
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
        let t: Stream<u32> = Stream::new();
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
