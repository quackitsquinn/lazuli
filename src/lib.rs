use std::{
    any,
    hash::{DefaultHasher, Hash, Hasher},
};

mod header;

/// Hashes the type_id of T.
#[inline]
fn hash_type_id<T: 'static>() -> u32 {
    let mut hasher = DefaultHasher::new();
    any::TypeId::of::<T>().hash(&mut hasher);
    hasher.finish() as u32
}
