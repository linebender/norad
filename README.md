# Norad

**a Rust crate for working with [Unified Font Object][ufo] files.**

This crate currently provides very minimal functionality for loading glyphs
from `.ufo` directories. It is expected to expand as needs require.

## This is a fork

This is the MFEQ fork of Norad. Right now it is essentially the same, just with
a new `UfoDataRequest` type so MFEQ modules don't need to parse an entire UFO
file to get the data out that they need. For example,
[Qmetadata](https://github.com/mfeq/Qmetadata) only needs the `MetaInfo` and
`FontInfo`. See
[linebender/norad#53](https://github.com/linebender/norad/pull/53).

## This is not how MFEQ modules parse glyphs

That'd be [`glifparser`](https://github.com/mfeq/glifparser)`.

[ufo]: http://unifiedfontobject.org/versions/ufo3
