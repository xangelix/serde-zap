//! Integration tests for the `std` stream adapters (`to_writer` / `from_reader`).
//!
//! The core invariants:
//! - `to_writer` produces byte-identical output to `to_vec` for any value,
//!   across the 8 KiB staging-buffer and 4 KiB write-through boundaries.
//! - Small writes are coalesced (unbuffered writers stay fast).
//! - Underlying I/O errors come back verbatim; serde_zap errors become
//!   `ErrorKind::InvalidData`.

use std::io::{self, Cursor, Read, Write};

use serde::{Deserialize, Serialize};

#[test]
fn byte_identity_across_stage_boundaries() {
    for len in [
        0usize, 1, 255, 256, 4095, 4096, 4097, 8191, 8192, 8193, 16384, 20000,
    ] {
        let s = "x".repeat(len);
        let expected = serde_zap::to_vec(&s).unwrap();
        let mut out = Vec::new();
        serde_zap::to_writer(&s, &mut out).unwrap();
        assert_eq!(out, expected, "to_writer != to_vec for string len {len}");
        let back: String = serde_zap::from_reader(&mut Cursor::new(&out)).unwrap();
        assert_eq!(back, s);

        let bytes = vec![0xABu8; len];
        let expected = serde_zap::to_vec(&bytes).unwrap();
        let mut out = Vec::new();
        serde_zap::to_writer(&bytes, &mut out).unwrap();
        assert_eq!(out, expected, "to_writer != to_vec for bytes len {len}");
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Mixed {
    smalls: Vec<u64>,
    big: String,
    tail: Vec<u64>,
}

/// Stage content must be flushed *before* a write-through bulk write, and
/// varint writes must resume correctly afterwards.
#[test]
fn stage_flush_ordering() {
    let m = Mixed {
        smalls: (0..1000u64).collect(),
        big: "b".repeat(9000),
        tail: (0..1000u64).collect(),
    };
    let expected = serde_zap::to_vec(&m).unwrap();
    let mut out = Vec::new();
    serde_zap::to_writer(&m, &mut out).unwrap();
    assert_eq!(out, expected);
    let back: Mixed = serde_zap::from_reader(&mut Cursor::new(&out)).unwrap();
    assert_eq!(back, m);
}

struct CountingWriter {
    buf: Vec<u8>,
    calls: usize,
}

impl Write for CountingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        self.calls += 1;
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The whole point of the staging buffer: ~20 KB of varints must not become
/// 10k separate `write` calls on an unbuffered writer.
#[test]
fn staging_coalesces_small_writes() {
    let v: Vec<u64> = (0..10_000).collect();
    let mut w = CountingWriter {
        buf: Vec::new(),
        calls: 0,
    };
    serde_zap::to_writer(&v, &mut w).unwrap();
    assert_eq!(w.buf, serde_zap::to_vec(&v).unwrap());
    assert!(w.calls < 50, "expected coalesced writes, got {}", w.calls);
}

struct FailingWriter {
    fail_after: usize,
    written: usize,
}

impl Write for FailingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.written >= self.fail_after {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "boom"));
        }
        self.written += buf.len();
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// I/O errors surface verbatim (kind and message), not as InvalidData —
/// both when they happen mid-stream and at the final flush.
#[test]
fn io_error_returned_verbatim() {
    // Mid-stream failure (value larger than the stage).
    let v: Vec<u64> = (0..10_000).collect();
    let mut w = FailingWriter {
        fail_after: 0,
        written: 0,
    };
    let err = serde_zap::to_writer(&v, &mut w).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(err.to_string(), "boom");

    // Failure at the final flush (value fits in the stage).
    let mut w = FailingWriter {
        fail_after: 0,
        written: 0,
    };
    let err = serde_zap::to_writer(&42u64, &mut w).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(err.to_string(), "boom");

    // A zero-byte value performs no writes, so nothing can fail.
    let mut w = FailingWriter {
        fail_after: 0,
        written: 0,
    };
    serde_zap::to_writer(&(), &mut w).unwrap();
}

struct Lengthless;

impl Serialize for Lengthless {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&1u32)?;
        seq.end()
    }
}

/// A serde_zap-side serialization failure becomes InvalidData wrapping the
/// original error.
#[test]
fn serde_error_becomes_invalid_data() {
    let mut out = Vec::new();
    let err = serde_zap::to_writer(&Lengthless, &mut out).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    assert!(
        err.to_string().contains("length"),
        "unexpected message: {err}"
    );
}

#[test]
fn from_reader_truncated_is_invalid_data() {
    let buf = serde_zap::to_vec(&vec![1u64, 2, 3]).unwrap();
    let err = serde_zap::from_reader::<Vec<u64>, _>(&mut Cursor::new(&buf[..2])).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

/// Trailing bytes are ignored, matching `from_bytes`.
#[test]
fn from_reader_ignores_trailing_bytes() {
    let mut buf = serde_zap::to_vec(&vec![1u64, 2, 3]).unwrap();
    buf.extend_from_slice(&[9, 9, 9]);
    let v: Vec<u64> = serde_zap::from_reader(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::ConnectionReset, "net down"))
    }
}

#[test]
fn read_error_returned_verbatim() {
    let err = serde_zap::from_reader::<u64, _>(&mut FailingReader).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::ConnectionReset);
    assert_eq!(err.to_string(), "net down");
}

#[test]
fn unit_roundtrip_zero_bytes() {
    let mut out = Vec::new();
    serde_zap::to_writer(&(), &mut out).unwrap();
    assert!(out.is_empty());
    let back: () = serde_zap::from_reader(&mut Cursor::new(&out)).unwrap();
    assert_eq!(back, ());
}

/// Roundtrip a struct representative of real workloads through a pipe-like
/// unbuffered writer and reader.
#[test]
fn end_to_end_roundtrip() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Record {
        id: u64,
        name: String,
        tags: Vec<String>,
        scores: Vec<f32>,
        active: bool,
        note: Option<String>,
    }
    let records: Vec<Record> = (0..500u64)
        .map(|i| Record {
            id: i * 999983,
            name: format!("user-{i}"),
            tags: vec![format!("t{}", i % 7), format!("g{}", i % 13)],
            scores: vec![i as f32; (i % 5) as usize],
            active: i % 2 == 0,
            note: (i % 3 == 0).then(|| format!("note-{i}")),
        })
        .collect();
    let mut out = Vec::new();
    serde_zap::to_writer(&records, &mut out).unwrap();
    assert_eq!(out, serde_zap::to_vec(&records).unwrap());
    let back: Vec<Record> = serde_zap::from_reader(&mut Cursor::new(&out)).unwrap();
    assert_eq!(back, records);
}
