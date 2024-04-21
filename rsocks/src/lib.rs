#![allow(dead_code)] // TODO: Remove when codebase is more mature
use std::{
    any,
    hash::{DefaultHasher, Hash, Hasher},
};

mod client;
mod header;
mod sendable;
mod stream;

pub(crate) type ArcMutex<T> = std::sync::Arc<std::sync::Mutex<T>>;

/// Hashes the type_id of T.
#[inline]
fn hash_type_id<T: 'static>() -> u32 {
    let mut hasher = DefaultHasher::new();
    any::TypeId::of::<T>().hash(&mut hasher);
    hasher.finish() as u32
}

pub use header::*;
pub use sendable::Sendable;
