//! Module for handling input from a socket. Contains several helper functions for reading data from a socket.
//! This module also provides functions that return IOResults, which in turn can be used with the ? operator.

use std::{io::Read, mem, net::TcpStream};

use log::trace;

use crate::{header, IOResult, PacketHeader, UnknownType};

/// Reads the header of a packet from a TcpStream.
#[inline]
pub fn input_header(stream: &mut TcpStream) -> IOResult<PacketHeader<UnknownType>> {
    let mut header = [0; mem::size_of::<PacketHeader<UnknownType>>()];

    stream.read_exact(&mut header)?;

    trace!("Read header: {:?}", header);

    let header = unsafe { PacketHeader::from_bytes_unchecked(header.as_slice()) };

    Ok(header)
}

/// Reads the data of a packet from a TcpStream.
/// The header type is UnknownType because this method is intended to be used in tandem with input_header,
/// or any other method that reads from a socket, where the type will be unknown.
#[inline]
pub fn input_data(stream: &mut TcpStream, header: &PacketHeader<UnknownType>) -> IOResult<Vec<u8>> {
    let mut data = vec![0; header.payload_size as usize];

    trace!("Reading {} bytes of data", header.payload_size);

    // We want the slice of the data, otherwise (at least in my testing) it will keep reading forever.
    // This is probably wrong, but it works for now.
    stream.read_exact(&mut data[0..header.payload_size as usize])?;

    trace!("Read data: {:?}", data);

    Ok(data)
}

/// Verifies the checksum of a packet.
///
/// This function is mainly a convenience function for verifying the checksum of a packet.
/// It runs PacketHeader::verify_checksum, but converts a bool to an IOResult.
#[inline]
pub fn verify_checksum(header: &PacketHeader<UnknownType>, data: &[u8]) -> IOResult<()> {
    if header.verify_checksum(data) {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Checksums do not match",
        ))
    }
}
