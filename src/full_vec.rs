//! `#[serde(with = "serde_zap::full_vec")]` — opt-in adapter for `Vec` fields.
//!
//! Serde's stock `Vec` deserializer preallocates with a capped size hint
//! (`1MB / size_of::<T>()`) and then grows by doubling, which costs several
//! reallocations and megabytes of memcpy on large vectors. This adapter
//! trusts the encoded length and preallocates it in full.
//!
//! ```rust,ignore
//! #[derive(serde::Serialize, serde::Deserialize)]
//! struct Mesh {
//!     #[serde(with = "serde_zap::full_vec")]
//!     triangles: Vec<Triangle>,
//! }
//! ```
//!
//! # Safety note
//!
//! Because the length prefix is trusted, a corrupt or malicious input can
//! cause a huge allocation up front. Prefer the stock `Vec` implementation
//! for untrusted input (or bound the input size by other means).

use alloc::vec::Vec;
use core::fmt;
use core::marker::PhantomData;

use serde::de::{Deserializer, SeqAccess, Visitor};
use serde::{Deserialize, Serialize, Serializer};

/// Serializes exactly like serde's stock `Vec<T>` impl: length prefix
/// followed by the elements in order.
///
/// # Errors
/// Returns the serializer's error if writing fails (e.g. insufficient output
/// space or an element that fails to serialize).
#[inline]
pub fn serialize<T, S>(value: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    serializer.collect_seq(value)
}

/// Deserializes a `Vec<T>`, preallocating the full encoded length.
///
/// # Errors
/// Returns an error if the input ends before the claimed elements are
/// decoded, or if an element fails to decode.
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    struct FullVecVisitor<T>(PhantomData<T>);

    impl<'de, T: Deserialize<'de>> Visitor<'de> for FullVecVisitor<T> {
        type Value = Vec<T>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a sequence")
        }

        #[inline]
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
            let mut values = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(value) = seq.next_element()? {
                values.push(value);
            }
            Ok(values)
        }
    }

    deserializer.deserialize_seq(FullVecVisitor(PhantomData))
}
