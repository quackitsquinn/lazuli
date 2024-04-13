//! A very *very* quick and dirty implementation of how we can send and receive structs over a network.

use std::{
    collections::VecDeque,
    io::{Cursor, Read},
};

struct ByteStack {
    // TODO: in future implementations, we will probably just use a standalone Cursor rather than a wrapper
    data: Cursor<Vec<u8>>,
}

impl ByteStack {
    fn new() -> ByteStack {
        ByteStack {
            data: Cursor::new(Vec::new()),
        }
    }

    fn from_bytes(data: Vec<u8>) -> ByteStack {
        ByteStack {
            data: Cursor::new(data),
        }
    }

    fn append(&mut self, data: Vec<u8>) {
        self.data.get_mut().extend(data);
    }

    fn read(&mut self, data: &mut [u8]) {
        self.data.read_exact(data).unwrap();
    }
}

trait Sendable {
    fn send(&self) -> Vec<u8>;
    fn recv(data: &mut ByteStack) -> Self;
}

macro_rules! sendable_int {
    ($t:ty) => {
        impl Sendable for $t {
            fn send(&self) -> Vec<u8> {
                <$t>::to_be_bytes(*self).to_vec()
            }

            fn recv(data: &mut ByteStack) -> Self{
                // We convert the slice from le bytes because the buffer gets reversed when we pop it
                let mut buffer = [0; std::mem::size_of::<$t>()];
                data.read(&mut buffer);
                <$t>::from_be_bytes(buffer)
            }
        }
    };
    ($($t:ty),*) => {
        $(sendable_int!($t);)*
    };
}

sendable_int!(u8, u16, u32, u64);

#[cfg(test)]
mod tests {
    macro_rules! test_sendable {
        ($t:ty, $v:expr) => {
            let mut data = ByteStack::new();
            let mut value: $t = $v;
            value.send().iter().for_each(|&b| data.append(vec![b]));
            let mut new_value: $t = Default::default();
            new_value = <$t>::recv(&mut data);
            assert_eq!(value, new_value);
        };
        ($($t:ty, $v:expr),*) => {
            $(test_sendable!($t, $v);)*
        };
    }

    use std::io::Read;

    use super::*;
    #[test]
    fn test_sendable() {
        test_sendable!(u8, 0, u16, 1, u32, 2, u64, 3);
    }
}
