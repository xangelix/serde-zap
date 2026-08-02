//! to_vec vs to_writer into a real buffer vs both piped through zstd.

use std::io::Cursor;
use std::time::Instant;

use serde::Serialize;

#[derive(Serialize)]
struct Log {
    a: u64,
    b: String,
    c: String,
    d: String,
    e: String,
    f: u16,
}

#[derive(Serialize, Clone, Copy)]
struct V3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Serialize)]
struct Tri {
    a: V3,
    b: V3,
    c: V3,
    n: V3,
}

fn run<T: Serialize>(name: &str, v: &T, iters: usize) {
    let size = serde_zap::serialized_size(v).unwrap();

    // (a) to_vec
    let t = Instant::now();
    for _ in 0..iters {
        std::hint::black_box(serde_zap::to_vec(std::hint::black_box(v)).unwrap());
    }
    let a = t.elapsed() / iters as u32;

    // (b) to_writer into a real Vec-backed cursor
    let t = Instant::now();
    for _ in 0..iters {
        let mut buf = Cursor::new(Vec::new());
        serde_zap::to_writer(std::hint::black_box(v), &mut buf).unwrap();
        std::hint::black_box(buf);
    }
    let b = t.elapsed() / iters as u32;

    // (c) to_vec, then zstd::encode_all
    let t = Instant::now();
    for _ in 0..iters {
        let buf = serde_zap::to_vec(std::hint::black_box(v)).unwrap();
        std::hint::black_box(zstd::encode_all(Cursor::new(&buf), 0).unwrap());
    }
    let c = t.elapsed() / iters as u32;

    // (d) to_writer into zstd::Encoder over a Vec
    let t = Instant::now();
    for _ in 0..iters {
        let mut enc = zstd::Encoder::new(Vec::new(), 0).unwrap();
        serde_zap::to_writer(std::hint::black_box(v), &mut enc).unwrap();
        std::hint::black_box(enc.finish().unwrap());
    }
    let d = t.elapsed() / iters as u32;

    println!(
        "{name} ({size} B)\n  direct: to_vec {a:?}  vs  to_writer→cursor {b:?}\n  zstd:   to_vec+encode_all {c:?}  vs  to_writer→encoder {d:?}"
    );
}

fn main() {
    let logs: Vec<Log> = (0..10_000)
        .map(|i| Log {
            a: i as u64 * 999983,
            b: format!("user{}", i % 500),
            c: format!("GET /items/{} HTTP/1.1", i % 1000),
            d: "HTTP/1.1".into(),
            e: format!("agent-{}", i % 97),
            f: (i % 600) as u16,
        })
        .collect();
    let tris: Vec<Tri> = (0..125_000)
        .map(|i| {
            let f = i as f32;
            let v = V3 { x: f, y: f, z: f };
            Tri {
                a: v,
                b: v,
                c: v,
                n: v,
            }
        })
        .collect();
    run("log-like", &logs, 200);
    run("mesh-like", &tris, 50);
}
