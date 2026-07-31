#![no_main]

//! Deserializing arbitrary bytes must never panic or cause UB — only clean
//! `Ok`/`Err` results. Covers every Reader/Deserializer code path including
//! the full_vec/pod_vec adapters and take_from_bytes.

use libfuzzer_sys::fuzz_target;

mod fuzz_types;
use fuzz_types::*;

use std::collections::BTreeMap;

fuzz_target!(|data: &[u8]| {
    let _ = serde_zap::from_bytes::<u8>(data);
    let _ = serde_zap::from_bytes::<u16>(data);
    let _ = serde_zap::from_bytes::<u32>(data);
    let _ = serde_zap::from_bytes::<u64>(data);
    let _ = serde_zap::from_bytes::<u128>(data);
    let _ = serde_zap::from_bytes::<i8>(data);
    let _ = serde_zap::from_bytes::<i16>(data);
    let _ = serde_zap::from_bytes::<i32>(data);
    let _ = serde_zap::from_bytes::<i64>(data);
    let _ = serde_zap::from_bytes::<i128>(data);
    let _ = serde_zap::from_bytes::<usize>(data);
    let _ = serde_zap::from_bytes::<isize>(data);
    let _ = serde_zap::from_bytes::<f32>(data);
    let _ = serde_zap::from_bytes::<f64>(data);
    let _ = serde_zap::from_bytes::<bool>(data);
    let _ = serde_zap::from_bytes::<char>(data);
    let _ = serde_zap::from_bytes::<String>(data);
    let _ = serde_zap::from_bytes::<&str>(data);
    let _ = serde_zap::from_bytes::<Vec<u8>>(data);
    let _ = serde_zap::from_bytes::<Vec<u64>>(data);
    let _ = serde_zap::from_bytes::<Vec<String>>(data);
    let _ = serde_zap::from_bytes::<Vec<Vec<i64>>>(data);
    let _ = serde_zap::from_bytes::<Option<Vec<String>>>(data);
    let _ = serde_zap::from_bytes::<(u32, String, Vec<u8>)>(data);
    let _ = serde_zap::from_bytes::<[i16; 4]>(data);
    let _ = serde_zap::from_bytes::<BTreeMap<String, u64>>(data);
    let _ = serde_zap::from_bytes::<SomeEnum>(data);
    let _ = serde_zap::from_bytes::<Outer>(data);
    let _ = serde_zap::from_bytes::<Vec<Outer>>(data);
    let _ = serde_zap::from_bytes::<FullVecW>(data);
    let _ = serde_zap::from_bytes::<PodVecW>(data);
    let _ = serde_zap::take_from_bytes::<Outer>(data);
});
