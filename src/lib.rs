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
    // use get_mut to stay panic-free
    let written = buf.get_mut(..n).ok_or(Error::ExceedsBuffer)?;
    Ok(written)
}

/// Serializes `value` into a freshly allocated `Vec<u8>`.
///
/// Single-pass: writes into the spare capacity of a `Vec` that grows
/// amortized, then `shrink_to_fit`s, so the result has `len == capacity`
/// with no allocation slack. This is the fast default: one traversal of
/// `value`. Its peak memory during the call is the final size plus the
/// amortized-doubling growth slack (up to 2× the final size in *virtual*
/// address space).
///
/// If peak memory matters more than one extra traversal, see
/// [`to_vec_two_pass`], which produces the exact same output and final
/// allocation.
///
/// # Errors
/// Returns [`Error::SeqLengthUnknown`] if a sequence is serialized without a
/// known length. Allocation failures abort, like any `Vec` operation.
#[cfg(feature = "alloc")]
pub fn to_vec<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>> {
    let mut vec = Vec::new();
    let mut w = write::VecWriter::new(&mut vec);
    value.serialize(ser::Ser::new(&mut w))?;
    vec.shrink_to_fit();
    Ok(vec)
}

/// Serializes `value` into a freshly allocated `Vec<u8>`, computing the
/// exact size first.
///
/// Two-pass: walks `value` once with a counting writer (no allocation),
/// then allocates exactly that many bytes and serializes into it. The
/// output is byte-identical to [`to_vec`] and likewise has
/// `len == capacity` — the *final* allocation is the same either way.
///
/// The trade is ~2× the CPU of [`to_vec`] (two full traversals) in
/// exchange for the lowest possible peak memory during the call: peak ≈
/// 1× the output size, instead of up to ~2× in virtual address space (or
/// ~3× transiently, on allocators whose `realloc` copies instead of
/// extending in place). How much that buys you is platform-dependent: on
/// glibc/Linux, where large `Vec` growth extends via `mremap` and
/// untouched pages never become resident, the measured peak-resident
/// difference is under 5%; on copying allocators (e.g. musl) or under
/// hard memory ceilings, the reduction can be significant. For
/// zero-allocation serialization instead, see [`to_slice`].
///
/// # Errors
/// Returns [`Error::SeqLengthUnknown`] if a sequence is serialized without a
/// known length. Allocation failures abort, like any `Vec` operation.
#[cfg(feature = "alloc")]
pub fn to_vec_two_pass<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>> {
    let size = serialized_size(value)?;
    let mut vec = Vec::with_capacity(size);
    let mut w = write::VecWriter::new(&mut vec);
    value.serialize(ser::Ser::new(&mut w))?;
    debug_assert_eq!(vec.len(), size);
    debug_assert_eq!(vec.len(), vec.capacity());
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
