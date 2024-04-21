use std::{
    any::Any,
    collections::HashMap,
    io::Read,
    mem::{self, MaybeUninit},
    net::TcpStream,
};

use crate::{stream::Stream, ArcMutex, Sendable};
#[repr(transparent)]
struct Unknown(u8);
/// The various data required to store a stream.
// TODO: This implementation is kinda clunky and relies on a lot of unsafe code.
// I don't know a better way to do this, but I'm sure there is one.
struct StreamData {
    raw_data: ArcMutex<Vec<Unknown>>,
    size: usize,
    grew_by: ArcMutex<usize>,
    conversion_fn: fn(&mut dyn Read) -> Vec<u8>,
}

impl StreamData {
    fn new<T: 'static + Sendable>(stream: &Stream<T>) -> Self {
        StreamData {
            raw_data: unsafe { mem::transmute(stream.get_vec()) },
            size: mem::size_of::<T>(),
            grew_by: stream.get_grow_by(),
            conversion_fn: T::as_conversion_fn(),
        }
    }
    /// Pushes data to the stream.
    /// Data is the raw data received from the socket.
    /// # Safety
    /// The caller must ensure that the data is the correct size for the type, and valid.
    unsafe fn push(&mut self, data: Vec<u8>) {
        let mut v = self.raw_data.lock().unwrap();
        // ptr, len in bytes, cap in bytes
        // converted to bytes because the vec we are "creating" is a Vec<u8> and not a Vec<T> where T is the type of the stream.
        let (ptr, len, cap) = (
            v.as_mut_ptr() as *mut u8,
            v.len() * self.size,
            v.capacity() * self.size,
        );
        // this vec MUST be forgotten, or else we will double free the data.
        let mut vec = Vec::from_raw_parts(ptr, len, cap);
        let mut data = (self.conversion_fn)(&mut data.as_slice());
        assert!(
            data.len() % self.size == 0,
            "Data is not the correct size for the type. Expected {}, got {}",
            self.size,
            data.len()
        );
        vec.append(&mut data);
        mem::forget(vec);
        *self.grew_by.lock().unwrap() += 1;
    }
}

pub struct TcpClient {
    socket: TcpStream,
    /// So... Im stuck.
    /// I need to store a hashmap of streams that all have different types.
    /// I can't use a generic type because I need to store them in a hashmap, unless I do something hacky like this:
    /// ```rust
    /// let t: HashMap<u32, Stream<UnknownType>> = HashMap::new();
    ///
    /// fn add_stream<T: 'static>(&mut self, stream: Stream<T>) {
    ///    let x = mem::transmute(stream);
    ///   self.streams.insert(stream.get_type_id(), x);
    /// }
    ///
    /// fn get_stream<T: 'static>(&mut self) -> Option<&mut Stream<T>> {
    ///    let x = self.streams.get(&crate::hash_type_id::<T>())?;
    ///   Some(mem::transmute(x))
    /// }
    /// ```
    /// But this approach is *extremely* gross and I don't want to do it.
    /// Anything that uses any type parameter or associated type will not work.
    /// This approach has a few pitfalls:
    /// - It's not type safe.
    /// - I don't know how a vec will react to its type being changed to a ZST.
    ///     - Theoretically, if we don't touch the stream without casting it back to its original type, it should be fine.
    ///     - If this is what I do, I think the HashMap would have to be hidden inside a opaque type, so that the user can't access it.
    ///
    /// I've made a mistake. This bases on the hashmap being a `Hashmap<u32, Stream<Unknown>>`, but it should be `HashMap<u32, Vec<Unknown>>`.
    /// I kinda doubt that you can dynamically transmute things to specific types, (as in like using a type id to get the type of the stream, and then transmuting it to that type).
    /// Actually, its supposed to be `HashMap<u32, Arc<Mutex<Vec<Unknown>>>>`.
    ///
    /// I think this is a kinda ***big*** problem. I want to cast a type param while keeping the original type param's information.
    /// I can't cast the type param back to add a new item to the stream, because we only have the type id.
    /// You would have to have a type that can be dynamically sized, and then cast it to the type that you want.
    /// Rust super does not do this.
    /// You would have to store `T` somewhere, and then cast it back to `T` when you need to add a new item to the stream.
    /// And like, machine code has no concept of types, so this would probably induce significant overhead.
    ///
    /// oh god. i have an idea.
    ///
    /// What if we take the underlying pointer the vec uses, and just use it to store the data?
    /// Length and capacity would have to be handled, and you might have to do some *incredibly* weird things to get the data out of the vec.
    /// We would have to store more information about each stream, like the size of the type, but this might work.
    ///
    streams: HashMap<u32, StreamData>,
}
#[test]
fn test_vec_bs() {
    let mut x: Vec<u8> = vec![0; 10];
    // actually, this doesn't seem too bad... The unknown type might have to be a 1 byte type, but this seems like it could work.
    // You would also probably have to do like memcpy to get the data in the vec..
    let ptr = x.as_mut_ptr();
    // assuming we don't touch the vec in **any** way where it could try to reallocate or something, this should work.
    // actually, we would have to grow the vec, so this wouldn't work.
    // we would have to do somthing kinda gross, (as if this isn't already gross) like this:
    // let mut buf = unsafe { Vec::from_raw_parts(ptr, x.len() * size_of_type, x.capacity() * size_of_type) };
    // fluff. another roadblock.
    // so if we do that, we would have to set the len of the original vec. The set_len fn checks the capacity of the vec. We cant set the capacity.
    // we *could* set the len in the struct, but repr(rust) structs layout isn't static and could change.
    // GOD this is frustrating. I know once i figure out a way to do this it will be fine, but god its driving me insane.
    // idea part 30. Keep the size of the type, a pointer to the vec, and a pointer to a number which will be how many elements the vec grew.
    // FLUFF THATS NOT EVEN THE BIGGEST ISSUE. How am i going to convert a Vec<u8> without knowing the type????
    // ok well wait that might not be that big of a deal.
    // along with the other data, store a vec that contains fn(Vec<u8>) -> Vec<u8> where the return is the layout in memory.
    struct StreamData {
        raw_data: ArcMutex<Vec<u8>>,
        size: usize,
        grew_by: ArcMutex<u8>,
        conv_fn: fn(Vec<u8>) -> Vec<u8>,
    }
}
