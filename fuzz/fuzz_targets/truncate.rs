#![no_main]

//! Truncation and mutation of valid buffers:
//! * every strict prefix of a valid encoding must fail cleanly (our format
//!   has no optional trailing data, so a prefix is always incomplete)
//! * mutated buffers may parse or fail, but must never panic
//!
//! This hits the bounds checks in Reader (read_slice, read_array,
//! varint tag dispatch) with near-valid inputs.

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;

mod fuzz_types;
use fuzz_types::Outer;

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);
    let Ok(value) = u.arbitrary::<Outer>() else {
        return;
    };
    let buf = serde_zap::to_vec(&value).unwrap();
    if buf.is_empty() {
        return;
    }

    // Truncate at an input-driven position.
    let cut = u.arbitrary::<usize>().unwrap() % (buf.len() + 1);
    if cut < buf.len() {
        assert!(
            serde_zap::from_bytes::<Outer>(&buf[..cut]).is_err(),
            "strict prefix ({cut} of {} bytes) unexpectedly parsed",
            buf.len()
        );
    }

    // Flip a few input-driven bytes; must not panic either way.
    let mut mutated = buf.clone();
    for _ in 0..4 {
        let idx = u.arbitrary::<usize>().unwrap() % mutated.len();
        let mask = u.arbitrary::<u8>().unwrap();
        mutated[idx] ^= mask;
    }
    let _ = serde_zap::from_bytes::<Outer>(&mutated);
});
