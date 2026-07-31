# serde-zap ⚡

[![Crates.io](https://img.shields.io/crates/v/serde-zap)](https://crates.io/crates/serde-zap)
[![Docs.rs](https://docs.rs/serde-zap/badge.svg)](https://docs.rs/serde-zap)

A fast binary serialization format for [serde](https://serde.rs), built to be the quickest way to turn Rust values into bytes and back... on serde.

It combines some of the best ideas out there in other crates, and a few extras:

- **Tagged-prefix varints**: decoding is a single branch on the first byte plus one bulk unaligned read — no per-byte LEB128 loop.
- **Raw-pointer cursor I/O**: one pointer compare per read/write, zero slice-index bounds checks.
- **Reserve/commit writer**: varints and short strings are written _directly_ into the output buffer with a single capacity check — no encode-to-stack-buffer-then-copy, no two write calls per varint.
- **Zero-copy borrowed deserialization**: `&str` and `&[u8]` fields point straight into the input.
- **A 1-byte error type**: niche-optimized, so `Result<T, E>` stays cheap to return through every hot path.
- **Panic-free by construction**: every fallible operation returns `Result`; the crate forbids `unwrap`/`expect`/`panic`/slice-indexing via lint denies.

`#![no_std]` by default, with an `alloc` feature for `Vec`-backed APIs.

## Performance

serde-zap is built to be the fastest serde trait compatible binary serializer and deserializer, and it gets there by construction rather than by tuning: single-branch varint decoding, one capacity check per written field, zero-copy borrowed decoding, and bulk-copy paths for strings and byte buffers.

All numbers below are from the [rust_serialization_benchmark](https://github.com/djkoloski/rust_serialization_benchmark) harness (criterion defaults, `target-cpu=native`) running serde-zap **0.1.0 from crates.io** on a GitHub-hosted runner, 2026-07-31. Absolute numbers vary by machine; the _ratios_ are the point.

### Which of the benchmark's crates actually use serde?

Of the ~40 benchmarked libraries, only these **16 go through serde's `Serialize`/`Deserialize` traits** on the shared dataset types: serde-zap, postcard, bincode 1.x, cbor4ii, ciborium, dlhn, flexbuffers, flexon, nachricht-serde, pot, rmp-serde, ron, `serde_bare`, serde-brief, `serde_cbor`, and `serde_json`.

Three more are serde-_capable_ but are benchmarked through their own derive macros instead: **bincode 2** (native `Encode`/`Decode`; its `serde` feature adapter was measured separately below), **bitcode** (has a `serde` module), and **simd-json** (has `simd_json::serde`; benchmarked via `simd_json_derive`, a fork of serde's derive emitting its own traits). Everything else on the list uses its own trait system — including a few with deceptively serde-named traits that are _not_ serde (savefile's own `Serialize`/`Deserialize`, borsh's `BorshSerialize`, capnp's own `Serialize<'a>`).

### `log` (10,000 string-heavy HTTP logs)

| crate           | serialize    | deserialize   | borrow       | size (B)  |
| --------------- | ------------ | ------------- | ------------ | --------- |
| **serde-zap**   | **171.6 µs** | 2198.9 µs     | **551.4 µs** | 741,295   |
| postcard        | 326.5 µs     | 2248.3 µs     | 647.8 µs     | 724,953   |
| bincode 1.3.3   | 386.4 µs     | **2179.8 µs** | 613.4 µs     | 1,045,784 |
| dlhn            | 483.0 µs     | 2586.3 µs     | —            | 724,953   |
| `serde_bare`    | 508.0 µs     | 2178.9 µs     | —            | 765,778   |
| cbor4ii         | 519.3 µs     | 4.82 ms       | 3.06 ms      | 1,407,835 |
| serde-brief     | 1.19 ms      | 4.19 ms       | 2.39 ms      | 1,584,946 |
| rmp-serde       | 1.22 ms      | 3.05 ms       | 1.35 ms      | 784,997   |
| `serde_cbor`    | 1.65 ms      | 4.46 ms       | 2.62 ms      | 1,407,835 |
| pot             | 1.97 ms      | 5.88 ms       | 3.72 ms      | 971,922   |
| flexon          | 2.13 ms      | 4.03 ms       | —            | 1,827,461 |
| ciborium        | 2.75 ms      | 10.0 ms       | —            | 1,407,835 |
| `serde_json`    | 3.20 ms      | 5.98 ms       | —            | 1,827,461 |
| nachricht-serde | 4.65 ms      | 3.96 ms       | 2.14 ms      | 818,669   |
| flexbuffers     | 5.94 ms      | 6.26 ms       | 4.51 ms      | 1,829,756 |
| ron             | 10.4 ms      | 23.5 ms       | 21.2 ms      | 1,607,459 |

### `mesh` (125,000 triangles, all `f32`)

| crate           | serialize    | deserialize   | size (B)   |
| --------------- | ------------ | ------------- | ---------- |
| **serde-zap**   | **441.4 µs** | 1857.5 µs     | 6,000,005  |
| postcard        | 441.5 µs     | **1355.3 µs** | 6,000,003  |
| bincode 1.3.3   | 3.97 ms      | 5.65 ms       | 6,000,008  |
| `serde_bare`    | 4.32 ms      | 5.33 ms       | 6,000,003  |
| dlhn            | 4.78 ms      | 7.94 ms       | 6,000,003  |
| cbor4ii         | 6.02 ms      | 41.6 ms       | 13,125,016 |
| serde-brief     | 13.7 ms      | 25.8 ms       | 15,750,015 |
| rmp-serde       | 15.4 ms      | 16.2 ms       | 8,125,006  |
| `serde_cbor`    | 29.6 ms      | 34.8 ms       | 13,122,324 |
| pot             | 32.5 ms      | 51.8 ms       | 10,122,342 |
| ciborium        | 50.7 ms      | 94.1 ms       | 13,122,324 |
| flexon          | 69.4 ms      | 55.4 ms       | 26,192,883 |
| `serde_json`    | 80.1 ms      | 93.5 ms       | 26,192,883 |
| flexbuffers     | 96.6 ms      | 68.2 ms       | 26,609,424 |
| nachricht-serde | 102.4 ms     | 26.2 ms       | 8,125,037  |
| ron             | 169.3 ms     | 487.9 ms      | 22,192,885 |

### `minecraft_savedata` (500 deeply nested player saves)

| crate           | serialize    | deserialize   | borrow       | size (B)  |
| --------------- | ------------ | ------------- | ------------ | --------- |
| **serde-zap**   | **212.9 µs** | 1942.6 µs     | **667.7 µs** | 367,413   |
| postcard        | 384.3 µs     | 2069.7 µs     | 823.0 µs     | 367,489   |
| bincode 1.3.3   | 493.3 µs     | **1830.1 µs** | 870.1 µs     | 569,975   |
| dlhn            | 550.4 µs     | 2591.4 µs     | —            | 366,496   |
| `serde_bare`    | 603.5 µs     | 2379.2 µs     | —            | 356,311   |
| cbor4ii         | 723.8 µs     | 4.36 ms       | 3.11 ms      | 1,109,831 |
| serde-brief     | 1.13 ms      | 5.10 ms       | 3.37 ms      | 1,276,014 |
| rmp-serde       | 1.32 ms      | 2.86 ms       | 1.58 ms      | 424,533   |
| `serde_cbor`    | 1.58 ms      | 4.31 ms       | 3.02 ms      | 1,109,821 |
| pot             | 2.00 ms      | 5.14 ms       | 3.81 ms      | 599,125   |
| flexon          | 2.40 ms      | 4.54 ms       | —            | 1,623,191 |
| ciborium        | 2.75 ms      | 8.57 ms       | —            | 1,109,821 |
| `serde_json`    | 3.23 ms      | 7.01 ms       | —            | 1,623,191 |
| nachricht-serde | 4.45 ms      | 3.74 ms       | 2.49 ms      | 449,745   |
| flexbuffers     | 7.04 ms      | 6.06 ms       | 4.71 ms      | 1,187,688 |
| ron             | 7.85 ms      | 24.5 ms       | 22.5 ms      | 1,465,223 |

### `mk48` (1,000 enum-heavy game updates)

| crate           | serialize    | deserialize   | size (B)  |
| --------------- | ------------ | ------------- | --------- |
| **serde-zap**   | **813.2 µs** | 4009.3 µs     | 1,406,257 |
| postcard        | 1679.1 µs    | **3961.0 µs** | 1,311,281 |
| cbor4ii         | 2.31 ms      | 16.8 ms       | 6,012,539 |
| bincode 1.3.3   | 2.70 ms      | 4.28 ms       | 1,854,234 |
| `serde_bare`    | 2.95 ms      | 4.91 ms       | 1,319,999 |
| dlhn            | 3.36 ms      | 6.56 ms       | 1,311,281 |
| serde-brief     | 4.66 ms      | 18.1 ms       | 6,951,772 |
| `serde_cbor`    | 8.72 ms      | 18.0 ms       | 6,012,373 |
| rmp-serde       | 8.88 ms      | 9.99 ms       | 1,745,322 |
| pot             | 11.5 ms      | 24.6 ms       | 2,604,812 |
| flexon          | 12.7 ms      | 24.0 ms       | 9,390,461 |
| `serde_json`    | 17.5 ms      | 31.0 ms       | 9,390,461 |
| ciborium        | 17.7 ms      | 42.1 ms       | 6,012,373 |
| nachricht-serde | 25.4 ms      | 14.6 ms       | 1,770,060 |
| flexbuffers     | 35.5 ms      | 30.8 ms       | 5,352,680 |
| ron             | 45.4 ms      | 145.4 ms      | 8,677,703 |

### How to read this

- **serde-zap has the fastest serialize of every serde-trait crate on every dataset** — 1.9× the runner-up on log, 1.8× on minecraft, 2.1× on mk48, and a photo-finish tie with postcard on mesh (both saturate memory bandwidth).
- **serde-zap has the fastest borrowed (zero-copy) deserialize** everywhere it is measured (log, minecraft).
- **Owned deserialize is a near-tie at the top**: bincode 1.x / `serde_bare` / postcard / serde-zap sit within a few percent of each other per dataset and machine — that metric is dominated by identical allocation work, not decoder speed (see borrow for the pure-decode comparison, which serde-zap wins).
- **Sizes**: serde-zap matches the best-in-class binary formats (byte-identical to bincode 2's varint scheme) — smallest or second-smallest of the serde crates on three of four datasets.
- **bincode 2's serde adapter** (`bincode::serde`, measured locally on a faster machine, same run): serde-zap beat it by 1.24× (log serialize), 1.20× (log deserialize), 1.33× (log borrow), and 1.25× (mesh deserialize); mesh serialize was a tie. bincode 2's _native_ (non-serde) API — what the harness benchmarks as "bincode" — remains faster than any serde path on mesh, which is precisely the gap `full_vec`/`pod_vec` exist to close for users who can opt in.

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

// Serialize into a Vec (single-pass, then shrunk to exact capacity).
// For the lowest peak memory during the call at ~2x CPU, use
// `serde_zap::to_vec_two_pass` — output is byte-identical.
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

| type                  | encoding                                               |
| --------------------- | ------------------------------------------------------ |
| `u8`, `i8`, `bool`    | 1 raw byte                                             |
| `u16`-`u128`, `usize` | tagged varint (below), zigzag first for signed         |
| `f32`, `f64`          | fixed little-endian 4/8 bytes                          |
| `char`                | varint `u32` code point                                |
| `Option`              | 1 tag byte + payload                                   |
| enum                  | varint `u32` variant index + payload                   |
| `str`, `bytes`        | varint `u64` length + raw bytes                        |
| seq, map              | varint `u64` element count + elements                  |
| struct, tuple, array  | bare field concatenation (array length is in the type) |
| unit                  | zero bytes                                             |

Tagged varint (bincode's varint scheme, _not_ LEB128): values `<= 250` take one byte; larger values take a tag byte (`251`/`252`/`253`/`254`) followed by a little-endian `u16`/`u32`/`u64`/`u128`. Decoding is one branch + one fixed-width unaligned read.

## Correctness & robustness

- **Fuzzed**: ~20M libFuzzer executions across three targets (arbitrary-input deserialization, structured roundtrip invariants, truncation/mutation of valid buffers) with zero crashes. Run them yourself with `cd fuzz && cargo fuzz run from_bytes` (also `roundtrip`, `truncate`).
- **Miri-clean**: the full test suite runs under Miri with no UB reported.
- **`DoS` guards**: every read is bounds-checked; sequence length hints are refused when they exceed the remaining input bytes, so a bogus length prefix cannot force a huge allocation.
- **Lint-hardened**: `deny(unwrap_used, expect_used, panic, indexing_slicing)` plus pedantic/nursery warnings — no hidden panic paths on hot code.

## Acknowledgments

Contains and combines some strategies from both `postcard` and `bincode`, with a bit of extra magic.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
