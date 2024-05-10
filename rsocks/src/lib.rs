#![allow(dead_code)] // TODO: Remove when codebase is more mature
#![deny(unsafe_op_in_unsafe_fn)]
use std::{
    any,
    hash::{DefaultHasher, Hash, Hasher},
};

mod client;
pub mod header;
mod sendable;
mod stream;

/// An Atomic Reference Counted Mutex. This is used to share data between threads.
// exists because ArcMutex<T> is easier to type than Arc<Mutex<T>>.
pub(crate) type ArcMutex<T> = std::sync::Arc<std::sync::Mutex<T>>;

/// A Result type that returns an io::Error. We use io::Error throughout the library because it is the most fitting error type for this library.
pub type IOResult<T> = std::result::Result<T, std::io::Error>;

/// Configures the logging for testing.
#[cfg(test)]
pub(crate) fn init_logging() {
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Debug,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap_or_else(|_| {
        eprintln!("Failed to initialize logger.");
    })
}

/// Hashes the type_id of T.
// TODO: After some performance testing, determine if this should just be converted into a mem::transmute::<u128>(TypeId::of::<T>) or something similar.
#[inline]
fn hash_type_id<T: 'static>() -> u32 {
    let mut hasher = DefaultHasher::new();
    any::TypeId::of::<T>().hash(&mut hasher);
    hasher.finish() as u32
}

pub use client::TcpClient;
pub(crate) use header::*;
pub use sendable::Sendable;
