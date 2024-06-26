//! A module for types that can be sent over the network.
//!
//! This module contains the Sendable trait, which is used to convert types to bytes that can be sent over the network.
//! The Sendable trait is implemented for all primitive types, and can be derived for custom types.
//!
//! Why use a Trait?
//! - The Sendable trait allows for any type that has stuff such as heap allocations to be sent over the network.
//! - The Sendable trait allows for optimization of the size of the type when sent over the network.
//!     - This in particularly useful for Option<T> because it can be sent as a single byte.
//! - The Sendable trait allows for the type to be converted to bytes in a way that is easy to implement.
//!
//!

use core::slice;
use std::{
    io::{self, Read},
    mem,
};

use log::trace;

use crate::header::PacketHeader;
use crate::Result;

/// A trait for types that can be sent over the network.
///
/// Sendable has the Debug bound because it is internally useful, and can be helpful for debugging.
pub trait Sendable: Sized + std::fmt::Debug {
    /// Returns the header of the packet.
    fn header(&self) -> PacketHeader<Self> {
        unsafe { PacketHeader::new(self.size()) }
    }

    /// Returns the size of the type.
    ///
    /// **This does not return the size of the type in memory, but the size of the type when sent over the network.**
    fn size(&self) -> u32 {
        std::mem::size_of::<Self>() as u32
    }

    /// Converts the type to a Vec<u8> that can be sent over the network.
    fn send(&self) -> Vec<u8>;

    /// Converts an incoming stream of bytes to the type.
    fn recv(data: &mut dyn Read) -> Result<Self>;
}

/// Converts the type to a function that can be used to convert incoming data to the type.
/// This function hides the type of the data, allowing for the conversion function to be used in a generic context.
///
/// This function is used internally by `StreamConnector`.
pub(crate) fn as_conversion_fn<T: Sendable>() -> fn(&mut dyn Read) -> Result<Box<[u8]>> {
    |data| {
        let conversion = Box::new(T::recv(data)?);
        trace!("Converted to bytes: {:?}", conversion);
        let as_slice_bytes = unsafe {
            // We use a slice to get the bytes of the type. This is safe because we are using the size of the type to get the slice.
            slice::from_raw_parts(
                Box::leak(conversion) as *mut T as *mut u8,
                mem::size_of::<T>(),
            )
        };
        Ok(as_slice_bytes.into())
    }
}

macro_rules! impl_sendable_number {
    ($t:ty) => {
        impl Sendable for $t {
            fn send(&self) -> Vec<u8> {
                // Follow the standard of big-endian
                <$t>::to_ne_bytes(*self).to_vec()
            }

            fn recv(data: &mut dyn Read,) -> Result<Self> {
                let mut buffer = [0; std::mem::size_of::<$t>()];
                data.read_exact(&mut buffer)?;
                Ok(<$t>::from_ne_bytes(buffer))
            }
        }
    };

    ($($t:ty),*) => {
        $(impl_sendable_number!($t);)*
    };
}

impl_sendable_number!(u8, u16, u32, u64, u128);

impl_sendable_number!(i8, i16, i32, i64, i128);

impl_sendable_number!(f32, f64);

impl Sendable for bool {
    fn send(&self) -> Vec<u8> {
        if *self {
            vec![1]
        } else {
            vec![0]
        }
    }

    fn recv(data: &mut dyn Read) -> Result<Self> {
        let mut buffer = [0; 1];
        data.read_exact(&mut buffer)?;
        Ok(buffer[0] != 0)
    }
}

impl<T> Sendable for Vec<T>
where
    T: Sendable,
{
    fn header(&self) -> PacketHeader<Self> {
        unsafe { PacketHeader::new(self.size()) }
    }

    fn size(&self) -> u32 {
        let mut size = 0;
        for item in self {
            size += item.size();
        }
        size + 4
    }

    fn send(&self) -> Vec<u8> {
        let mut data: Vec<u8> = Vec::new();
        data.extend((self.len() as u32).send());
        for item in self {
            data.extend(item.send());
        }
        data
    }

    fn recv(data: &mut dyn Read) -> Result<Self> {
        let mut vec = Vec::new();
        let length = u32::recv(data)?;
        for _ in 0..length {
            vec.push(T::recv(data)?);
        }
        Ok(vec)
    }
}

impl Sendable for String {
    fn header(&self) -> PacketHeader<Self> {
        unsafe { PacketHeader::new(self.size()) }
    }
    fn size(&self) -> u32 {
        self.len() as u32 + 4 // Add 4 bytes for the length of the string.
    }

    fn send(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend((self.len() as u32).send());
        data.extend(self.as_bytes());
        data
    }

    fn recv(data: &mut dyn Read) -> Result<Self> {
        let length = u32::recv(data)?;
        let mut buffer = vec![0; length as usize];
        data.read_exact(&mut buffer)?;
        let string = String::from_utf8(buffer);
        match string {
            Ok(s) => {
                trace!("Received string: {}", s);
                Ok(s)
            }
            Err(_) => Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8").into()),
        }
    }
}

impl<T> Sendable for Option<T>
where
    T: Sendable,
{
    fn header(&self) -> PacketHeader<Self> {
        match self {
            Some(value) => unsafe { PacketHeader::new(value.size() + 1) },
            None => unsafe { PacketHeader::new(1) },
        }
    }

    fn size(&self) -> u32 {
        match self {
            Some(value) => value.size() + 1,
            None => 1,
        }
    }

    fn send(&self) -> Vec<u8> {
        let mut data = Vec::new();
        match self {
            Some(value) => {
                data.extend(true.send());
                data.extend(value.send());
            }
            None => {
                data.extend(false.send());
            }
        }
        data
    }

    fn recv(data: &mut dyn Read) -> Result<Self> {
        let is_present = bool::recv(data).unwrap();
        if !is_present {
            Ok(None)
        } else {
            Ok(Some(T::recv(data)?))
        }
    }
}

impl<T> Sendable for Box<T>
where
    T: Sendable + Copy,
{
    fn header(&self) -> PacketHeader<Self> {
        unsafe { PacketHeader::new(self.size()) }
    }

    fn size(&self) -> u32 {
        T::size(&**self)
    }

    fn send(&self) -> Vec<u8> {
        T::send(&**self)
    }

    fn recv(data: &mut dyn Read) -> Result<Self> {
        Ok(Box::new(T::recv(data)?))
    }
}

macro_rules! impl_sendable_tuple {
    ($($name:ident)+) => {
        #[allow(non_snake_case)]
        impl<$($name: Sendable + std::fmt::Debug,)*> Sendable for ($($name,)*) {
            fn size(&self) -> u32{
                let ($(ref $name,)*) = *self;
                let mut total = 0;
                $(total += $name.size();)*
                total
            }

            fn send(&self) -> Vec<u8> {
                let ($(ref $name,)*) = *self;
                let mut buf = Vec::new();
                $(buf.extend($name.send());)*
                buf
            }

            fn recv(reader: &mut dyn std::io::Read) -> Result<Self >{
                Ok(($($name::recv(reader)?,)*))
            }

        }
    };
}
// Implement the Sendable trait for tuples of size 0 to 12.
// We don't go above 12 because A. A tuple of size 12 is already pretty big, and B. Debug implements up to 12.
impl_sendable_tuple!(A);
impl_sendable_tuple!(A B);
impl_sendable_tuple!(A B C);
impl_sendable_tuple!(A B C D);
impl_sendable_tuple!(A B C D E);
impl_sendable_tuple!(A B C D E F);
impl_sendable_tuple!(A B C D E F G );
impl_sendable_tuple!(A B C D E F G H );
impl_sendable_tuple!(A B C D E F G H I);
impl_sendable_tuple!(A B C D E F G H I J);
impl_sendable_tuple!(A B C D E F G H I J K);
impl_sendable_tuple!(A B C D E F G H I J K L);

impl Sendable for () {
    fn size(&self) -> u32 {
        0
    }

    fn send(&self) -> Vec<u8> {
        Vec::new()
    }

    fn recv(_reader: &mut dyn std::io::Read) -> Result<Self> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Thank god for macros.
    use super::*;
    macro_rules! test_sendable_number {
        ($t:ty, $name: ident) => {
            #[test]
            fn $name() {
                let value: $t = 42.0 as $t;
                let data = value.send();
                let mut reader = std::io::Cursor::new(&data);
                let result = <$t>::recv(&mut reader).unwrap();
                assert_eq!(value, result);
            }
        };
        ($($t:ty, $name:ident),*) => {
            $(test_sendable_number!($t, $name);)*
        };
    }
    // The vs code rust analyzer just shows like 12 run test buttons, which is mildly funny.
    test_sendable_number!(
        u8, test_u8, u16, test_u16, u32, test_u32, u64, test_u64, u128, test_u128, i8, test_i8,
        i16, test_i16, i32, test_i32, i64, test_i64, i128, test_i128, f32, test_f32, f64, test_f64
    );

    macro_rules! test_sendable_vec {
        ($t: ty, $name: ident) => {
            #[test]
            fn $name() {
                let value: Vec<$t> = vec![1 as $t, 2 as $t, 3 as $t, 4 as $t, 5 as $t];
                let data = value.send();
                let mut reader = std::io::Cursor::new(&data);
                let result = Vec::<$t>::recv(&mut reader).unwrap();
                assert_eq!(value, result);
            }
        };
        ($($t:ty, $name:ident),*) => {
            $(test_sendable_vec!($t, $name);)*
        };
    }
    test_sendable_vec!(
        u8,
        test_u8_vec,
        u16,
        test_u16_vec,
        u32,
        test_u32_vec,
        u64,
        test_u64_vec,
        u128,
        test_u128_vec,
        i8,
        test_i8_vec,
        i16,
        test_i16_vec,
        i32,
        test_i32_vec,
        i64,
        test_i64_vec,
        i128,
        test_i128_vec,
        f32,
        test_f32_vec,
        f64,
        test_f64_vec
    );

    #[test]
    fn test_vec_variable_size() {
        let mut vecs = Vec::<Vec<u8>>::new();
        for i in 0..10 {
            let mut vec = Vec::new();
            for j in 0..i {
                vec.push(j);
            }
            vecs.push(vec);
        }
        let data = vecs.send();
        let mut reader = std::io::Cursor::new(&data);
        let result = Vec::<Vec<u8>>::recv(&mut reader).unwrap();
        assert_eq!(vecs, result);
    }
    #[test]
    fn test_string_send() {
        let value = "Hello, World!".to_string();
        let data = value.send();
        let mut reader = std::io::Cursor::new(&data);
        let result = String::recv(&mut reader).unwrap();
        assert_eq!(value, result);
    }

    #[test]
    fn test_option_send_some() {
        let value = Some(42);
        let data = value.send();
        assert_eq!(data[0], 1);
        let mut reader = std::io::Cursor::new(&data);
        let result = Option::<u32>::recv(&mut reader).unwrap();
        assert_eq!(value, result);
    }

    #[test]
    fn test_option_send_none() {
        let value = None;
        let data = value.send();
        assert_eq!(data[0], 0);
        let mut reader = std::io::Cursor::new(&data);
        let result = Option::<u32>::recv(&mut reader).unwrap();
        assert_eq!(value, result);
    }

    #[test]
    fn test_box_send() {
        let value = Box::new(42);
        let data = value.send();
        let mut reader = std::io::Cursor::new(&data);
        let result = Box::<u32>::recv(&mut reader).unwrap();
        assert_eq!(value, result);
    }

    #[test]
    fn test_tuple_send() {
        let t = (1u32, 10.0, String::from("Hello, World!"), vec![1, 2, 3, 4]);
        let data = t.send();
        let mut reader = std::io::Cursor::new(data);
        let recv: (u32, f64, String, Vec<i32>) = Sendable::recv(&mut reader).unwrap();
        assert_eq!(t, recv);
    }
    #[test]
    fn test_recursive_tuple_send() {
        let init = (1, 2);
        let init1 = (init, init);
        let send = (init1, init1);
        let data = send.send();
        let mut reader = std::io::Cursor::new(data);
        let recv: (((i32, i32), (i32, i32)), ((i32, i32), (i32, i32))) =
            Sendable::recv(&mut reader).unwrap();
        assert_eq!(send, recv);
    }
}
