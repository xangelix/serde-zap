#![cfg(feature = "alloc")]

use std::collections::BTreeMap;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use serde_zap::{Error, from_bytes, serialized_size, take_from_bytes, to_slice, to_vec};

fn roundtrip<T>(v: &T)
where
    T: Serialize + for<'de> Deserialize<'de> + PartialEq + Debug,
{
    let buf = to_vec(v).unwrap();
    assert_eq!(
        buf.len(),
        buf.capacity(),
        "to_vec must shrink to exact capacity"
    );
    assert_eq!(buf.len(), serialized_size(v).unwrap(), "size mismatch");
    let back: T = from_bytes(&buf).unwrap();
    assert_eq!(&back, v, "roundtrip mismatch");

    // to_slice with an exact-fit buffer must agree with to_vec.
    let mut slice_buf = vec![0u8; buf.len()];
    let written = to_slice(v, &mut slice_buf).unwrap();
    assert_eq!(written.len(), buf.len());
    assert_eq!(written, buf.as_slice());
}

#[test]
fn primitives() {
    roundtrip(&false);
    roundtrip(&true);
    roundtrip(&0u8);
    roundtrip(&255u8);
    roundtrip(&0i8);
    roundtrip(&-128i8);
    roundtrip(&127i8);
    roundtrip(&0.0f32);
    roundtrip(&-1.5f32);
    roundtrip(&f32::INFINITY);
    roundtrip(&0.0f64);
    roundtrip(&1.0e300f64);
    // NaN payloads must round-trip bit-exactly (NaN != NaN under PartialEq).
    let nan = -f64::NAN;
    let back: f64 = from_bytes(&to_vec(&nan).unwrap()).unwrap();
    assert_eq!(back.to_bits(), nan.to_bits());
    roundtrip(&'a');
    roundtrip(&'é');
    roundtrip(&'\u{10FFFF}');
    roundtrip(&'🦀');
    roundtrip(&());
}

#[test]
fn integer_edges() {
    for v in [
        0u64,
        1,
        250,
        251,
        255,
        65535,
        65536,
        u32::MAX as u64,
        u32::MAX as u64 + 1,
        u64::MAX,
    ] {
        roundtrip(&v);
    }
    for v in [0u16, 250, 251, u16::MAX] {
        roundtrip(&v);
    }
    for v in [0u32, 250, 251, 65535, 65536, u32::MAX] {
        roundtrip(&v);
    }
    roundtrip(&0u128);
    roundtrip(&u128::MAX);
    roundtrip(&0i16);
    roundtrip(&i16::MIN);
    roundtrip(&i16::MAX);
    roundtrip(&i32::MIN);
    roundtrip(&i32::MAX);
    roundtrip(&i64::MIN);
    roundtrip(&i64::MAX);
    roundtrip(&-1i64);
    roundtrip(&1i64);
    roundtrip(&i128::MIN);
    roundtrip(&i128::MAX);
    roundtrip(&usize::MAX);
    roundtrip(&isize::MIN);
}

#[test]
fn varint_encoded_sizes() {
    // Tagged varint: 1 byte <= 250, tag+u16 = 3, tag+u32 = 5, tag+u64 = 9.
    let cases: [(u64, usize); 10] = [
        (0, 1),
        (1, 1),
        (250, 1),
        (251, 3),
        (255, 3),
        (65535, 3),
        (65536, 5),
        (u32::MAX as u64, 5),
        (u32::MAX as u64 + 1, 9),
        (u64::MAX, 9),
    ];
    for (v, len) in cases {
        assert_eq!(to_vec(&v).unwrap().len(), len, "u64 {v}");
    }
    // u16 values never take more than tag+u16.
    assert_eq!(to_vec(&u16::MAX).unwrap().len(), 3);
    // zigzag: -1 -> 1, small magnitudes stay small.
    assert_eq!(to_vec(&-1i64).unwrap().len(), 1);
    assert_eq!(to_vec(&63i64).unwrap().len(), 1);
    assert_eq!(to_vec(&64i64).unwrap().len(), 1); // zigzag(64) = 128 <= 250
    assert_eq!(to_vec(&i64::MIN).unwrap().len(), 9);
    assert_eq!(to_vec(&u128::MAX).unwrap().len(), 17);
}

#[test]
fn strings() {
    roundtrip(&String::new());
    roundtrip(&"hello".to_string());
    roundtrip(&"🦀 rust 言語".to_string());
    let long = "x".repeat(300);
    roundtrip(&long);
    let very_long = "y".repeat(70_000);
    roundtrip(&very_long);
    // &str roundtrips through the borrowed path.
    let s = "borrowed";
    let buf = to_vec(&s).unwrap();
    let back: &str = from_bytes(&buf).unwrap();
    assert_eq!(back, s);
}

#[test]
fn borrowed_deserialize() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Borrowed<'a> {
        name: &'a str,
        tags: Vec<&'a str>,
        owned: String,
    }
    let v = Borrowed {
        name: "zap",
        tags: vec!["a", "bc", "def"],
        owned: "own".to_string(),
    };
    let buf = to_vec(&v).unwrap();
    let back: Borrowed<'_> = from_bytes(&buf).unwrap();
    assert_eq!(back, v);
}

#[test]
fn collections() {
    roundtrip(&Vec::<u32>::new());
    roundtrip(&vec![0u32, 250, 251, 65536]);
    roundtrip(&vec!["a".to_string(), "bc".to_string()]);
    roundtrip(&vec![vec![1u8, 2], vec![], vec![3]]);
    roundtrip(&[1u16, 2, 3, 4]);
    roundtrip(&[0u8; 32]);
    let mut map = BTreeMap::new();
    map.insert("one".to_string(), 1u64);
    map.insert("two".to_string(), 2);
    roundtrip(&map);
    roundtrip(&(1u8, "two".to_string(), 3.0f64));
    roundtrip(&((), (1u8, (2u16,))));
}

#[test]
fn enums_and_options() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum E {
        Unit,
        Newtype(u32),
        Tuple(u8, String),
        Struct { a: i64, b: Option<bool> },
    }
    roundtrip(&E::Unit);
    roundtrip(&E::Newtype(65536));
    roundtrip(&E::Tuple(7, "seven".to_string()));
    roundtrip(&E::Struct {
        a: -42,
        b: Some(true),
    });
    roundtrip(&Option::<u64>::None);
    roundtrip(&Some(42u64));
    roundtrip(&Some(Some(Some(false))));
    roundtrip(&Option::<String>::None);
    roundtrip(&Result::<u8, String>::Ok(9));
    roundtrip(&Result::<u8, String>::Err("boom".to_string()));
}

#[test]
fn nested_structs() {
    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Inner {
        x: f32,
        y: f32,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Outer {
        id: u64,
        inner: Inner,
        items: Vec<Inner>,
        label: String,
        maybe: Option<Box<Inner>>,
    }
    roundtrip(&Outer {
        id: 123_456_789,
        inner: Inner { x: 1.0, y: -1.0 },
        items: vec![Inner { x: 0.5, y: 0.25 }; 10],
        label: "outer".to_string(),
        maybe: Some(Box::new(Inner { x: 9.0, y: 8.0 })),
    });
}

#[test]
fn error_cases() {
    // Truncated input.
    let buf = to_vec(&"hello".to_string()).unwrap();
    assert!(from_bytes::<String>(&buf[..buf.len() - 1]).is_err());
    // Invalid bool.
    assert_eq!(from_bytes::<bool>(&[2]), Err(Error::InvalidBool));
    // Invalid option tag.
    assert_eq!(from_bytes::<Option<u8>>(&[3]), Err(Error::InvalidOption));
    // Invalid varint tag for u16 (tag 252 is u32-width).
    assert_eq!(
        from_bytes::<u16>(&[252, 0, 0, 0, 0]),
        Err(Error::InvalidVarint)
    );
    // Invalid char (surrogate).
    assert_eq!(
        from_bytes::<char>(&[251, 0x00, 0xD8]),
        Err(Error::InvalidChar)
    );
    // Invalid UTF-8.
    assert_eq!(from_bytes::<String>(&[1, 0xFF]), Err(Error::InvalidUtf8));
    // to_slice into a too-small buffer.
    let mut tiny = [0u8; 2];
    assert_eq!(to_slice(&"hello", &mut tiny), Err(Error::ExceedsBuffer));
    // deserialize_any is unsupported.
    assert!(from_bytes::<serde::de::IgnoredAny>(&[0]).is_err());
}

#[test]
fn full_vec_adapter() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct W {
        #[serde(with = "serde_zap::full_vec")]
        items: Vec<u32>,
    }
    let v = W {
        items: (0..1000).collect(),
    };
    // Byte-identical to the stock Vec encoding.
    let stock = to_vec(&v.items).unwrap();
    let buf = to_vec(&v).unwrap();
    assert_eq!(buf, stock);
    let back: W = from_bytes(&buf).unwrap();
    assert_eq!(back, v);
}

#[test]
fn pod_vec_adapter() {
    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
    #[repr(C)]
    struct Point {
        x: f32,
        y: f32,
    }
    // SAFETY: repr(C), no padding, all bit patterns valid.
    unsafe impl serde_zap::pod_vec::Pod for Point {}

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct W {
        #[serde(with = "serde_zap::pod_vec")]
        points: Vec<Point>,
    }
    let v = W {
        points: (0..100)
            .map(|i| Point {
                x: i as f32,
                y: -i as f32,
            })
            .collect(),
    };
    let buf = to_vec(&v).unwrap();
    // Layout: byte-length varint (800 > 250 -> 3 bytes) + len * 8 raw bytes.
    assert_eq!(buf.len(), 3 + 100 * 8);
    let back: W = from_bytes(&buf).unwrap();
    assert_eq!(back, v);

    // Corrupt: byte length not a multiple of the element size.
    let mut bad = buf.clone();
    bad[0] = 7; // claim 7 bytes
    bad.truncate(1 + 7);
    assert!(from_bytes::<W>(&bad).is_err());
}

#[test]
fn dos_length_guard() {
    // A u64::MAX length prefix must produce a clean error, not a huge
    // allocation, on both the stock Vec path and the full_vec adapter.
    let evil = [253, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    assert!(from_bytes::<Vec<u32>>(&evil).is_err());

    #[derive(Serialize, Deserialize, Debug)]
    struct W {
        #[serde(with = "serde_zap::full_vec")]
        items: Vec<u32>,
    }
    assert!(from_bytes::<W>(&evil).is_err());

    // Zero-sized elements still roundtrip (size_hint falls back to None).
    roundtrip(&vec![(); 100]);
}

#[test]
fn to_vec_two_pass_matches_to_vec() {
    let v = (
        42u64,
        "a reasonably short string".to_string(),
        (0..300).map(|i| format!("item-{i}")).collect::<Vec<_>>(),
        vec![7u32; 1000],
    );
    let single = to_vec(&v).unwrap();
    let two = serde_zap::to_vec_two_pass(&v).unwrap();
    assert_eq!(single, two, "single-pass and two-pass must agree");
    assert_eq!(two.len(), two.capacity(), "two-pass must be exact capacity");
}

#[test]
fn take_from_bytes_rest() {
    let mut buf = to_vec(&42u64).unwrap();
    buf.extend_from_slice(&[9, 9, 9]);
    let (v, rest): (u64, _) = take_from_bytes(&buf).unwrap();
    assert_eq!(v, 42);
    assert_eq!(rest, &[9, 9, 9]);
}
