# v0.1.1

## Added

- **Streaming I/O** (`std` feature, on by default): `to_writer` serializes into any `io::Write` — small writes are staged in an 8 KiB buffer, bulk writes pass straight through — and `from_reader` deserializes owned values from any `io::Read`. Both return `std::io::Result` with underlying I/O errors preserved verbatim, and output is byte-identical to `to_vec`.
- `tests/stream.rs`: integration tests for the stream adapters (stage-boundary byte-identity, write coalescing, error propagation); also runs under Miri.
- `examples/stream_bench.rs`: `to_vec` vs `to_writer`, directly and piped through zstd.

## Changed

- `default` features are now `["alloc", "std"]`; `--no-default-features` bare-metal `no_std` builds are unaffected.
- README and crates.io description now lead with the independently published [rust_serialization_benchmark](https://github.com/djkoloski/rust_serialization_benchmark) results (merged upstream in PR #151, 2026-08-02 run), and document the stream adapters.
- CI: the benchmark job pins upstream past the serde-zap merge (PR #151); its patch now only redirects serde-zap to the local checkout.
- CI: the test job now also covers the alloc-only build (`--no-default-features --features alloc`), and the stream integration tests are feature-gated on `std` like the rest of the suite.

# v0.1.0

Initial release!
