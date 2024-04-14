use std::io::Read;

use crate::header::PacketHeader;

/// A trait for types that can be sent over the network.
pub trait Sendable: Sized {
    type Error: std::error::Error;
    /// Returns the header of the packet.
    fn header(&self) -> PacketHeader<Self> {
        unsafe { PacketHeader::new(self.size()) }
    }
    /// Returns whether the size of the type is constant.
    /// This is used to determine if the type needs special handling.
    fn size_const() -> bool {
        true
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
    fn recv(data: &mut dyn Read) -> Result<Self, Self::Error>;
}

macro_rules! impl_sendable_number {
    ($t:ty) => {
        impl Sendable for $t {
            type Error = std::io::Error;
            fn send(&self) -> Vec<u8> {
                // Follow the standard of big-endian
                <$t>::to_be_bytes(*self).to_vec()
            }

            fn recv(data: &mut dyn Read,) -> Result<Self, Self::Error> {
                let mut buffer = [0; std::mem::size_of::<$t>()];
                data.read_exact(&mut buffer)?;
                Ok(<$t>::from_be_bytes(buffer))
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
    type Error = std::io::Error;

    fn send(&self) -> Vec<u8> {
        if *self {
            vec![1]
        } else {
            vec![0]
        }
    }

    fn recv(data: &mut dyn Read) -> Result<Self, Self::Error> {
        let mut buffer = [0; 1];
        data.read_exact(&mut buffer)?;
        Ok(buffer[0] != 0)
    }
}
impl<T> Sendable for Vec<T>
where
    T: Sendable,
{
    type Error = T::Error;
    fn header(&self) -> PacketHeader<Self> {
        unsafe { PacketHeader::new(self.size()) }
    }

    fn size_const() -> bool {
        false
    }

    fn size(&self) -> u32 {
        if T::size_const() {
            // Safety: We know that the size of the payload is constant, so we can calculate the size of the payload.
            (std::mem::size_of::<T>() * self.len() + 4) as u32 // Add 4 bytes for the length of the vector.
        } else {
            let mut size = 0;
            for item in self {
                size += item.size();
            }
            // Safety: We have just calculated the size of the payload.
            size + 4 // Add 4 bytes for the length of the vector.
        }
    }

    fn send(&self) -> Vec<u8> {
        let mut data: Vec<u8> = Vec::new();
        data.extend((self.len() as u32).send());
        for item in self {
            data.extend(item.send());
        }
        data
    }

    fn recv(data: &mut dyn Read) -> Result<Self, Self::Error> {
        let mut vec = Vec::new();
        let length = u32::recv(data).unwrap_or(0);
        for _ in 0..length {
            vec.push(T::recv(data)?);
        }
        Ok(vec)
    }
}

impl Sendable for String {
    type Error = std::io::Error;
    fn header(&self) -> PacketHeader<Self> {
        unsafe { PacketHeader::new(self.size()) }
    }

    fn size_const() -> bool {
        false
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

    fn recv(data: &mut dyn Read) -> Result<Self, Self::Error> {
        let length = u32::recv(data).unwrap_or(0);
        let mut buffer = vec![0; length as usize];
        data.read_exact(&mut buffer)?;
        Ok(String::from_utf8(buffer).unwrap())
    }
}

impl<T> Sendable for Option<T>
where
    T: Sendable,
{
    type Error = T::Error;
    fn header(&self) -> PacketHeader<Self> {
        match self {
            Some(value) => unsafe { PacketHeader::new(value.size() + 1) },
            None => unsafe { PacketHeader::new(1) },
        }
    }

    fn size_const() -> bool {
        T::size_const()
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

    fn recv(data: &mut dyn Read) -> Result<Self, Self::Error> {
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
    type Error = T::Error;
    fn header(&self) -> PacketHeader<Self> {
        unsafe { PacketHeader::new(self.size()) }
    }

    fn size_const() -> bool {
        T::size_const()
    }

    fn size(&self) -> u32 {
        T::size(&**self)
    }

    fn send(&self) -> Vec<u8> {
        T::send(&**self)
    }

    fn recv(data: &mut dyn Read) -> Result<Self, Self::Error> {
        Ok(Box::new(T::recv(data)?))
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
}
