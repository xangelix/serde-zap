// -- Clippy Denies --
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![deny(clippy::indexing_slicing)]
// --- Clippy Lint Groups & Specific Warnings ---
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
#![warn(clippy::needless_return)]
// --- Allowed Lints (Overrides) ---
#![allow(clippy::mod_module_files)]
#![allow(clippy::inline_always)]
#![doc = include_str!("../README.md")]
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

mod de;
mod error;
mod read;
mod ser;
mod varint;
mod write;

#[cfg(feature = "alloc")]
pub mod full_vec;
#[cfg(feature = "alloc")]
pub mod pod_vec;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

pub use error::{Error, Result};

/// Serializes `value` into `buf`, returning the written portion.
///
/// # Errors
/// Returns [`Error::ExceedsBuffer`] if `buf` is too small, or
/// [`Error::SeqLengthUnknown`] if a sequence is serialized without a known
/// length.
pub fn to_slice<'a, T: Serialize + ?Sized>(value: &T, buf: &'a mut [u8]) -> Result<&'a mut [u8]> {
    let n = {
        let mut w = write::SliceWriter::new(&mut *buf);
        value.serialize(ser::Ser::new(&mut w))?;
        w.written()
    };
    // n <= buf.len() by construction (SliceWriter bounds every write), but
    // use get_mut to stay panic-free by lint policy.
    let written = buf.get_mut(..n).ok_or(Error::ExceedsBuffer)?;
    Ok(written)
}

/// Serializes `value` into a freshly allocated `Vec<u8>`.
///
/// Two-pass: computes the exact size first, then writes into the spare
/// capacity of a single exact-sized allocation.
///
/// # Errors
/// Returns [`Error::SeqLengthUnknown`] if a sequence is serialized without a
/// known length. Allocation failures abort, like any `Vec` operation.
#[cfg(feature = "alloc")]
pub fn to_vec<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>> {
    let size = serialized_size(value)?;
    let mut vec = Vec::with_capacity(size);
    let mut w = write::VecWriter::new(&mut vec);
    value.serialize(ser::Ser::new(&mut w))?;
    debug_assert_eq!(vec.len(), size);
    Ok(vec)
}

/// Computes the exact serialized size of `value` in bytes.
///
/// # Errors
/// Returns [`Error::SeqLengthUnknown`] if a sequence is serialized without a
/// known length.
pub fn serialized_size<T: Serialize + ?Sized>(value: &T) -> Result<usize> {
    let mut w = write::SizeWriter::new();
    value.serialize(ser::Ser::new(&mut w))?;
    Ok(w.size)
}

/// Deserializes a `T` from the beginning of `buf`.
///
/// `&str` and `&[u8]` fields are borrowed zero-copy from `buf`. Trailing
/// bytes are ignored.
///
/// # Errors
/// Returns an [`Error`] if the input is truncated or malformed (invalid
/// varint tag, bool/option tag, UTF-8, char value, or out-of-range length).
pub fn from_bytes<'de, T: Deserialize<'de>>(buf: &'de [u8]) -> Result<T> {
    let mut d = de::Deserializer::new(buf);
    T::deserialize(&mut d)
}

/// Deserializes a `T` from the beginning of `buf`, returning the value and
/// the unused remainder of the input.
///
/// # Errors
/// Returns an [`Error`] if the input is truncated or malformed (invalid
/// varint tag, bool/option tag, UTF-8, char value, or out-of-range length).
pub fn take_from_bytes<'de, T: Deserialize<'de>>(buf: &'de [u8]) -> Result<(T, &'de [u8])> {
    let mut d = de::Deserializer::new(buf);
    let t = T::deserialize(&mut d)?;
    let rest = d.r.read_slice(d.r.remaining())?;
    Ok((t, rest))
}
