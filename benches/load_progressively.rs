use std::path::Path;

use criterion::BenchmarkId;
use criterion::Throughput;
use criterion::{criterion_group, criterion_main, Criterion};

use norad::Font;

#[inline]
fn load(path: &Path) {
    let _ = Font::load(path).unwrap();
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("load_progressively");

    for (glyph_number, ufo_name) in [
        (107, "NotoAmalgamated-SemiLight.ufo"),
        (347, "NotoAmalgamated-RegularCondensed.ufo"),
        (718, "NotoAmalgamated-Semibold.ufo"),
        (989, "NotoAmalgamated-BlackCondensed.ufo"),
        (1022, "NotoAmalgamated-Medium.ufo"),
        (2234, "NotoAmalgamated-Thin.ufo"),
        (2876, "NotoAmalgamated-Black.ufo"),
        (3793, "NotoAmalgamated-DisplayBoldCondensedItalic.ufo"),
        (7079, "NotoAmalgamated-DisplayBold.ufo"),
        (12871, "NotoAmalgamated-CondensedBoldItalic.ufo"),
        (24254, "NotoAmalgamated-SemiBold.ufo"),
        (35358, "NotoAmalgamated-Bold.ufo"),
        (60967, "NotoAmalgamated-Regular.ufo"),
    ] {
        let path = Path::new("../amalgamate-noto/").join(ufo_name);
        group.throughput(Throughput::Elements(glyph_number));
        group.bench_with_input(BenchmarkId::from_parameter(glyph_number), &path, |b, path| {
            b.iter(|| load(path));
        });
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
