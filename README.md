[![crates.io](https://img.shields.io/crates/v/norad.svg)](https://crates.io/crates/norad)
[![docs.rs](https://img.shields.io/docsrs/norad.svg)](https://docs.rs/norad/latest/norad/)
[![Continuous integration](https://github.com/linebender/norad/actions/workflows/rust.yml/badge.svg)](https://github.com/linebender/norad/actions/workflows/rust.yml)

# Norad

A crate for reading, writing, and manipulating [Unified Font Object][ufo] files,
a common font-design format.

The types in this crate correspond to types described in the spec.

[ufo]: http://unifiedfontobject.org/versions/ufo3

## Basic Usage

Instantiate a UFO font object with a [`Font`] struct like this:

```rust
use norad::Font;

let inpath = "RoflsExtraDim.ufo";
let mut font_obj = Font::load(inpath).expect("failed to load font");
```

The API may be used to access and modify data in the [`Font`]:

```rust
let layer = font_obj.default_layer();
let glyph_a = layer.get_glyph("A").expect("missing glyph");
assert_eq!(glyph_a.name().as_ref(), "A");
```

Serialize the [`Font`] to UFO files on disk with the [`Font::save`] method:

```rust
let outpath = "RoflsSemiDim.ufo";
font_obj.save(outpath);
```

Refer to the [`examples` directory of the source repository](https://github.com/linebender/norad/tree/master/examples)
for additional source code examples.

[`Font`]: https://docs.rs/norad/latest/norad/struct.Font.html
[`Font::save`]: https://docs.rs/norad/latest/norad/struct.Font.html#method.save

## API Documentation

Details on the full API for working with UFO fonts are available on [docs.rs](https://docs.rs/norad/latest/norad/).

## License

norad is licensed under the [MIT](https://github.com/linebender/norad/blob/master/LICENSE-MIT)
and [Apache v2.0](https://github.com/linebender/norad/blob/master/LICENSE-APACHE) licenses.

## Source

Source files are available [on GitHub](https://github.com/linebender/norad).
