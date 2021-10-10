use std::path::Path;

use criterion::BenchmarkId;
use criterion::SamplingMode;
use criterion::Throughput;
use criterion::{criterion_group, criterion_main, Criterion};

use norad::DataRequest;
use norad::Font;

#[inline]
fn load(path: &Path) {
    let _ = Font::load(path).unwrap();
}

#[inline]
fn load_no_glyph_lib(path: &Path) {
    let mut request = DataRequest::default();
    request.glyph_lib(false);
    let _ = Font::load_requested_data(path, request).unwrap();
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("load_progressively");
    group.sampling_mode(SamplingMode::Flat);

    for (glyph_number, ufo_name) in [
        (107, "NotoAmalgamated-SemiLight.ufo"),
        (347, "NotoAmalgamated-RegularCondensed.ufo"),
        (1022, "NotoAmalgamated-Medium.ufo"),
        (2234, "NotoAmalgamated-Thin.ufo"),
        (3793, "NotoAmalgamated-DisplayBoldCondensedItalic.ufo"),
        (7079, "NotoAmalgamated-DisplayBold.ufo"),
        (12871, "NotoAmalgamated-CondensedBoldItalic.ufo"),
        (24254, "NotoAmalgamated-SemiBold.ufo"),
        (35358, "NotoAmalgamated-Bold.ufo"),
        (60967, "NotoAmalgamated-Regular.ufo"),
    ] {
        let path = Path::new("../amalgamate-noto/").join(ufo_name);
        group.throughput(Throughput::Elements(glyph_number));
        group.bench_with_input(
            BenchmarkId::new("With glyph lib", glyph_number),
            &path,
            |b, path| {
                b.iter(|| load(path));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("Without glyph lib", glyph_number),
            &path,
            |b, path| {
                b.iter(|| load_no_glyph_lib(path));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
