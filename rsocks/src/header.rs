use std::{
    convert::Infallible,
    hash::{DefaultHasher, Hash, Hasher},
    mem,
};

use crate::{hash_type_id, Sendable};

const HEADER: [u8; 5] = *b"RSOCK";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)] // This is important for the safety of the from_bytes_unchecked function.
/// The header of a packet. When a packet is sent over a socket, it is prepended with this header.
/// # Why the type parameter?
/// The type parameter is used to have some sort of type safety.
/// Without the type parameter, it would be possible to create a PacketHeader with a `type_id` that does not match the actual type of the payload.
/// This would lead to undefined behavior.
///
/// The type parameter is used to ensure that the `type_id` matches the actual type of the payload.
/// You can make an untyped PacketHeader by using `PacketHeader<UnknownType>`, but this is only intended for receiving packets.
pub struct PacketHeader<T>
where
    T: 'static + Sendable,
{
    // should always be "RSOCK"
    header: [u8; 5],
    has_checksum: bool,
    checksum: u32,
    pub payload_size: u32,
    type_id: u32,
    // allow for some sort of type safety
    _phantom: std::marker::PhantomData<T>,
}
/// A ZST that represents an unknown type.
/// This is used when the type of the payload is unknown.
#[derive(Clone, Copy)]
pub struct UnknownType;

impl Sendable for UnknownType {
    // literally nothing to send
    type Error = Infallible;

    fn send(&self) -> Vec<u8> {
        Vec::new()
    }

    fn recv(_: &mut dyn std::io::Read) -> Result<Self, Self::Error> {
        Ok(UnknownType)
    }
}

impl<T> PacketHeader<T>
where
    T: 'static + Sendable,
{
    /// Creates a new PacketHeader with the type_id of T and the payload_size of T.
    pub fn auto() -> PacketHeader<T> {
        PacketHeader {
            header: HEADER,
            checksum: 0,
            has_checksum: false,
            payload_size: std::mem::size_of::<T>() as u32,
            type_id: hash_type_id::<T>(),
            _phantom: std::marker::PhantomData,
        }
    }
    /// Creates a new PacketHeader with the specified length of the payload.
    ///
    /// This can be useful for types where the size of the payload is not constant. (e.g. Vec<T>, String, etc.)
    /// This can also be useful for reference types.
    ///
    /// # Safety
    /// The caller must ensure that the payload_size is correct, and that the sendable implementation accounts for the variable size of the payload.
    pub unsafe fn new(payload_size: u32) -> PacketHeader<T> {
        PacketHeader {
            header: HEADER,
            checksum: 0,
            has_checksum: false,
            payload_size,
            type_id: hash_type_id::<T>(),
            _phantom: std::marker::PhantomData,
        }
    }
    /// Calculates the checksum of the payload. Sets the checksum field to the calculated checksum.
    pub fn calculate_checksum(&mut self, payload: &[u8]) {
        let mut hasher = DefaultHasher::new();
        hasher.write(payload);
        self.checksum = hasher.finish() as u32;
        self.has_checksum = true;
    }
    /// Verifies the checksum of the payload.
    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        if !self.has_checksum {
            return true;
        }
        let mut hasher = DefaultHasher::new();
        hasher.write(payload);
        self.checksum == hasher.finish() as u32
    }

    /// Converts the PacketHeader into a byte array.
    pub fn to_bytes(&self) -> [u8; mem::size_of::<PacketHeader<UnknownType>>()] {
        unsafe {
            // SAFETY: We know that PacketHeader<T> is the same size as PacketHeader<UnknownType>
            let bytes = std::mem::transmute_copy::<
                PacketHeader<T>,
                [u8; mem::size_of::<PacketHeader<UnknownType>>()],
            >(self);
            bytes
        }
    }
}

impl PacketHeader<UnknownType> {
    /// Converts the PacketHeader into a PacketHeader with a different type.
    /// # Safety
    /// The caller must ensure that the type_id and payload_size are correct.
    /// The caller must also ensure that the type T is the correct type.
    pub unsafe fn into_ty<U: Copy + Sendable>(self) -> PacketHeader<U> {
        assert_eq!(self.payload_size, std::mem::size_of::<U>() as u32);
        assert_eq!(self.type_id, hash_type_id::<U>());

        PacketHeader {
            header: self.header,
            checksum: self.checksum,
            has_checksum: self.has_checksum,
            payload_size: self.payload_size,
            type_id: self.type_id,
            _phantom: std::marker::PhantomData,
        }
    }
    /// Creates a new PacketHeader from a byte array.
    /// # Safety
    /// This function is unsafe because it creates a PacketHeader from a byte array without checking the checksum.
    /// Use `PacketHeader::from_bytes` if you want to check the checksum.
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> PacketHeader<UnknownType> {
        assert!(
            bytes.len() == mem::size_of::<PacketHeader<UnknownType>>(),
            "bytes.len() = {}",
            bytes.len()
        );
        assert!(
            bytes.starts_with(&HEADER),
            "Header is not correct (Expected: {:?}, Got: {:?})",
            HEADER,
            &bytes[..5]
        );
        // Safety: We just checked that the length of bytes is the same as the size of PacketHeader
        // and that it starts with the HEADER.
        unsafe { *(bytes.as_ptr() as *const PacketHeader<UnknownType>) }
    }
    /// Creates a new PacketHeader from a byte array.
    pub fn from_bytes(bytes: &[u8], data: &[u8]) -> Option<PacketHeader<UnknownType>> {
        let header: PacketHeader<UnknownType> =
            unsafe { PacketHeader::<UnknownType>::from_bytes_unchecked(bytes) };
        assert_eq!(header.payload_size as usize, data.len());
        let checksum_ok: bool = header.verify_checksum(data);
        let len_ok: bool = bytes.len() == mem::size_of::<PacketHeader<UnknownType>>();
        let header_ok: bool = bytes.starts_with(&HEADER);
        if checksum_ok && len_ok && header_ok {
            Some(header)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::hash_type_id;

    use super::*;

    #[test]
    fn test_packet_header() {
        let mut header: PacketHeader<u128> = PacketHeader::auto();
        let data = 32u128.send();
        header.calculate_checksum(&data);
        let bytes = header.to_bytes();
        let new_header = PacketHeader::from_bytes(&bytes, &data).unwrap();
        let ty_header = unsafe { new_header.into_ty::<u128>() };
        assert_eq!(header, ty_header);
    }

    #[test]
    fn test_new_auto() {
        let header: PacketHeader<u32> = PacketHeader::auto();
        assert_eq!(header.payload_size, 4);
        assert_eq!(header.type_id, hash_type_id::<u32>());
    }
}
