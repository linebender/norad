//! Simple benchmarks of glyph parsing.
//!
//! This should be run when making any changes to glyph parsing.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use norad::Glyph;

static S_GLYPH: &str = "testdata/MutatorSansLightWide.ufo/glyphs/S_.glif";
static DOT: &str = "testdata/MutatorSansLightWide.ufo/glyphs/dot.glif";
static A_ACUTE_GLYPH: &str = "testdata/MutatorSansLightWide.ufo/glyphs/A_acute.glif";
// largest glyph in noto cjk
static CID61855: &str = "testdata/cid61855.glif";

fn load_bytes(path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

pub fn criterion_benchmark(c: &mut Criterion) {
    // a normal glyph
    c.bench_function("parse S", |b| {
        let bytes = load_bytes(S_GLYPH);
        b.iter(|| {
            Glyph::parse_raw(black_box(&bytes)).unwrap();
        })
    });
    // a very small glyph
    c.bench_function("parse dot", |b| {
        let bytes = load_bytes(DOT);
        b.iter(|| {
            Glyph::parse_raw(black_box(&bytes)).unwrap();
        })
    });
    // a very large glyph
    c.bench_function("parse large CJK glyph", |b| {
        let bytes = load_bytes(CID61855);
        b.iter(|| {
            Glyph::parse_raw(black_box(&bytes)).unwrap();
        })
    });
    // a component glyph
    c.bench_function("parse A_acute", |b| {
        let bytes = load_bytes(A_ACUTE_GLYPH);
        b.iter(|| {
            Glyph::parse_raw(black_box(&bytes)).unwrap();
        })
    });
    // Note to somebody using this:
    //
    // It might be nice if we also had some other examples, like a glyph with
    // a large 'lib' section?
    c.bench_function("load S glyph", |b| {
        b.iter(|| {
            let data = std::fs::read(black_box(S_GLYPH)).unwrap();
            // just make sure we can't be optimized away?
            assert!(data.len() != 42);
        })
    });

    c.bench_function("load large CJK glyph", |b| {
        b.iter(|| {
            let data = std::fs::read(black_box(CID61855)).unwrap();
            // just make sure we can't be optimized away?
            assert!(data.len() != 42);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
