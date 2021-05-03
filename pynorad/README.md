# Pynorad: python bindings for norad

Pynorad is an attempt to emulate the API of [`ufoLib2`], on top of the norad
crate.

## Status

This was an experiment, and it produced mixed results. I was able to produce
something that was approximately api-compatible, but that required me to write a
*lot* of code. A key takeaway for me is that python bindings for Rust projects
should probably not attempt to conform fully to things like python object
semantics.

## Getting Started

Setting up a development environment:

- have a newish version of python (3.6+)
- set up a venv (`python3 -m venv venv`)
- active (`source venv/bin/activate`)
- install packages: `pip install maturin pytest fonttools fs`
- `maturin develop`
- run tests `pytest`

## API compatability:

Most things work, but a few do not.

- object identity: pynorad uses "proxy objects" to represent references to
objects that are part of a font/layer. To python, each of these proxy objects
are distinct; this means that `assert font["a"] is font["a"]` will fail.
- `lib` fields: these aren't fully implemented. Basic access works, but you
  can't do `font.lib["my.dict"]["another.field"] = 42`. (That is: nested dict
  access doesn't really work.)
- module structure: everything in pynorad is in the root module; there is no
  `objects` module, for instance.

[`ufoLib2`]: https://ufolib2.readthedocs.io/en/latest/#

## Architecture

This library is weird.

First, objects generally have three layers; there is a thin wrapper defined in
python, which holds a reference to a python object defined in Rust, which itself
represents some pure rust object.

### Proxy objects

Because of rust's ownership semantics, if you do something like take a reference
to a glyph that is part of a layer, we cannot just hand off a reference to a
glyph. Instead what we do is create a proxy object; this is a shared reference
to the top-level font object, along with a function for retreiving the glyph in
question from the font.

Every time you access something on that glyph, we retreive it from the font
object, and then return the appropriate property.

One consequence of this is that you can invalidate references; for instance in
python you could write,

```python
font = Font.open("my_font.ufo")
glyph = font["a"]
del font["a"]
print(glyph.points)
```

This will raise an exception in python, because the call to `glyph.points`
attempts to find the glyph in the default layer, and it has been deleted.

This approach is interesting, and it mostly works, but it required a lot of
boilerplate code.


### But not *only* proxy objects

If every `Glyph` had to exist in a layer, you would not be able to do something
like, `new_glyph = Glyph("B")`. To make this work, most types can be *either* a
proxy object or a concrete object. When a concrete object is added to a layer,
it is turned into a proxy. This is error prone.
