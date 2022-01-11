use std::collections::HashSet;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::str::FromStr;

use super::*;
use crate::error::{ErrorKind, GlifLoadError};
use crate::glyph::builder::OutlineBuilder;
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
    glyph: Glyph,
    seen_identifiers: HashSet<Identifier>,
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

        start(&mut reader, &mut buf, names).and_then(|glyph| {
            GlifParser { glyph, seen_identifiers: HashSet::new(), names }.parse_body(
                &mut reader,
                xml,
                &mut buf,
            )
        })
    }

    fn parse_body(
        mut self,
        reader: &mut Reader<&[u8]>,
        raw_xml: &[u8],
        buf: &mut Vec<u8>,
    ) -> Result<Glyph, GlifLoadError> {
        let mut seen_advance = false;
        let mut seen_lib = false;
        let mut seen_outline = false;

        loop {
            match reader.read_event(buf)? {
                // outline, lib and note are expected to be start element tags.
                Event::Start(start) => match start.name() {
                    b"outline" if seen_outline => {
                        return Err(ErrorKind::DuplicateElement("outline").into());
                    }
                    b"outline" => {
                        seen_outline = true;
                        self.parse_outline(reader, buf)?
                    }
                    b"lib" if seen_lib => {
                        return Err(ErrorKind::DuplicateElement("lib").into());
                    }
                    b"lib" => {
                        seen_lib = true;
                        self.parse_lib(reader, raw_xml, buf)?
                    }
                    b"note" if self.glyph.format == GlifVersion::V1 => {
                        return Err(ErrorKind::UnexpectedV1Element("note").into());
                    }
                    b"note" if self.glyph.note.is_some() => {
                        return Err(ErrorKind::DuplicateElement("note").into());
                    }
                    b"note" => self.parse_note(reader, buf)?,
                    _other => return Err(ErrorKind::UnexpectedElement.into()),
                },
                // The rest are expected to be empty element tags (exception: outline) with attributes.
                Event::Empty(start) => match start.name() {
                    b"outline" if seen_outline => {
                        return Err(ErrorKind::DuplicateElement("outline").into());
                    }
                    b"outline" => {
                        seen_outline = true;
                    }
                    b"advance" if seen_advance => {
                        return Err(ErrorKind::DuplicateElement("advance").into());
                    }
                    b"advance" => {
                        seen_advance = true;
                        self.parse_advance(reader, start)?
                    }
                    b"unicode" => self.parse_unicode(reader, start)?,
                    b"anchor" if self.glyph.format == GlifVersion::V1 => {
                        return Err(ErrorKind::UnexpectedV1Element("anchor").into());
                    }
                    b"anchor" => self.parse_anchor(reader, start)?,
                    b"guideline" if self.glyph.format == GlifVersion::V1 => {
                        return Err(ErrorKind::UnexpectedV1Element("guideline").into());
                    }
                    b"guideline" => self.parse_guideline(reader, start)?,
                    b"image" if self.glyph.format == GlifVersion::V1 => {
                        return Err(ErrorKind::UnexpectedV1Element("image").into());
                    }
                    b"image" if self.glyph.image.is_some() => {
                        return Err(ErrorKind::DuplicateElement("image").into());
                    }
                    b"image" => self.parse_image(reader, start)?,
                    _other => return Err(ErrorKind::UnexpectedElement.into()),
                },
                Event::End(ref end) if end.name() == b"glyph" => break,
                _other => return Err(ErrorKind::MissingCloseTag.into()),
            }
        }

        self.glyph.load_object_libs()?;
        self.glyph.format = GlifVersion::V2;

        Ok(self.glyph)
    }

    fn parse_outline(
        &mut self,
        reader: &mut Reader<&[u8]>,
        buf: &mut Vec<u8>,
    ) -> Result<(), GlifLoadError> {
        let mut outline_builder = OutlineBuilder::new();

        // TODO: Not checking for (the absence of) attributes here because we'd need to
        // pass through the element data, but that'd clash with the mutable borrow of
        // buf. Better way?

        loop {
            match reader.read_event(buf)? {
                Event::Start(start) => {
                    let mut new_buf = Vec::new(); // borrowck :/
                    match start.name() {
                        b"contour" => {
                            self.parse_contour(start, reader, &mut new_buf, &mut outline_builder)?
                        }
                        _other => return Err(ErrorKind::UnexpectedElement.into()),
                    }
                }
                Event::Empty(start) => {
                    match start.name() {
                        b"contour" => (), // Empty contours are meaningless.
                        b"component" => {
                            self.parse_component(reader, start, &mut outline_builder)?
                        }
                        _other => return Err(ErrorKind::UnexpectedElement.into()),
                    }
                }
                Event::End(ref end) if end.name() == b"outline" => break,
                Event::Eof => return Err(ErrorKind::UnexpectedEof.into()),
                _other => return Err(ErrorKind::UnexpectedElement.into()),
            }
        }

        let (mut contours, components) = outline_builder.finish()?;

        // Upgrade implicit anchors to explicit ones.
        if self.glyph.format == GlifVersion::V1 {
            for c in &mut contours {
                if c.points.len() == 1
                    && c.points[0].typ == PointType::Move
                    && c.points[0].name.is_some()
                {
                    let anchor_point = c.points.remove(0);
                    let anchor = Anchor::new(
                        anchor_point.x,
                        anchor_point.y,
                        anchor_point.name,
                        None,
                        None,
                        None,
                    );
                    self.glyph.anchors.push(anchor);
                }
            }

            // Clean up now empty contours.
            contours.retain(|c| !c.points.is_empty());
        }

        self.glyph.contours.extend(contours);
        self.glyph.components.extend(components);

        Ok(())
    }

    fn parse_identifier(&mut self, value: &str) -> Result<Identifier, GlifLoadError> {
        if self.glyph.format == GlifVersion::V1 {
            return Err(ErrorKind::UnexpectedV1Attribute("identifier").into());
        }

        let id =
            Identifier::new(value).map_err(|_| GlifLoadError::Parse(ErrorKind::BadIdentifier))?;
        if !self.seen_identifiers.insert(id.clone()) {
            return Err(ErrorKind::DuplicateIdentifier.into());
        }
        Ok(id)
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
            if self.glyph.format == GlifVersion::V1 {
                return Err(ErrorKind::UnexpectedAttribute.into());
            }
            let attr = attr?;
            let value = attr.unescaped_value()?;
            let value = reader.decode(&value)?;
            match attr.key {
                b"identifier" => identifier = Some(self.parse_identifier(value)?),
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
        let mut base: Option<Name> = None;
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
                b"base" if value.is_empty() => {
                    return Err(ErrorKind::ComponentEmptyBase.into());
                }
                b"base" => {
                    let name = Name::new(value).map_err(|_| ErrorKind::InvalidName)?;
                    let name = self.names.as_ref().map(|n| n.get(&name)).unwrap_or(name);
                    base = Some(name);
                }
                b"identifier" => {
                    identifier = Some(self.parse_identifier(value)?);
                }
                _other => return Err(ErrorKind::UnexpectedComponentField.into()),
            }
        }

        match base {
            Some(base) => {
                outline_builder.add_component(base, transform, identifier);
                Ok(())
            }
            None => Err(ErrorKind::ComponentMissingBase.into()),
        }
    }

    fn parse_lib(
        &mut self,
        reader: &mut Reader<&[u8]>,
        raw_xml: &[u8],
        buf: &mut Vec<u8>,
    ) -> Result<(), GlifLoadError> {
        // The plist crate currently uses a different XML parsing library internally, so
        // we can't pass over control to it directly. Instead, pass it the precise slice
        // of the raw buffer to parse.
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
            .map_err(|_| GlifLoadError::Parse(ErrorKind::BadLib))?
            .into_dictionary()
            .ok_or(GlifLoadError::Parse(ErrorKind::LibMustBeDictionary))?;

        self.glyph.lib = dict;
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
                    self.glyph.note = Some(text.unescape_and_decode(reader)?);
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
                    identifier = Some(self.parse_identifier(value)?);
                }
                _other => return Err(ErrorKind::UnexpectedPointField.into()),
            }
        }

        match (x, y) {
            (Some(x), Some(y)) => {
                outline_builder.add_point((x, y), typ, smooth, name, identifier)?;
                Ok(())
            }
            _ => Err(ErrorKind::BadPoint.into()),
        }
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

        self.glyph.width = width;
        self.glyph.height = height;
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
                    self.glyph.codepoints.push(chr);
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
                    identifier = Some(self.parse_identifier(value)?);
                }
                _other => return Err(ErrorKind::UnexpectedAnchorField.into()),
            }
        }

        match (x, y) {
            (Some(x), Some(y)) => {
                self.glyph.anchors.push(Anchor::new(x, y, name, color, identifier, None));
                Ok(())
            }
            _ => Err(ErrorKind::BadAnchor.into()),
        }
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
                    let angle_value = value.parse().map_err(|_| ErrorKind::BadNumber)?;
                    if !(0.0..=360.0).contains(&angle_value) {
                        return Err(ErrorKind::BadAngle.into());
                    }
                    angle = Some(angle_value);
                }
                b"name" => name = Some(value.to_string()),
                b"color" => color = Some(value.parse().map_err(|_| ErrorKind::BadColor)?),
                b"identifier" => {
                    identifier = Some(self.parse_identifier(value)?);
                }
                _other => return Err(ErrorKind::UnexpectedGuidelineField.into()),
            }
        }

        let line = match (x, y, angle) {
            (Some(x), None, None) => Line::Vertical(x),
            (None, Some(y), None) => Line::Horizontal(y),
            (Some(x), Some(y), Some(degrees)) => Line::Angle { x, y, degrees },
            _ => return Err(ErrorKind::BadGuideline.into()),
        };
        self.glyph.guidelines.push(Guideline::new(line, name, color, identifier, None));

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

        match filename {
            Some(file_name) => {
                self.glyph.image = Some(
                    Image::new(file_name, color, transform)
                        .map_err(|_| GlifLoadError::Parse(ErrorKind::BadImage))?,
                );
                Ok(())
            }
            None => Err(ErrorKind::BadImage.into()),
        }
    }
}

fn start(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    names: Option<&NameList>,
) -> Result<Glyph, GlifLoadError> {
    loop {
        match reader.read_event(buf)? {
            Event::Comment(_) => (),
            Event::Decl(_decl) => (),
            Event::Start(ref start) if start.name() == b"glyph" => {
                let mut name: Option<Name> = None;
                let mut format: Option<GlifVersion> = None;
                //let mut pos
                for attr in start.attributes() {
                    let attr = attr?;
                    let value = attr.unescaped_value()?;
                    let value = reader.decode(&value)?;
                    // XXX: support `formatMinor`
                    match attr.key {
                        b"name" => {
                            let value = Name::new(value).map_err(|_| ErrorKind::InvalidName)?;
                            name = Some(names.as_ref().map(|n| n.get(&value)).unwrap_or(value));
                        }
                        b"format" => {
                            format = Some(value.parse()?);
                        }
                        _other => return Err(ErrorKind::UnexpectedAttribute.into()),
                    }
                }
                if let (Some(name), Some(format)) = (name.take(), format.take()) {
                    return Ok(Glyph::new(name, format));
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
