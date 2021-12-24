use std::borrow::Borrow;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use super::*;
use crate::error::{ErrorKind, GlifLoadError};
use crate::glyph::builder::{GlyphBuilder, Outline, OutlineBuilder};
use crate::names::NameList;

use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

#[cfg(test)]
pub(crate) fn parse_glyph(xml: &[u8]) -> Result<Glyph, GlifLoadError> {
    GlifParser::from_xml(xml, None)
}

pub(crate) struct GlifParser<'names> {
    builder: GlyphBuilder,
    /// Optional set of glyph names to be reused between glyphs.
    names: Option<&'names NameList>,
}

impl<'names> GlifParser<'names> {
    pub(crate) fn from_xml(
        xml: &[u8],
        names: Option<&'names NameList>,
    ) -> Result<Glyph, GlifLoadError> {
        let mut reader = Reader::from_reader(xml);
        let mut buf = Vec::new();
        reader.trim_text(true);

        start(&mut reader, &mut buf).and_then(|builder| {
            GlifParser { builder, names }.parse_body(&mut reader, xml, &mut buf)
        })
    }

    fn parse_body(
        mut self,
        reader: &mut Reader<&[u8]>,
        raw_xml: &[u8],
        buf: &mut Vec<u8>,
    ) -> Result<Glyph, GlifLoadError> {
        loop {
            match reader.read_event(buf)? {
                // outline, lib and note are expected to be start element tags.
                Event::Start(start) => {
                    let tag_name = reader.decode(start.name())?;
                    match tag_name.borrow() {
                        "outline" => self.parse_outline(reader, buf)?,
                        "lib" => self.parse_lib(reader, raw_xml, buf)?,
                        "note" => self.parse_note(reader, buf)?,
                        _other => return Err(ErrorKind::UnexpectedTag.into()),
                    }
                }
                // The rest are expected to be empty element tags (exception: outline) with attributes.
                Event::Empty(start) => {
                    let tag_name = reader.decode(start.name())?;
                    match tag_name.borrow() {
                        "outline" => {
                            // ufoLib parses `<outline/>` as an empty outline.
                            self.builder.outline(Outline::default(), HashSet::new())?;
                        }
                        "advance" => self.parse_advance(reader, start)?,
                        "unicode" => self.parse_unicode(reader, start)?,
                        "anchor" => self.parse_anchor(reader, start)?,
                        "guideline" => self.parse_guideline(reader, start)?,
                        "image" => self.parse_image(reader, start)?,
                        _other => return Err(ErrorKind::UnexpectedTag.into()),
                    }
                }
                Event::End(ref end) if end.name() == b"glyph" => break,
                _other => return Err(ErrorKind::MissingCloseTag.into()),
            }
        }

        let mut glyph = self.builder.finish()?;
        glyph.load_object_libs()?;

        Ok(glyph)
    }

    fn parse_outline(
        &mut self,
        reader: &mut Reader<&[u8]>,
        buf: &mut Vec<u8>,
    ) -> Result<(), GlifLoadError> {
        let mut outline_builder = OutlineBuilder::new();

        loop {
            match reader.read_event(buf)? {
                Event::Start(start) => {
                    let tag_name = reader.decode(start.name())?;
                    let mut new_buf = Vec::new(); // borrowck :/
                    match tag_name.borrow() {
                        "contour" => {
                            self.parse_contour(start, reader, &mut new_buf, &mut outline_builder)?
                        }
                        _other => return Err(ErrorKind::UnexpectedTag.into()),
                    }
                }
                Event::Empty(start) => {
                    let tag_name = reader.decode(start.name())?;
                    match tag_name.borrow() {
                        // Skip empty contours as meaningless.
                        // https://github.com/unified-font-object/ufo-spec/issues/150
                        "contour" => (),
                        "component" => self.parse_component(reader, start, &mut outline_builder)?,
                        _other => return Err(ErrorKind::UnexpectedTag.into()),
                    }
                }
                Event::End(ref end) if end.name() == b"outline" => break,
                Event::Eof => return Err(ErrorKind::UnexpectedEof.into()),
                _other => return Err(ErrorKind::UnexpectedElement.into()),
            }
        }

        let (outline, identifiers) = outline_builder.finish()?;
        self.builder.outline(outline, identifiers)?;

        Ok(())
    }

    fn parse_contour(
        &mut self,
        data: BytesStart,
        reader: &mut Reader<&[u8]>,
        buf: &mut Vec<u8>,
        outline_builder: &mut OutlineBuilder,
    ) -> Result<(), GlifLoadError> {
        let mut identifier = None;
        for attr in data.attributes() {
            if self.builder.get_format() == &GlifVersion::V1 {
                return Err(ErrorKind::UnexpectedAttribute.into());
            }
            let attr = attr?;
            match attr.key {
                b"identifier" => {
                    let ident = attr.unescape_and_decode_value(reader)?;
                    identifier = Some(Identifier::new(ident)?);
                }
                _other => return Err(ErrorKind::UnexpectedAttribute.into()),
            }
        }

        outline_builder.begin_path(identifier)?;
        loop {
            match reader.read_event(buf)? {
                Event::End(ref end) if end.name() == b"contour" => break,
                Event::Empty(ref start) if start.name() == b"point" => {
                    self.parse_point(reader, start, outline_builder)?;
                }
                Event::Eof => return Err(ErrorKind::UnexpectedEof.into()),
                _other => return Err(ErrorKind::UnexpectedElement.into()),
            }
        }
        outline_builder.end_path()?;

        Ok(())
    }

    fn parse_component(
        &mut self,
        reader: &mut Reader<&[u8]>,
        start: BytesStart,
        outline_builder: &mut OutlineBuilder,
    ) -> Result<(), GlifLoadError> {
        let mut base: Option<GlyphName> = None;
        let mut identifier: Option<Identifier> = None;
        let mut transform = AffineTransform::default();

        for attr in start.attributes() {
            let attr = attr?;
            let value = attr.unescaped_value()?;
            let value = reader.decode(&value)?;
            let kind = ErrorKind::BadNumber;
            match attr.key {
                b"xScale" => transform.x_scale = value.parse().map_err(|_| kind)?,
                b"xyScale" => transform.xy_scale = value.parse().map_err(|_| kind)?,
                b"yxScale" => transform.yx_scale = value.parse().map_err(|_| kind)?,
                b"yScale" => transform.y_scale = value.parse().map_err(|_| kind)?,
                b"xOffset" => transform.x_offset = value.parse().map_err(|_| kind)?,
                b"yOffset" => transform.y_offset = value.parse().map_err(|_| kind)?,
                b"base" => {
                    let name: Arc<str> = value.into();
                    let name = match self.names.as_ref() {
                        Some(names) => names.get(&name),
                        None => name,
                    };
                    base = Some(name);
                }
                b"identifier" => {
                    identifier = Some(value.parse()?);
                }
                _other => return Err(ErrorKind::UnexpectedComponentField.into()),
            }
        }

        if base.is_none() {
            return Err(ErrorKind::BadComponent.into());
        }

        outline_builder.add_component(base.unwrap(), transform, identifier)?;
        Ok(())
    }

    fn parse_lib(
        &mut self,
        reader: &mut Reader<&[u8]>,
        raw_xml: &[u8],
        buf: &mut Vec<u8>,
    ) -> Result<(), GlifLoadError> {
        let start = reader.buffer_position();
        let mut end = start;
        loop {
            match reader.read_event(buf)? {
                Event::End(ref end) if end.name() == b"lib" => break,
                Event::Eof => return Err(ErrorKind::UnexpectedEof.into()),
                _other => end = reader.buffer_position(),
            }
        }

        let plist_slice = &raw_xml[start..end];
        let dict = plist::Value::from_reader_xml(plist_slice)
            .ok()
            .and_then(plist::Value::into_dictionary)
            .ok_or(ErrorKind::BadLib)?;
        self.builder.lib(dict)?;
        Ok(())
    }

    fn parse_note(
        &mut self,
        reader: &mut Reader<&[u8]>,
        buf: &mut Vec<u8>,
    ) -> Result<(), GlifLoadError> {
        loop {
            match reader.read_event(buf)? {
                Event::End(ref end) if end.name() == b"note" => break,
                Event::Text(text) => {
                    self.builder.note(text.unescape_and_decode(reader)?)?;
                }
                Event::Eof => return Err(ErrorKind::UnexpectedEof.into()),
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
    ) -> Result<(), GlifLoadError> {
        let mut name: Option<String> = None;
        let mut x: Option<f64> = None;
        let mut y: Option<f64> = None;
        let mut typ = PointType::OffCurve;
        let mut identifier: Option<Identifier> = None;
        let mut smooth = false;

        for attr in data.attributes() {
            let attr = attr?;
            let value = attr.unescaped_value()?;
            let value = reader.decode(&value)?;
            match attr.key {
                b"x" => {
                    x = Some(value.parse().map_err(|_| ErrorKind::BadNumber)?);
                }
                b"y" => {
                    y = Some(value.parse().map_err(|_| ErrorKind::BadNumber)?);
                }
                b"name" => name = Some(value.to_string()),
                b"type" => {
                    typ = value.parse()?;
                }
                b"smooth" => smooth = value == "yes",
                b"identifier" => {
                    identifier = Some(value.parse()?);
                }
                _other => return Err(ErrorKind::UnexpectedPointField.into()),
            }
        }
        if x.is_none() || y.is_none() {
            return Err(ErrorKind::BadPoint.into());
        }
        outline_builder.add_point((x.unwrap(), y.unwrap()), typ, smooth, name, identifier)?;

        Ok(())
    }

    fn parse_advance<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), GlifLoadError> {
        let mut width: f64 = 0.0;
        let mut height: f64 = 0.0;
        for attr in data.attributes() {
            let attr = attr?;
            match attr.key {
                b"width" | b"height" => {
                    let value = attr.unescaped_value()?;
                    let value = reader.decode(&value)?;
                    let value: f64 = value.parse().map_err(|_| ErrorKind::BadNumber)?;
                    match attr.key {
                        b"width" => width = value,
                        b"height" => height = value,
                        _other => unreachable!(),
                    };
                }
                _other => return Err(ErrorKind::UnexpectedAttribute.into()),
            }
        }
        self.builder.width(width)?.height(height)?;
        Ok(())
    }

    fn parse_unicode<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), GlifLoadError> {
        for attr in data.attributes() {
            let attr = attr?;
            match attr.key {
                b"hex" => {
                    let value = attr.unescaped_value()?;
                    let value = reader.decode(&value)?;
                    let chr = u32::from_str_radix(value, 16)
                        .map_err(|_| value.to_string())
                        .and_then(|n| char::try_from(n).map_err(|_| value.to_string()))
                        .map_err(|_| ErrorKind::BadHexValue)?;
                    self.builder.unicode(chr);
                }
                _other => return Err(ErrorKind::UnexpectedAttribute.into()),
            }
        }
        Ok(())
    }

    fn parse_anchor<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), GlifLoadError> {
        let mut x: Option<f64> = None;
        let mut y: Option<f64> = None;
        let mut name: Option<String> = None;
        let mut color: Option<Color> = None;
        let mut identifier: Option<Identifier> = None;

        for attr in data.attributes() {
            let attr = attr?;
            let value = attr.unescaped_value()?;
            let value = reader.decode(&value)?;
            match attr.key {
                b"x" => {
                    x = Some(value.parse().map_err(|_| ErrorKind::BadNumber)?);
                }
                b"y" => {
                    y = Some(value.parse().map_err(|_| ErrorKind::BadNumber)?);
                }
                b"name" => name = Some(value.to_string()),
                b"color" => color = Some(value.parse().map_err(|_| ErrorKind::BadColor)?),
                b"identifier" => {
                    identifier = Some(value.parse()?);
                }
                _other => return Err(ErrorKind::UnexpectedAnchorField.into()),
            }
        }

        if x.is_none() || y.is_none() {
            return Err(ErrorKind::BadAnchor.into());
        }
        self.builder.anchor(Anchor::new(x.unwrap(), y.unwrap(), name, color, identifier, None))?;
        Ok(())
    }

    fn parse_guideline<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), GlifLoadError> {
        let mut x: Option<f64> = None;
        let mut y: Option<f64> = None;
        let mut angle: Option<f64> = None;
        let mut name: Option<String> = None;
        let mut color: Option<Color> = None;
        let mut identifier: Option<Identifier> = None;

        for attr in data.attributes() {
            let attr = attr?;
            let value = attr.unescaped_value()?;
            let value = reader.decode(&value)?;
            match attr.key {
                b"x" => {
                    x = Some(value.parse().map_err(|_| ErrorKind::BadNumber)?);
                }
                b"y" => {
                    y = Some(value.parse().map_err(|_| ErrorKind::BadNumber)?);
                }
                b"angle" => {
                    angle = Some(value.parse().map_err(|_| ErrorKind::BadNumber)?);
                }
                b"name" => name = Some(value.to_string()),
                b"color" => color = Some(value.parse().map_err(|_| ErrorKind::BadColor)?),
                b"identifier" => {
                    identifier = Some(value.parse()?);
                }
                _other => return Err(ErrorKind::UnexpectedGuidelineField.into()),
            }
        }

        let line = match (x, y, angle) {
            (Some(x), None, None) => Line::Vertical(x),
            (None, Some(y), None) => Line::Horizontal(y),
            (Some(x), Some(y), Some(degrees)) => {
                if !(0.0..=360.0).contains(&degrees) {
                    return Err(ErrorKind::BadGuideline.into());
                }
                Line::Angle { x, y, degrees }
            }
            _other => return Err(ErrorKind::BadGuideline.into()),
        };
        self.builder.guideline(Guideline::new(line, name, color, identifier, None))?;

        Ok(())
    }

    fn parse_image<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), GlifLoadError> {
        let mut filename: Option<PathBuf> = None;
        let mut color: Option<Color> = None;
        let mut transform = AffineTransform::default();

        for attr in data.attributes() {
            let attr = attr?;
            let value = attr.unescaped_value()?;
            let value = reader.decode(&value)?;
            let kind = ErrorKind::BadNumber;
            match attr.key {
                b"xScale" => transform.x_scale = value.parse().map_err(|_| kind)?,
                b"xyScale" => transform.xy_scale = value.parse().map_err(|_| kind)?,
                b"yxScale" => transform.yx_scale = value.parse().map_err(|_| kind)?,
                b"yScale" => transform.y_scale = value.parse().map_err(|_| kind)?,
                b"xOffset" => transform.x_offset = value.parse().map_err(|_| kind)?,
                b"yOffset" => transform.y_offset = value.parse().map_err(|_| kind)?,
                b"color" => color = Some(value.parse().map_err(|_| ErrorKind::BadColor)?),
                b"fileName" => filename = Some(PathBuf::from(value.to_string())),
                _other => return Err(ErrorKind::UnexpectedImageField.into()),
            }
        }

        if filename.is_none() {
            return Err(ErrorKind::BadImage.into());
        }

        self.builder.image(Image { file_name: filename.unwrap(), color, transform })?;

        Ok(())
    }
}

fn start(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<GlyphBuilder, GlifLoadError> {
    loop {
        match reader.read_event(buf)? {
            Event::Comment(_) => (),
            Event::Decl(_decl) => (),
            Event::Start(ref start) if start.name() == b"glyph" => {
                let mut name = String::new();
                let mut format: Option<GlifVersion> = None;
                //let mut pos
                for attr in start.attributes() {
                    let attr = attr?;
                    // XXX: support `formatMinor`
                    match attr.key {
                        b"name" => {
                            name = attr.unescape_and_decode_value(reader)?;
                        }
                        b"format" => {
                            let value = attr.unescaped_value()?;
                            let value = reader.decode(&value)?;
                            format = Some(value.parse()?);
                        }
                        _other => return Err(ErrorKind::UnexpectedAttribute.into()),
                    }
                }
                if !name.is_empty() && format.is_some() {
                    return Ok(GlyphBuilder::new(name, format.take().unwrap()));
                } else {
                    return Err(ErrorKind::WrongFirstElement.into());
                }
            }
            _other => return Err(ErrorKind::WrongFirstElement.into()),
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
