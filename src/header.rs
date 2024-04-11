use std::{
    any::{self, Any},
    hash::{DefaultHasher, Hash, Hasher},
    mem,
};

use crate::hash_type_id;

const HEADER: [u8; 5] = *b"RSOCK";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)] // This is important for the safety of the from_bytes_unchecked function.
/// The header of a packet. When a packet is sent over a socket, it is prepended with this header.
pub struct PacketHeader {
    header: [u8; 5],
    checksum: u32,
    payload_size: u32,
    type_id: u32,
}

impl PacketHeader {
    /// Creates a new PacketHeader with the given payload_size and type_id.
    /// # Safety
    /// The caller must ensure that the payload_size is the same as the size of the payload, and that the type_id is the same as the type_id of the payload.
    pub unsafe fn new(payload_size: u32, type_id: u32) -> PacketHeader {
        PacketHeader {
            header: HEADER,
            checksum: 0,
            payload_size,
            type_id,
        }
    }
    /// Creates a new PacketHeader with the type_id of T and the payload_size of T.
    pub fn auto<T: 'static>() -> PacketHeader {
        PacketHeader {
            header: HEADER,
            checksum: 0,
            payload_size: std::mem::size_of::<T>() as u32,
            type_id: hash_type_id::<T>(),
        }
    }
    /// Calculates the checksum of the payload. Sets the checksum field to the calculated checksum.
    pub fn calculate_checksum(&mut self, payload: &[u8]) {
        let mut hasher = DefaultHasher::new();
        hasher.write(payload);
        self.checksum = hasher.finish() as u32;
    }
    /// Verifies the checksum of the payload.
    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        let mut hasher = DefaultHasher::new();
        hasher.write(payload);
        self.checksum == hasher.finish() as u32
    }

    /// Creates a new PacketHeader from a byte array.
    /// # Safety
    /// This function is unsafe because it creates a PacketHeader from a byte array without checking the checksum.
    /// Use `PacketHeader::from_bytes` if you want to check the checksum.
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> PacketHeader {
        assert!(bytes.len() == mem::size_of::<PacketHeader>());
        assert!(bytes.starts_with(&HEADER));
        // Safety: We just checked that the length of bytes is the same as the size of PacketHeader
        // and that it starts with the HEADER.
        let tmut: PacketHeader = unsafe { *(bytes.as_ptr() as *const PacketHeader) };
        tmut
    }
    /// Creates a new PacketHeader from a byte array.
    pub fn from_bytes(bytes: &[u8], data: &[u8]) -> Option<PacketHeader> {
        let mut header = unsafe { PacketHeader::from_bytes_unchecked(bytes) };
        assert_eq!(header.payload_size as usize, data.len());
        if header.verify_checksum(data)
            && bytes.len() == mem::size_of::<PacketHeader>()
            && bytes.starts_with(&HEADER)
        {
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
        let mut header = unsafe { PacketHeader::new(10, 10) };
        let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        header.calculate_checksum(&data);
        let bytes = unsafe {
            std::mem::transmute::<PacketHeader, [u8; mem::size_of::<PacketHeader>()]>(header)
        };
        let new_header = PacketHeader::from_bytes(&bytes, &data).unwrap();
        assert_eq!(header, new_header);
    }

    #[test]
    fn test_new_auto() {
        let header = PacketHeader::auto::<u32>();
        assert_eq!(header.payload_size, 4);
        assert_eq!(header.type_id, hash_type_id::<u32>());
    }
}
