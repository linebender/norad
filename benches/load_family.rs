use criterion::{criterion_group, criterion_main, Criterion};

use norad::Font;

#[inline]
fn load_family_small() {
    let _ = Font::load("testdata/mutatorSans/MutatorSansBoldCondensed.ufo").unwrap();
    let _ = Font::load("testdata/mutatorSans/MutatorSansBoldWide.ufo").unwrap();
    let _ = Font::load("testdata/mutatorSans/MutatorSansIntermediateCondensed.ufo").unwrap();
    let _ = Font::load("testdata/mutatorSans/MutatorSansIntermediateWide.ufo").unwrap();
    let _ = Font::load("testdata/mutatorSans/MutatorSansLightCondensed.ufo").unwrap();
    let _ = Font::load("testdata/mutatorSans/MutatorSansLightWide.ufo").unwrap();
}

#[inline]
fn load_family_medium() {
    let _ = Font::load("/tmp/NotoSans-Bold.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-Condensed.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-CondensedBold.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-CondensedLight.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-CondensedSemiBold.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-DisplayBold.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-DisplayCondensed.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-DisplayLight.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-DisplayLightCondensed.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-DisplayRegular.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-DisplaySemiBold.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-DisplaySemiBoldCondensed.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-Light.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-Regular.ufo").unwrap();
    let _ = Font::load("/tmp/NotoSans-SemiBold.ufo").unwrap();
}

#[inline]
fn load_family_very_large() {
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-SemiLight.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-RegularCondensed.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-ThinCondensed.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-Semibold.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-CondensedExtraBlack.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-CondensedExtraThin.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-BlackCondensed.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-Medium.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-Thin.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-Black.ufo").unwrap();
    let _ =
        Font::load("../amalgamate-noto/NotoAmalgamated-DisplayBoldCondensedItalic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplayBoldItalic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplayCondensedItalic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplayItalic.ufo").unwrap();
    let _ =
        Font::load("../amalgamate-noto/NotoAmalgamated-DisplayLightCondensedItalic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplayLightItalic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplaySemiBoldCondensedItalic.ufo")
        .unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplaySemiBoldItalic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplayBold.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplayBoldCondensed.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplayCondensed.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplayLight.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplayLightCondensed.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplayRegular.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplaySemiBold.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-DisplaySemiBoldCondensed.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-CondensedBoldItalic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-CondensedItalic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-CondensedLightItalic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-CondensedSemiBoldItalic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-LightItalic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-SemiBoldItalic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-BoldItalic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-Italic.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-CondensedSemiBold.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-Condensed.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-CondensedBold.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-CondensedLight.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-SemiBold.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-Light.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-Bold.ufo").unwrap();
    let _ = Font::load("../amalgamate-noto/NotoAmalgamated-Regular.ufo").unwrap();
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("load_family_small", |b| b.iter(load_family_small));
    c.bench_function("load_family_medium", |b| b.iter(load_family_medium));
    c.bench_function("load_family_very_large", |b| b.iter(load_family_very_large));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
