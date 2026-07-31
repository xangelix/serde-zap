#![no_main]

//! Roundtrip invariants over arbitrary structured values:
//! * `to_vec` length == `serialized_size`
//! * `to_slice` into an exact-fit buffer produces identical bytes
//! * re-encoding the decoded value yields byte-identical output (bit-exact
//!   roundtrip; robust against NaN payloads)
//! * `take_from_bytes` leaves no remainder
//!
//! These pin the Writer unsafe paths (reserve/commit accounting on
//! SliceWriter, VecWriter, and SizeWriter).

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

mod fuzz_types;
use fuzz_types::*;

fn check<T>(value: &T)
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    let buf = serde_zap::to_vec(value).unwrap();
    assert_eq!(
        buf.len(),
        serde_zap::serialized_size(value).unwrap(),
        "serialized_size mismatch"
    );

    let mut sbuf = vec![0u8; buf.len()];
    let written = serde_zap::to_slice(value, &mut sbuf).unwrap();
    assert_eq!(written, buf.as_slice(), "to_slice/to_vec mismatch");

    // Exact-fit minus one must fail, not overflow.
    if !buf.is_empty() {
        let mut small = vec![0u8; buf.len() - 1];
        assert!(serde_zap::to_slice(value, &mut small).is_err());
    }

    let back: T = serde_zap::from_bytes(&buf).unwrap();
    let buf2 = serde_zap::to_vec(&back).unwrap();
    assert_eq!(buf2, buf, "re-encode mismatch (roundtrip not bit-exact)");

    let (_back2, rest) = serde_zap::take_from_bytes::<T>(&buf).unwrap();
    assert!(rest.is_empty(), "take_from_bytes left trailing bytes");
}

fuzz_target!(|values: (Outer, FullVecW, PodVecW)| {
    check(&values.0);
    check(&values.1);
    check(&values.2);
});
