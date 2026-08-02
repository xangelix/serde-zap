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

All numbers below are the **independently published results** from the [rust_serialization_benchmark](https://github.com/djkoloski/rust_serialization_benchmark) README: serde-zap was merged upstream in [PR #151](https://github.com/djkoloski/rust_serialization_benchmark/pull/151) as **0.1.0 from crates.io**, and the tables are from the [2026-08-02 run](https://github.com/djkoloski/rust_serialization_benchmark/tree/f1de883f94316724798a9d4e5a452839fb4627cb) (criterion defaults on a GitHub-hosted AMD EPYC 7763 runner). Absolute numbers vary by machine; the _ratios_ are the point.

### Which of the benchmark's crates actually use serde?

Of the ~40 benchmarked libraries, only these **16 go through serde's `Serialize`/`Deserialize` traits** on the shared dataset types: serde-zap, postcard, bincode 1.x, cbor4ii, ciborium, dlhn, flexbuffers, flexon, nachricht-serde, pot, rmp-serde, ron, `serde_bare`, serde-brief, `serde_cbor`, and `serde_json`.

Three more are serde-_capable_ but are benchmarked through their own derive macros instead: **bincode 2** (native `Encode`/`Decode`; its `serde` feature adapter was measured separately below), **bitcode** (has a `serde` module), and **simd-json** (has `simd_json::serde`; benchmarked via `simd_json_derive`, a fork of serde's derive emitting its own traits). Everything else on the list uses its own trait system — including a few with deceptively serde-named traits that are _not_ serde (savefile's own `Serialize`/`Deserialize`, borsh's `BorshSerialize`, capnp's own `Serialize<'a>`).

### `log` (10,000 string-heavy HTTP logs)

| crate           | serialize    | deserialize   | borrow       | size (B)  |
| --------------- | ------------ | ------------- | ------------ | --------- |
| **serde-zap**   | **199.2 µs** | 2.171 ms      | **508.6 µs** | 741,295   |
| postcard        | 425.1 µs     | 2.310 ms      | 619.7 µs     | 724,953   |
| bincode 1.3.3   | 525.1 µs     | **2.065 ms**  | 589.1 µs     | 1,045,784 |
| cbor4ii         | 616.9 µs     | 4.956 ms      | 3.440 ms     | 1,407,835 |
| dlhn            | 661.2 µs     | 2.586 ms      | —            | 724,953   |
| `serde_bare`    | 685.6 µs     | 2.089 ms      | —            | 765,778   |
| serde-brief     | 1.32 ms      | 4.575 ms      | 3.004 ms     | 1,584,946 |
| rmp-serde       | 1.54 ms      | 3.165 ms      | 1.429 ms     | 784,997   |
| `serde_cbor`    | 2.03 ms      | 4.674 ms      | 3.260 ms     | 1,407,835 |
| pot             | 2.33 ms      | 6.226 ms      | 4.698 ms     | 971,922   |
| flexon          | 2.69 ms      | 3.892 ms      | —            | 1,827,461 |
| ciborium        | 3.93 ms      | 11.06 ms      | —            | 1,407,835 |
| `serde_json`    | 3.99 ms      | 6.027 ms      | —            | 1,827,461 |
| nachricht-serde | 5.34 ms      | 4.054 ms      | 2.463 ms     | 818,669   |
| flexbuffers     | 6.84 ms      | 7.337 ms      | 5.660 ms     | 1,829,756 |
| ron             | 11.4 ms      | 26.9 ms       | 24.4 ms      | 1,607,459 |

### `mesh` (125,000 triangles, all `f32`)

| crate           | serialize    | deserialize   | size (B)   |
| --------------- | ------------ | ------------- | ---------- |
| **serde-zap**   | **481.4 µs** | 1.304 ms      | 6,000,005  |
| postcard        | 493.6 µs     | **1.076 ms**  | 6,000,003  |
| bincode 1.3.3   | 5.56 ms      | 6.01 ms       | 6,000,008  |
| `serde_bare`    | 5.80 ms      | 4.82 ms       | 6,000,003  |
| dlhn            | 6.03 ms      | 6.99 ms       | 6,000,003  |
| cbor4ii         | 8.99 ms      | 44.5 ms       | 13,125,016 |
| serde-brief     | 17.2 ms      | 34.7 ms       | 15,750,015 |
| rmp-serde       | 19.8 ms      | 16.9 ms       | 8,125,006  |
| `serde_cbor`    | 32.3 ms      | 43.9 ms       | 13,122,324 |
| pot             | 40.2 ms      | 63.9 ms       | 10,122,342 |
| ciborium        | 63.3 ms      | 111.1 ms      | 13,122,324 |
| flexon          | 68.6 ms      | 55.5 ms       | 26,192,883 |
| `serde_json`    | 86.0 ms      | 99.7 ms       | 26,192,883 |
| flexbuffers     | 104.4 ms     | 80.6 ms       | 26,609,424 |
| nachricht-serde | 118.6 ms     | 26.0 ms       | 8,125,037  |
| ron             | 170.5 ms     | 589.6 ms      | 22,192,885 |

### `minecraft_savedata` (500 deeply nested player saves)

| crate           | serialize    | deserialize   | borrow       | size (B)  |
| --------------- | ------------ | ------------- | ------------ | --------- |
| **serde-zap**   | **233.2 µs** | 1.949 ms      | **677.5 µs** | 367,413   |
| postcard        | 451.5 µs     | 2.159 ms      | 823.5 µs     | 367,489   |
| bincode 1.3.3   | 600.4 µs     | **1.837 ms**  | 840.2 µs     | 569,975   |
| dlhn            | 700.3 µs     | 2.590 ms      | —            | 366,496   |
| `serde_bare`    | 707.3 µs     | 2.313 ms      | —            | 356,311   |
| cbor4ii         | 765.2 µs     | 4.335 ms      | 3.214 ms     | 1,109,831 |
| serde-brief     | 1.18 ms      | 5.043 ms      | 3.473 ms     | 1,276,014 |
| rmp-serde       | 1.48 ms      | 2.992 ms      | 1.700 ms     | 424,533   |
| `serde_cbor`    | 1.93 ms      | 4.579 ms      | 3.368 ms     | 1,109,821 |
| pot             | 2.41 ms      | 5.842 ms      | 4.754 ms     | 599,125   |
| flexon          | 2.69 ms      | 4.521 ms      | —            | 1,623,191 |
| `serde_json`    | 3.71 ms      | 6.788 ms      | —            | 1,623,191 |
| ciborium        | 3.74 ms      | 9.905 ms      | —            | 1,109,821 |
| nachricht-serde | 4.89 ms      | 3.816 ms      | 2.779 ms     | 449,745   |
| flexbuffers     | 7.78 ms      | 6.825 ms      | 5.457 ms     | 1,187,688 |
| ron             | 8.15 ms      | 27.3 ms       | 25.7 ms      | 1,465,223 |

### `mk48` (1,000 enum-heavy game updates)

| crate           | serialize    | deserialize   | size (B)  |
| --------------- | ------------ | ------------- | --------- |
| **serde-zap**   | **719.3 µs** | **4.045 ms**  | 1,406,257 |
| postcard        | 1.87 ms      | 4.159 ms      | 1,311,281 |
| cbor4ii         | 3.47 ms      | 16.9 ms       | 6,012,539 |
| bincode 1.3.3   | 3.86 ms      | 4.357 ms      | 1,854,234 |
| `serde_bare`    | 4.12 ms      | 4.992 ms      | 1,319,999 |
| dlhn            | 4.45 ms      | 6.64 ms       | 1,311,281 |
| serde-brief     | 5.44 ms      | 20.5 ms       | 6,951,772 |
| `serde_cbor`    | 10.1 ms      | 20.1 ms       | 6,012,373 |
| rmp-serde       | 10.7 ms      | 11.1 ms       | 1,745,322 |
| pot             | 13.9 ms      | 29.7 ms       | 2,604,812 |
| flexon          | 15.0 ms      | 24.1 ms       | 9,390,461 |
| `serde_json`    | 20.3 ms      | 30.9 ms       | 9,390,461 |
| ciborium        | 23.6 ms      | 53.9 ms       | 6,012,373 |
| nachricht-serde | 29.4 ms      | 16.4 ms       | 1,770,060 |
| flexbuffers     | 39.4 ms      | 35.6 ms       | 5,352,680 |
| ron             | 45.8 ms      | 172.7 ms      | 8,677,703 |

### How to read this

- **serde-zap has the fastest serialize of every serde-trait crate on every dataset** — 2.1× the runner-up on log, 1.9× on minecraft, 2.6× on mk48, and a narrow win over postcard on mesh (both saturate memory bandwidth).
- **serde-zap has the fastest borrowed (zero-copy) deserialize** everywhere it is measured (log, minecraft) — 1.16× the runner-up (bincode 1.x) on log, 1.2× the runner-up (postcard) on minecraft.
- **Owned deserialize is a near-tie at the top**: serde-zap is outright fastest of the serde-trait crates on mk48, sits within ~6% of the leader (bincode 1.x) on log and minecraft, and is second to postcard on mesh. That metric is dominated by identical allocation work, not decoder speed (see borrow for the pure-decode comparison, which serde-zap wins).
- **Sizes**: serde-zap is byte-identical to bincode 2's varint scheme on all four datasets (identical raw _and_ zlib/zstd-compressed sizes in the published tables) — within a few percent of the smallest serde-trait encoding on every dataset (2 bytes behind the smallest on mesh).
- **bincode 2's serde adapter** (`bincode::serde`) is not covered by the upstream harness, which benches bincode 2 through its native API only. Measured locally on the same datasets/seed with the harness's calling convention (`to_slice` into a preallocated buffer vs `encode_to_vec`): serde-zap beat it by 1.5× (log serialize), 1.1× (log deserialize), 1.4× (log borrow), 2.2× (mesh serialize), and 1.4× (mesh deserialize) — with byte-identical wire output.
- **bincode 2's _native_ (non-serde) API** — what the harness benchmarks as "bincode" — beats every serde path only on mesh _deserialize_ (790.9 µs); on mesh _serialize_ it is 5× slower than serde-zap. The real non-serde frontier on mesh is ~149 µs ser/de (savefile, speedy, wincode — none go through serde), which is precisely the gap `full_vec`/`pod_vec` exist to close for users who can opt in.

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
