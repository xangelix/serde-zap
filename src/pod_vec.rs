//! `#[serde(with = "serde_zap::pod_vec")]` — bulk-copy adapter for `Vec<T>`
//! fields of plain-old-data structs.
//!
//! Serializing a `Vec<Triangle>`-style Vec element by element costs one serde
//! call per scalar field. When `T` is plain old data, the whole vector is
//! just its bytes: this adapter writes a byte-length prefix followed by one
//! bulk copy of the element bytes (the `serde_bytes` convention), and
//! deserializes with a single allocation plus one bulk copy. On little-endian
//! hosts the element bytes match the stock per-element encoding; on
//! big-endian hosts it falls back to per-element encoding.
//!
//! Note: the length prefix is a *byte* count, unlike the stock `Vec`
//! element-count prefix — data written with this adapter must be read with
//! this adapter.
//!
//! ```rust,ignore
//! #[derive(serde::Serialize, serde::Deserialize)]
//! struct Mesh {
//!     #[serde(with = "serde_zap::pod_vec")]
//!     triangles: Vec<Triangle>,
//! }
//!
//! // SAFETY: repr(C), no padding, all bit patterns valid.
//! unsafe impl serde_zap::Pod for Triangle {}
//! ```
//!
//! # Safety note
//!
//! Like [`crate::full_vec`], deserialization trusts the length prefix
//! (bounded by the remaining input size). Prefer the stock `Vec`
//! implementation for untrusted input.

use alloc::vec::Vec;
use core::fmt;
use core::marker::PhantomData;

use serde::de::{Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};

/// Marks a type as safe to (de)serialize as raw bytes.
///
/// # Safety
///
/// Implementors must satisfy all of the following:
/// * `repr(C)` (or a single field), so in-memory field order matches
///   declaration order — the stock encoding writes fields in order.
/// * No padding bytes (all bytes of the value are initialized).
/// * Every bit pattern of `size_of::<T>()` bytes is a valid `T` (integers,
///   floats, and structs/arrays of those; never bools, chars, enums with
///   invalid discriminants, references, or pointers).
/// * No interior mutability, no ownership semantics (`Copy`).
pub unsafe trait Pod: Copy + 'static {}

macro_rules! impl_pod {
    ($($t:ty),*) => {
        $(unsafe impl Pod for $t {})*
    };
}

impl_pod!(u8, u16, u32, u64, u128, usize);
impl_pod!(i8, i16, i32, i64, i128, isize);
impl_pod!(f32, f64);

// SAFETY: arrays of Pod have no padding and every bit pattern is valid.
unsafe impl<T: Pod, const N: usize> Pod for [T; N] {}

/// Views a Pod slice as its raw bytes (little-endian hosts only, where the
/// wire format matches memory layout).
#[cfg(target_endian = "little")]
#[inline]
const fn as_bytes<T: Pod>(slice: &[T]) -> &[u8] {
    // SAFETY: Pod guarantees no padding, so every byte is initialized, and
    // u8 is valid for any byte content.
    unsafe {
        core::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), core::mem::size_of_val(slice))
    }
}

/// Serializes as a length prefix plus one bulk copy of the element bytes
/// (per-element on big-endian hosts; identical bytes either way).
///
/// # Errors
/// Returns the serializer's error if writing fails.
#[inline]
pub fn serialize<T, S>(value: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
    T: Pod + Serialize,
    S: Serializer,
{
    #[cfg(target_endian = "little")]
    {
        serializer.serialize_bytes(as_bytes(value))
    }
    #[cfg(not(target_endian = "little"))]
    {
        serializer.collect_seq(value)
    }
}

/// Deserializes a `Vec<T>` with a single exact allocation and one bulk copy.
///
/// # Errors
/// Returns an error if the input ends early, if the byte length is not a
/// multiple of `size_of::<T>()`, or if the deserializer fails.
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: Pod + Deserialize<'de>,
    D: Deserializer<'de>,
{
    #[cfg(target_endian = "little")]
    {
        struct PodVecVisitor<T>(PhantomData<T>);

        impl<T: Pod> Visitor<'_> for PodVecVisitor<T> {
            type Value = Vec<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a byte buffer")
            }

            #[inline]
            fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<Vec<T>, E> {
                let elem = size_of::<T>();
                if elem == 0 || !bytes.len().is_multiple_of(elem) {
                    return Err(E::custom("byte length is not a multiple of element size"));
                }
                let len = bytes.len() / elem;
                let mut values: Vec<T> = Vec::with_capacity(len);
                // SAFETY: Pod guarantees every bit pattern is a valid T and
                // T: Copy, so copying the bytes into the Vec's spare capacity
                // produces `len` initialized elements. u8 and T have no
                // aliasing concerns here (fresh allocation, single copy).
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        values.as_mut_ptr().cast::<u8>(),
                        bytes.len(),
                    );
                    values.set_len(len);
                }
                Ok(values)
            }
        }

        deserializer.deserialize_bytes(PodVecVisitor(PhantomData))
    }
    #[cfg(not(target_endian = "little"))]
    {
        crate::full_vec::deserialize(deserializer)
    }
}
