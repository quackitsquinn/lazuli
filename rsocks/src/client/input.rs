//! Module for handling input from a socket. Contains several helper functions for reading data from a socket.

use std::{io::Read, mem, net::TcpStream};

use crate::{PacketHeader, UnknownType};

pub fn input_header(stream: &mut TcpStream) -> Option<PacketHeader<UnknownType>> {
    let mut header = [0; mem::size_of::<PacketHeader<UnknownType>>()];

    if let Ok(_) = stream.read_exact(&mut header) {
        // Safety: We just checked that the length of header is the same as the size of PacketHeader
        // and that it starts with the HEADER.
        return Some(unsafe { PacketHeader::from_bytes_unchecked(&header) });
    }
    None
}
