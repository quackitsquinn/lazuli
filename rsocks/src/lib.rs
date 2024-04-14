#![allow(dead_code)] // TODO: Remove when codebase is more mature
use std::{
    any,
    hash::{DefaultHasher, Hash, Hasher},
};

mod header;
mod sendable;

/// Hashes the type_id of T.
#[inline]
fn hash_type_id<T: 'static>() -> u32 {
    let mut hasher = DefaultHasher::new();
    any::TypeId::of::<T>().hash(&mut hasher);
    hasher.finish() as u32
}
// Re-export for proc-macro
#[doc(hidden)]
pub use static_assertions as __sa;

pub use header::*;
pub use sendable::Sendable;
