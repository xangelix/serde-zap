# serde-zap

[![Crates.io](https://img.shields.io/crates/v/serde-zap)](https://crates.io/crates/serde-zap)
[![Docs.rs](https://docs.rs/serde-zap/badge.svg)](https://docs.rs/serde-zap)

A fast binary serialization format for [serde](https://serde.rs), built to be the quickest way to turn Rust values into bytes and back... on serde.

It combines some of the best ideas out there in other crates, and a few extras:

- **Tagged-prefix varints**: decoding is a single branch on the first byte plus one bulk unaligned read — no per-byte LEB128 loop.
- **Raw-pointer cursor I/O**: one pointer compare per read/write, zero slice-index bounds checks.
- **Reserve/commit writer**: varints and short strings are written *directly* into the output buffer with a single capacity check — no encode-to-stack-buffer-then-copy, no two write calls per varint.
- **Zero-copy borrowed deserialization**: `&str` and `&[u8]` fields point straight into the input.
- **A 1-byte error type**: niche-optimized, so `Result<T, E>` stays cheap to return through every hot path.
- **Panic-free by construction**: every fallible operation returns `Result`; the crate forbids `unwrap`/`expect`/`panic`/slice-indexing via lint denies.

`#![no_std]` by default, with an `alloc` feature for `Vec`-backed APIs.

## Performance

serde-zap is built to be the fastest serde trait compatible binary serializer and deserializer, and it gets there by construction rather than by tuning: single-branch varint decoding, one capacity check per written field, zero-copy borrowed decoding, and bulk-copy paths for strings and byte buffers.

Comparative benchmarks against other serde formats will be published here once serde-zap is included upstream in the [rust_serialization_benchmark](https://github.com/djkoloski/rust_serialization_benchmark) harness. Until then, see the design notes above for *why* it is fast, and the `full_vec` / `pod_vec` adapters below for the two places where serde's own machinery would otherwise hold it back.

## Usage

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Reading {
    sensor_id: u32,
    value: f64,
    label: String,
}

let reading = Reading { sensor_id: 7, value: 21.5, label: "ok".into() };

// Serialize into a Vec (two-pass: exact-size single allocation).
let bytes = serde_zap::to_vec(&reading).unwrap();

// Deserialize (borrowing &str/&[u8] zero-copy where the type allows).
let back: Reading = serde_zap::from_bytes(&bytes).unwrap();
assert_eq!(back, reading);
```

### Without allocation (`no_std`, embedded)

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Reading {
    sensor_id: u32,
}

let reading = Reading { sensor_id: 7 };
let mut buf = [0u8; 64];
let written = serde_zap::to_slice(&reading, &mut buf).unwrap();

let back: Reading = serde_zap::from_bytes(written).unwrap();
assert_eq!(back, reading);
```

### Borrowed deserialization

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Record<'a> {
    name: &'a str,
    tags: Vec<&'a str>,
}

let record = Record { name: "zap", tags: vec!["fast", "small"] };
let bytes = serde_zap::to_vec(&record).unwrap();

// Zero-copy: `name` and `tags` point into `bytes`.
let back: Record<'_> = serde_zap::from_bytes(&bytes).unwrap();
assert_eq!(back, record);
```

### `full_vec`: full-length preallocation for big vectors

Serde's stock `Vec` deserializer caps its preallocation and grows by doubling, which costs several reallocations and megabytes of memcpy on large vectors. This field adapter trusts the length prefix and allocates it in full. Wire output is byte-identical to the stock encoding.

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Mesh {
    #[serde(with = "serde_zap::full_vec")]
    triangles: Vec<f32>,
}
```

### `pod_vec`: bulk memcpy for plain-old-data vectors

For `Vec<T>` of plain-old-data structs, this adapter writes the whole vector as one bulk byte copy on both serialize and deserialize (the `serde_bytes` convention: byte-length prefix + raw bytes).

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Copy)]
#[repr(C)]
struct Point { x: f32, y: f32, z: f32 }

// SAFETY: repr(C), no padding, every bit pattern is a valid Point.
unsafe impl serde_zap::pod_vec::Pod for Point {}

#[derive(Serialize, Deserialize)]
struct Cloud {
    #[serde(with = "serde_zap::pod_vec")]
    points: Vec<Point>,
}
```

## When serde-zap is the right choice

- You want the **fastest serde-compatible** binary serialization.
- `no_std` / embedded: `to_slice` and `from_bytes` work with zero allocation.
- You deserialize large payloads and can borrow: zero-copy `&str`/`&[u8]`.
- Big flat vectors of numbers/POD structs: `pod_vec` turns ser/de into memcpy.

## When serde-zap is **not** the right choice

- **Self-describing data**: the format carries no field names or types. You must know the type to decode. `deserialize_any` is unsupported. If you need schema-free decoding, use JSON/CBOR/etc.
- **Schema evolution**: structs are encoded as bare field sequences. Adding, removing, or reordering fields breaks compatibility with existing data. Use a self-describing format or version your payloads explicitly.
- **Long-term storage / interoperability**: this is a young crate; the wire format is not yet declared stable. Pin your version and re-encode on upgrades until 1.0.
- **Streaming I/O**: the API is slice-based (`to_vec`, `to_slice`, `from_bytes`, `take_from_bytes`). There are no `io::Read`/`Write` flavors yet — frame messages yourself (e.g. length-prefix each `to_vec` output).
- **Fully untrusted input for opt-ins**: `full_vec`/`pod_vec` trust length prefixes by design, don't opt into them for attacker-controlled data, and prefer size-limiting input you don't trust, as with any binary format.

## Wire format

Non-self-describing; fields are written in declaration order with no names, no preamble, no versioning.

| type | encoding |
|---|---|
| `u8`, `i8`, `bool` | 1 raw byte |
| `u16`-`u128`, `usize` | tagged varint (below), zigzag first for signed |
| `f32`, `f64` | fixed little-endian 4/8 bytes |
| `char` | varint `u32` code point |
| `Option` | 1 tag byte + payload |
| enum | varint `u32` variant index + payload |
| `str`, `bytes` | varint `u64` length + raw bytes |
| seq, map | varint `u64` element count + elements |
| struct, tuple, array | bare field concatenation (array length is in the type) |
| unit | zero bytes |

Tagged varint (bincode's varint scheme, *not* LEB128): values `<= 250` take one byte; larger values take a tag byte (`251`/`252`/`253`/`254`) followed by a little-endian `u16`/`u32`/`u64`/`u128`. Decoding is one branch + one fixed-width unaligned read.

## Correctness & robustness

- **Fuzzed**: ~20M libFuzzer executions across three targets (arbitrary-input deserialization, structured roundtrip invariants, truncation/mutation of valid buffers) with zero crashes. Run them yourself with `cd fuzz && cargo fuzz run from_bytes` (also `roundtrip`, `truncate`).
- **Miri-clean**: the full test suite runs under Miri with no UB reported.
- **`DoS` guards**: every read is bounds-checked; sequence length hints are refused when they exceed the remaining input bytes, so a bogus length prefix cannot force a huge allocation.
- **Lint-hardened**: `deny(unwrap_used, expect_used, panic, indexing_slicing)` plus pedantic/nursery warnings — no hidden panic paths on hot code.

## Acknowledgments

Contains and combines some strategies from both `postcard` and `bincode`, with a bit of extra magic.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
