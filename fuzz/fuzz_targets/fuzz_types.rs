// Shared fuzz types: they exercise every serializer/deserializer code path
// (all int widths, floats, bool, char, strings, bytes, options, enums of all
// four variant kinds, tuples, arrays, maps, nested vecs, and the full_vec /
// pod_vec adapters).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, arbitrary::Arbitrary)]
pub enum SomeEnum {
    Unit,
    Newtype(u64),
    Tuple(u32, String),
    Struct { a: i64, b: Option<bool>, c: Vec<u8> },
}

#[derive(Serialize, Deserialize, Debug, arbitrary::Arbitrary)]
pub struct Inner {
    pub flag: bool,
    pub ch: char,
    pub f: f32,
    pub d: f64,
    pub s: String,
    pub v: Vec<i128>,
}

#[derive(Serialize, Deserialize, Debug, arbitrary::Arbitrary)]
pub struct Outer {
    pub a: u128,
    pub b: i64,
    pub inner: Inner,
    pub opt: Option<Vec<String>>,
    pub e: SomeEnum,
    pub t: (u8, u16, String),
    pub arr: [i32; 3],
    pub map: BTreeMap<String, u64>,
    pub nested: Vec<Vec<Inner>>,
}

#[derive(Serialize, Deserialize, Debug, arbitrary::Arbitrary)]
pub struct FullVecW {
    #[serde(with = "serde_zap::full_vec")]
    pub items: Vec<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, arbitrary::Arbitrary)]
#[repr(C)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// SAFETY: repr(C), all fields f32 (no padding), every bit pattern valid.
unsafe impl serde_zap::pod_vec::Pod for Point {}

#[derive(Serialize, Deserialize, Debug, arbitrary::Arbitrary)]
pub struct PodVecW {
    #[serde(with = "serde_zap::pod_vec")]
    pub points: Vec<Point>,
}
