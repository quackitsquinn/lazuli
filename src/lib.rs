use core::fmt;
use std::any::{self, Any, TypeId};
use std::borrow::Borrow;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Copy, Clone)]
#[repr(C)]
struct PacketHeader {
    _head: u32,
    packet_type: u64,
    packet_length: u16,
}

impl PacketHeader {
    fn new(ptype: u64, len: u16) -> Self {
        Self {
            _head: u32::from_le_bytes(*b"RSOC"),
            packet_type: ptype,
            packet_length: len,
        }
    }
    fn validate(&self) -> bool {
        self._head == u32::from_le_bytes(*b"RSOC")
    }

    pub fn packet_type(&self) -> u64 {
        self.packet_type
    }

    pub fn packet_length(&self) -> u16 {
        self.packet_length
    }
}

trait Packet {
    fn header(&self) -> PacketHeader;
    fn data(&self) -> &[u8];
}

impl<T> Packet for T
where
    T: 'static,
{
    fn header(&self) -> PacketHeader {
        let size = std::mem::size_of::<T>();
        let hash = typeid_u64::<T>();
        println!("Hash: {:?}", hash);

        PacketHeader::new(hash, size as u16)
    }

    fn data(&self) -> &[u8] {
        unsafe {
            let ptr = self as *const T as *const u8;
            let len = std::mem::size_of::<T>();
            std::slice::from_raw_parts(ptr, len)
        }
    }
}
fn typeid_u64<T: 'static>() -> u64 {
    let mut s = DefaultHasher::new();
    any::TypeId::of::<T>().hash(&mut s);
    s.finish()
}
/// A pool of packets.
struct PacketPool {
    packets: Vec<Box<dyn Packet>>,
}

impl PacketPool {
    fn new() -> Self {
        Self {
            packets: Vec::new(),
        }
    }

    fn add<T>(&mut self, packet: T)
    where
        T: Packet + 'static,
    {
        println!("Adding packet of type: {:?}", packet.header().packet_type());
        println!("Packet length: {:?}", packet.header().packet_length());
        println!("Packet data: {:?}", packet.data());
        self.packets.push(Box::new(packet));
    }

    fn validate(&self) -> bool {
        self.packets.iter().all(|p| p.header().validate())
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        println!("Finding packet of type: {:?}", typeid_u64::<T>());
        self.packets.iter().find_map(|p| {
            println!("{} != {}", p.header().packet_type, typeid_u64::<T>());
            let ptype = (**p).header().packet_type;
            if ptype == typeid_u64::<T>() {
                println!("Found packet of type: {:?}", p.header().packet_type());
                // make sure the size of the packet is the same as the size of the type
                debug_assert!(std::mem::size_of::<T>() == p.header().packet_length as usize);
                debug_assert!(std::mem::size_of::<T>() == p.data().len());
                unsafe { p.data().as_ptr().cast::<T>().as_ref() }
            } else {
                println!("Packet type does not match");
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut pool = PacketPool::new();
        macro_rules! print_type_id {
            ($t:ty) => {
                println!("Type id of {}: {:?}", stringify!($t), typeid_u64::<$t>());
            };
        }

        print_type_id!(u32);
        print_type_id!(u64);
        print_type_id!(u128);
        print_type_id!(&u32);
        print_type_id!(&u64);
        print_type_id!(&u128);
        print_type_id!(&Box<u32>);
        print_type_id!(&Box<u64>);
        print_type_id!(&Box<u128>);
        print_type_id!(&Box<dyn Packet>);
        print_type_id!(Box<dyn Packet>);

        println!("====== Adding u32 ======");
        pool.add(42u32);
        println!("====== Adding u64 ======");
        pool.add(42u64);
        println!("====== Adding u128 ======");
        pool.add(42u128);
        println!("====== Validating ======");
        assert_eq!(pool.validate(), true);
        println!("====== Getting u32 ======");
        assert_eq!(pool.get::<u32>(), Some(&42u32));
        println!("====== Getting u64 ======");
        assert_eq!(pool.get::<u64>(), Some(&42u64));
        println!("====== Getting u128 ======");
        assert_eq!(pool.get::<u128>(), Some(&42u128));
    }
}
