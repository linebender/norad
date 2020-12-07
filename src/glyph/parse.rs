use std::borrow::Borrow;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use super::*;
use crate::error::{ErrorKind, GlifErrorInternal};
use crate::glyph::builder::{GlyphBuilder, OutlineBuilder};
use crate::names::NameList;

use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

#[cfg(test)]
pub(crate) fn parse_glyph(xml: &[u8]) -> Result<Glyph, GlifErrorInternal> {
    GlifParser::from_xml(xml, None)
}

macro_rules! err {
    ($r:expr, $errtype:expr) => {
        GlifErrorInternal::Spec { kind: $errtype, position: $r.buffer_position() }
    };
}

type Error = GlifErrorInternal;

pub(crate) struct GlifParser<'names> {
    builder: GlyphBuilder,
    /// Optional set of glyph names to be reused between glyphs.
    names: Option<&'names NameList>,
}

impl<'names> GlifParser<'names> {
    pub(crate) fn from_xml(xml: &[u8], names: Option<&'names NameList>) -> Result<Glyph, Error> {
        let mut reader = Reader::from_reader(xml);
        let mut buf = Vec::new();
        reader.trim_text(true);

        let builder = start(&mut reader, &mut buf)?;
        let this = GlifParser { builder, names };
        this.parse_body(&mut reader, &mut buf)
    }

    fn parse_body(mut self, reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<Glyph, Error> {
        loop {
            match reader.read_event(buf)? {
                // outline, lib and note are expected to be start element tags.
                Event::Start(start) => {
                    let tag_name = reader.decode(&start.name())?;
                    match tag_name.borrow() {
                        "outline" => self.parse_outline(reader, buf)?,
                        "lib" => self.parse_lib(reader, buf)?, // do this at some point?
                        "note" => self.parse_note(reader, buf)?,
                        _other => return Err(err!(reader, ErrorKind::UnexpectedTag)),
                    }
                }
                // The rest are expected to be empty element tags (exception: outline) with attributes.
                Event::Empty(start) => {
                    let tag_name = reader.decode(&start.name())?;
                    match tag_name.borrow() {
                        "outline" => {
                            // ufoLib parses `<outline/>` as an empty outline.
                            self.builder
                                .outline(Outline::default(), HashSet::new())
                                .map_err(|e| err!(reader, e))?;
                        }
                        "advance" => self.parse_advance(reader, start)?,
                        "unicode" => self.parse_unicode(reader, start)?,
                        "anchor" => self.parse_anchor(reader, start)?,
                        "guideline" => self.parse_guideline(reader, start)?,
                        "image" => self.parse_image(reader, start)?,
                        _other => return Err(err!(reader, ErrorKind::UnexpectedTag)),
                    }
                }
                Event::End(ref end) if end.name() == b"glyph" => break,
                _other => return Err(err!(reader, ErrorKind::MissingCloseTag)),
            }
        }
        Ok(self.builder.finish().map_err(|e| err!(reader, e))?)
    }

    fn parse_outline(
        &mut self,
        reader: &mut Reader<&[u8]>,
        buf: &mut Vec<u8>,
    ) -> Result<(), Error> {
        let mut outline_builder = OutlineBuilder::new();

        loop {
            match reader.read_event(buf)? {
                Event::Start(start) => {
                    let tag_name = reader.decode(&start.name())?;
                    let mut new_buf = Vec::new(); // borrowck :/
                    match tag_name.borrow() {
                        "contour" => {
                            self.parse_contour(start, reader, &mut new_buf, &mut outline_builder)?
                        }
                        _other => return Err(err!(reader, ErrorKind::UnexpectedTag)),
                    }
                }
                Event::Empty(start) => {
                    let tag_name = reader.decode(&start.name())?;
                    match tag_name.borrow() {
                        // Skip empty contours as meaningless.
                        // https://github.com/unified-font-object/ufo-spec/issues/150
                        "contour" => (),
                        "component" => self.parse_component(reader, start, &mut outline_builder)?,
                        _other => return Err(err!(reader, ErrorKind::UnexpectedTag)),
                    }
                }
                Event::End(ref end) if end.name() == b"outline" => break,
                Event::Eof => return Err(err!(reader, ErrorKind::UnexpectedEof)),
                _other => return Err(err!(reader, ErrorKind::UnexpectedElement)),
            }
        }

        let (outline, identifiers) = outline_builder.finish().map_err(|e| err!(reader, e))?;
        self.builder.outline(outline, identifiers).map_err(|e| err!(reader, e))?;

        Ok(())
    }

    fn parse_contour(
        &mut self,
        data: BytesStart,
        reader: &mut Reader<&[u8]>,
        buf: &mut Vec<u8>,
        outline_builder: &mut OutlineBuilder,
    ) -> Result<(), Error> {
        let mut identifier = None;
        for attr in data.attributes() {
            if self.builder.get_format() == &GlifVersion::V1 {
                return Err(err!(reader, ErrorKind::UnexpectedAttribute));
            }
            let attr = attr?;
            match attr.key {
                b"identifier" => {
                    let ident = attr.unescape_and_decode_value(reader)?;
                    identifier = Some(Identifier::new(ident).map_err(|kind| err!(reader, kind))?);
                }
                _other => return Err(err!(reader, ErrorKind::UnexpectedAttribute)),
            }
        }

        outline_builder.begin_path(identifier).map_err(|e| err!(reader, e))?;
        loop {
            match reader.read_event(buf)? {
                Event::End(ref end) if end.name() == b"contour" => break,
                Event::Empty(ref start) if start.name() == b"point" => {
                    self.parse_point(reader, start, outline_builder)?;
                }
                Event::Eof => return Err(err!(reader, ErrorKind::UnexpectedEof)),
                _other => return Err(err!(reader, ErrorKind::UnexpectedElement)),
            }
        }
        outline_builder.end_path().map_err(|e| err!(reader, e))?;

        Ok(())
    }

    fn parse_component(
        &mut self,
        reader: &mut Reader<&[u8]>,
        start: BytesStart,
        outline_builder: &mut OutlineBuilder,
    ) -> Result<(), Error> {
        let mut base: Option<GlyphName> = None;
        let mut identifier: Option<Identifier> = None;
        let mut transform = AffineTransform::default();

        for attr in start.attributes() {
            let attr = attr?;
            let value = attr.unescaped_value()?;
            let value = reader.decode(&value)?;
            let pos = reader.buffer_position();
            let kind = ErrorKind::BadNumber;
            match attr.key {
                b"xScale" => transform.x_scale = value.parse().map_err(|_| (kind, pos))?,
                b"xyScale" => transform.xy_scale = value.parse().map_err(|_| (kind, pos))?,
                b"yxScale" => transform.yx_scale = value.parse().map_err(|_| (kind, pos))?,
                b"yScale" => transform.y_scale = value.parse().map_err(|_| (kind, pos))?,
                b"xOffset" => transform.x_offset = value.parse().map_err(|_| (kind, pos))?,
                b"yOffset" => transform.y_offset = value.parse().map_err(|_| (kind, pos))?,
                b"base" => {
                    let name: Arc<str> = value.into();
                    let name = match self.names.as_ref() {
                        Some(names) => names.get(&name),
                        None => name,
                    };
                    base = Some(name);
                }
                b"identifier" => {
                    identifier = Some(value.parse().map_err(|kind| err!(reader, kind))?);
                }
                _other => return Err(err!(reader, ErrorKind::UnexpectedComponentField)),
            }
        }

        if base.is_none() {
            return Err(err!(reader, ErrorKind::BadComponent));
        }

        outline_builder
            .add_component(base.unwrap(), transform, identifier)
            .map_err(|e| err!(reader, e))?;
        Ok(())
    }

    fn parse_lib(&mut self, reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<(), Error> {
        loop {
            match reader.read_event(buf)? {
                Event::End(ref end) if end.name() == b"lib" => break,
                Event::Eof => return Err(err!(reader, ErrorKind::UnexpectedEof)),
                _other => (),
            }
        }
        Ok(())
    }

    fn parse_note(&mut self, reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<(), Error> {
        loop {
            match reader.read_event(buf)? {
                Event::End(ref end) if end.name() == b"note" => break,
                Event::Text(text) => {
                    self.builder
                        .note(text.unescape_and_decode(reader)?)
                        .map_err(|e| err!(reader, e))?;
                }
                Event::Eof => return Err(err!(reader, ErrorKind::UnexpectedEof)),
                _other => (),
            }
        }
        Ok(())
    }

    fn parse_point<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: &BytesStart<'a>,
        outline_builder: &mut OutlineBuilder,
    ) -> Result<(), Error> {
        let mut name: Option<String> = None;
        let mut x: Option<f32> = None;
        let mut y: Option<f32> = None;
        let mut typ = PointType::OffCurve;
        let mut identifier: Option<Identifier> = None;
        let mut smooth = false;

        for attr in data.attributes() {
            let attr = attr?;
            let value = attr.unescaped_value()?;
            let value = reader.decode(&value)?;
            let pos = reader.buffer_position();
            match attr.key {
                b"x" => {
                    x = Some(value.parse().map_err(|_| (ErrorKind::BadNumber, pos))?);
                }
                b"y" => {
                    y = Some(value.parse().map_err(|_| (ErrorKind::BadNumber, pos))?);
                }
                b"name" => name = Some(value.to_string()),
                b"type" => {
                    typ = value
                        .parse()
                        .map_err(|e: ErrorKind| e.to_error(reader.buffer_position()))?
                }
                b"smooth" => smooth = value == "yes",
                b"identifier" => {
                    identifier = Some(value.parse().map_err(|kind| err!(reader, kind))?);
                }
                _other => return Err(err!(reader, ErrorKind::UnexpectedPointField)),
            }
        }
        if x.is_none() || y.is_none() {
            return Err(err!(reader, ErrorKind::BadPoint));
        }
        outline_builder
            .add_point((x.unwrap(), y.unwrap()), typ, smooth, name, identifier)
            .map_err(|e| err!(reader, e))?;

        Ok(())
    }

    fn parse_advance<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), Error> {
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        for attr in data.attributes() {
            let attr = attr?;
            match attr.key {
                b"width" | b"height" => {
                    let value = attr.unescaped_value()?;
                    let value = reader.decode(&value)?;
                    let value: f32 =
                        value.parse().map_err(|_| err!(reader, ErrorKind::BadNumber))?;
                    match attr.key {
                        b"width" => width = value,
                        b"height" => height = value,
                        _other => unreachable!(),
                    };
                }
                _other => return Err(err!(reader, ErrorKind::UnexpectedAttribute)),
            }
        }
        self.builder
            .width(width)
            .map_err(|e| err!(reader, e))?
            .height(height)
            .map_err(|e| err!(reader, e))?;
        Ok(())
    }

    fn parse_unicode<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), Error> {
        for attr in data.attributes() {
            let attr = attr?;
            match attr.key {
                b"hex" => {
                    let value = attr.unescaped_value()?;
                    let value = reader.decode(&value)?;
                    let chr = u32::from_str_radix(&value, 16)
                        .map_err(|_| value.to_string())
                        .and_then(|n| char::try_from(n).map_err(|_| value.to_string()))
                        .map_err(|_| err!(reader, ErrorKind::BadHexValue))?;
                    self.builder.unicode(chr);
                }
                _other => return Err(err!(reader, ErrorKind::UnexpectedAttribute)),
            }
        }
        Ok(())
    }

    fn parse_anchor<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), Error> {
        let mut x: Option<f32> = None;
        let mut y: Option<f32> = None;
        let mut name: Option<String> = None;
        let mut color: Option<Color> = None;
        let mut identifier: Option<Identifier> = None;

        for attr in data.attributes() {
            let attr = attr?;
            let value = attr.unescaped_value()?;
            let value = reader.decode(&value)?;
            match attr.key {
                b"x" => {
                    x = Some(value.parse().map_err(|_| err!(reader, ErrorKind::BadNumber))?);
                }
                b"y" => {
                    y = Some(value.parse().map_err(|_| err!(reader, ErrorKind::BadNumber))?);
                }
                b"name" => name = Some(value.to_string()),
                b"color" => {
                    color = Some(value.parse().map_err(|_| err!(reader, ErrorKind::BadColor))?)
                }
                b"identifier" => {
                    identifier = Some(value.parse().map_err(|kind| err!(reader, kind))?);
                }
                _other => return Err(err!(reader, ErrorKind::UnexpectedAnchorField)),
            }
        }

        if x.is_none() || y.is_none() {
            return Err(err!(reader, ErrorKind::BadAnchor));
        }
        self.builder
            .anchor(Anchor { x: x.unwrap(), y: y.unwrap(), name, color, identifier })
            .map_err(|e| err!(reader, e))?;
        Ok(())
    }

    fn parse_guideline<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), Error> {
        let mut x: Option<f32> = None;
        let mut y: Option<f32> = None;
        let mut angle: Option<f32> = None;
        let mut name: Option<String> = None;
        let mut color: Option<Color> = None;
        let mut identifier: Option<Identifier> = None;

        for attr in data.attributes() {
            let attr = attr?;
            let value = attr.unescaped_value()?;
            let value = reader.decode(&value)?;
            let pos = reader.buffer_position();
            match attr.key {
                b"x" => {
                    x = Some(value.parse().map_err(|_| err!(reader, ErrorKind::BadNumber))?);
                }
                b"y" => {
                    y = Some(value.parse().map_err(|_| err!(reader, ErrorKind::BadNumber))?);
                }
                b"angle" => {
                    angle = Some(value.parse().map_err(|_| err!(reader, ErrorKind::BadNumber))?);
                }
                b"name" => name = Some(value.to_string()),
                b"color" => color = Some(value.parse().map_err(|e: ErrorKind| e.to_error(pos))?),
                b"identifier" => {
                    identifier = Some(value.parse().map_err(|kind| err!(reader, kind))?);
                }
                _other => return Err(err!(reader, ErrorKind::UnexpectedGuidelineField)),
            }
        }

        let line = match (x, y, angle) {
            (Some(x), None, None) => Line::Vertical(x),
            (None, Some(y), None) => Line::Horizontal(y),
            (Some(x), Some(y), Some(degrees)) => {
                if !(0.0..=360.0).contains(&degrees) {
                    return Err(err!(reader, ErrorKind::BadGuideline));
                }
                Line::Angle { x, y, degrees }
            }
            _other => return Err(err!(reader, ErrorKind::BadGuideline)),
        };
        self.builder
            .guideline(Guideline { line, name, color, identifier })
            .map_err(|e| err!(reader, e))?;

        Ok(())
    }

    fn parse_image<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), Error> {
        let mut filename: Option<PathBuf> = None;
        let mut color: Option<Color> = None;
        let mut transform = AffineTransform::default();

        for attr in data.attributes() {
            let attr = attr?;
            let value = attr.unescaped_value()?;
            let value = reader.decode(&value)?;
            let pos = reader.buffer_position();
            let kind = ErrorKind::BadNumber;
            match attr.key {
                b"xScale" => transform.x_scale = value.parse().map_err(|_| (kind, pos))?,
                b"xyScale" => transform.xy_scale = value.parse().map_err(|_| (kind, pos))?,
                b"yxScale" => transform.yx_scale = value.parse().map_err(|_| (kind, pos))?,
                b"yScale" => transform.y_scale = value.parse().map_err(|_| (kind, pos))?,
                b"xOffset" => transform.x_offset = value.parse().map_err(|_| (kind, pos))?,
                b"yOffset" => transform.y_offset = value.parse().map_err(|_| (kind, pos))?,
                b"color" => color = Some(value.parse().map_err(|e: ErrorKind| e.to_error(pos))?),
                b"fileName" => filename = Some(PathBuf::from(value.to_string())),
                _other => return Err(err!(reader, ErrorKind::UnexpectedImageField)),
            }
        }

        if filename.is_none() {
            return Err(err!(reader, ErrorKind::BadImage));
        }

        self.builder
            .image(Image { file_name: filename.unwrap(), color, transform })
            .map_err(|e| err!(reader, e))?;

        Ok(())
    }
}

fn start(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<GlyphBuilder, Error> {
    loop {
        match reader.read_event(buf)? {
            Event::Comment(_) => (),
            Event::Decl(_decl) => (),
            Event::Start(ref start) if start.name() == b"glyph" => {
                let mut name = String::new();
                let mut format: Option<GlifVersion> = None;
                for attr in start.attributes() {
                    let attr = attr?;
                    // XXX: support `formatMinor`
                    match attr.key {
                        b"name" => {
                            name = attr.unescape_and_decode_value(&reader)?;
                        }
                        b"format" => {
                            let value = attr.unescaped_value()?;
                            let value = reader.decode(&value)?;
                            format =
                                Some(value.parse().map_err(|e: ErrorKind| {
                                    e.to_error(reader.buffer_position())
                                })?);
                        }
                        _other => return Err(err!(reader, ErrorKind::UnexpectedAttribute)),
                    }
                }
                if !name.is_empty() && format.is_some() {
                    return Ok(GlyphBuilder::new(name, format.take().unwrap()));
                } else {
                    return Err(err!(reader, ErrorKind::WrongFirstElement));
                }
            }
            _other => return Err(err!(reader, ErrorKind::WrongFirstElement)),
        }
    }
}

impl FromStr for GlifVersion {
    type Err = ErrorKind;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(GlifVersion::V1),
            "2" => Ok(GlifVersion::V2),
            _other => Err(ErrorKind::UnsupportedGlifVersion),
        }
    }
}

impl FromStr for PointType {
    type Err = ErrorKind;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "move" => Ok(PointType::Move),
            "line" => Ok(PointType::Line),
            "offcurve" => Ok(PointType::OffCurve),
            "curve" => Ok(PointType::Curve),
            "qcurve" => Ok(PointType::QCurve),
            _other => Err(ErrorKind::UnknownPointType),
        }
    }
}
