use std::collections::HashSet;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::sync::Arc;

use super::*;
use crate::error::InvalidColorString;
use crate::glyph::builder2::{OutlineBuilder, OutlineBuilderError};
use crate::names::NameList;

use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

pub(crate) struct GlifParser<'names> {
    glyph: Glyph,
    seen_identifiers: HashSet<Identifier>,
    /// Optional set of glyph names to be reused between glyphs.
    names: Option<&'names NameList>,
}

/// An error that occurs while attempting to read a .glif file.
///
/// No reliable line number information is available as of quick_xml 0.22.0.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GlifParserError {
    /// ...
    #[error("failed to parse hexadecimal Unicode code point value '{0}'")]
    BadUnicodeValue(String),
    /// ...
    #[error("a 'component' element has an empty 'base' attribute")]
    ComponentEmptyBase,
    /// ...
    #[error("a 'component' element is missing a 'base' attribute")]
    ComponentMissingBase,
    /// ...
    #[error("failed to draw glyph")]
    Draw(#[source] OutlineBuilderError),
    /// ...
    #[error("there must be only one '{0}' element")]
    DuplicateElement(String),
    /// Found a duplicate identifier while parsing glyph data.
    #[error("duplicate identifier '{0}'")]
    DuplicateIdentifier(String),
    /// ...
    #[error("the 'image' element is missing a 'fileName' attribute")]
    ImageMissingFilename,
    /// ...
    #[error("invalid advance for '{0}': {1}")]
    InvalidAdvance(String, std::num::ParseFloatError),
    /// ...
    #[error("an anchor needs at least an 'x' and 'y' attribute")]
    InvalidAnchor,
    /// ...
    #[error("failed to parse angle '{0}': {1}")]
    InvalidAngle(String, std::num::ParseFloatError),
    /// ...
    #[error("angle must be between 0 and 360°, inclusive")]
    InvalidAngleBounds,
    /// ...
    #[error("invalid color '{0}'")]
    InvalidColor(String),
    /// ...
    #[error("failed to parse component transformation value '{0}': {1}")]
    InvalidComponentTransformation(String, std::num::ParseFloatError),
    /// ...
    #[error("failed to parse point coordinate '{0}': {1}")]
    InvalidCoordinate(String, std::num::ParseFloatError),
    /// ...
    #[error("a guideline must have either 'x' or 'y' or 'x' and 'y' and 'angle' attributes")]
    InvalidGuideline,
    /// ...
    #[error("invalid identifier '{0}'")]
    InvalidIdentifier(String),
    /// ...
    #[error("failed to parse image color")]
    InvalidImageColor(#[source] InvalidColorString),
    /// ...
    #[error("failed to parse image transformation value '{0}': {1}")]
    InvalidImageTransformation(String, std::num::ParseFloatError),
    /// ...
    #[error("a point needs at least an 'x' and 'y' attribute")]
    InvalidPoint,
    /// ...
    #[error("the glyph lib must be a dictionary")]
    LibMustBeDictionary,
    /// ...
    #[error("missing the closing tag for element '{0}'")]
    MissingCloseTag(&'static str),
    /// ...
    #[error("the glyph lib's 'public.objectLibs' entry for the object with identifier '{0}' must be a dictionary")]
    ObjectLibMustBeDictionary(String),
    /// ...
    #[error("failed to parse glyph lib data")]
    ParseLib(#[source] plist::Error),
    /// ...
    #[error("the glyph lib's 'public.objectLibs' value must be a dictionary")]
    PublicObjectLibsMustBeDictionary,
    /// ...
    #[error("unexpected '{0}' element attribute '{1}'")]
    UnexpectedAttribute(&'static str, String),
    /// ...
    #[error("unrecognized element '{1}' inside '{0}' parent element")]
    UnexpectedElement(&'static str, String),
    /// ...
    #[error("unexpected end of file")]
    UnexpectedEof,
    /// ...
    #[error("format 1 does not support attributes for element '{0}'")]
    UnexpectedV1Attributes(String),
    /// ...
    #[error("format 1 does not support the '{0}' element")]
    UnexpectedV1Element(&'static str),
    /// ...
    #[error("format 1 does not support identifiers")]
    UnexpectedV1Identifier,
    /// ...
    #[error("unrecognized point type '{0}'")]
    UnknownPointType(String),
    /// ...
    #[error("unsupported glif format version '{0}'")]
    UnsupportedGlifVersion(String),
    /// ...
    #[error("the first element must be a 'glyph' with at least a 'name' and 'format' attribute")]
    WrongFirstElement,
    /// A [`quick_xml::Error`].
    #[error("failed to read or parse XML structure")]
    Xml(#[source] quick_xml::Error),
}

impl<'names> GlifParser<'names> {
    pub(crate) fn from_xml(
        xml: &[u8],
        names: Option<&'names NameList>,
    ) -> Result<Glyph, GlifParserError> {
        let mut reader = Reader::from_reader(xml);
        let mut buf = Vec::new();
        reader.trim_text(true);

        let glyph = Self::parse_first_element(&mut reader, &mut buf)?;
        let parser = GlifParser { glyph, seen_identifiers: HashSet::new(), names };
        parser.parse_body(&mut reader, xml, &mut buf)
    }

    fn parse_first_element(
        reader: &mut Reader<&[u8]>,
        buf: &mut Vec<u8>,
    ) -> Result<Glyph, GlifParserError> {
        loop {
            match reader.read_event(buf).map_err(GlifParserError::Xml)? {
                Event::Comment(_) => (),
                Event::Decl(_decl) => (),
                Event::Start(ref start) if start.name() == b"glyph" => {
                    let mut name = String::new();
                    let mut format: Option<GlifVersion> = None;
                    for attr in start.attributes() {
                        let attr = attr.map_err(GlifParserError::Xml)?;
                        let value = attr.unescaped_value().map_err(GlifParserError::Xml)?;
                        let value = reader.decode(&value).map_err(GlifParserError::Xml)?;
                        match attr.key {
                            b"name" => name = value.into(),
                            b"format" => {
                                format = Some(Self::parse_format(value)?);
                            }
                            b"formatMinor" => (),
                            _ => {
                                return Err(GlifParserError::UnexpectedAttribute(
                                    "glyph",
                                    b2s(attr.key),
                                ))
                            }
                        }
                    }
                    if !name.is_empty() && format.is_some() {
                        return Ok(Glyph::new(name.into(), format.take().unwrap()));
                    }
                    return Err(GlifParserError::WrongFirstElement);
                }
                Event::Eof => return Err(GlifParserError::UnexpectedEof),
                _ => return Err(GlifParserError::WrongFirstElement),
            }
        }
    }

    fn parse_format(value: &str) -> Result<GlifVersion, GlifParserError> {
        match value {
            "1" => Ok(GlifVersion::V1),
            "2" => Ok(GlifVersion::V2),
            _ => Err(GlifParserError::UnsupportedGlifVersion(value.into())),
        }
    }

    fn parse_body(
        mut self,
        reader: &mut Reader<&[u8]>,
        raw_xml: &[u8],
        buf: &mut Vec<u8>,
    ) -> Result<Glyph, GlifParserError> {
        let mut seen_advance = false;
        let mut seen_image = false;
        let mut seen_lib = false;
        let mut seen_note = false;
        let mut seen_outline = false;

        loop {
            match reader.read_event(buf).map_err(GlifParserError::Xml)? {
                // outline, lib and note are expected to be start element tags.
                Event::Start(e) => match e.name() {
                    b"outline" => {
                        if seen_outline {
                            return Err(GlifParserError::DuplicateElement(b2s(e.name())));
                        }
                        seen_outline = true;
                        self.parse_outline(reader, buf)?
                    }
                    b"lib" => {
                        if seen_lib {
                            return Err(GlifParserError::DuplicateElement(b2s(e.name())));
                        }
                        seen_lib = true;
                        self.parse_lib(reader, raw_xml, buf)?
                    }
                    b"note" => {
                        if self.glyph.format == GlifVersion::V1 {
                            return Err(GlifParserError::UnexpectedV1Element("note"));
                        }
                        if seen_note {
                            return Err(GlifParserError::DuplicateElement(b2s(e.name())));
                        }
                        seen_note = true;
                        self.parse_note(reader, buf)?
                    }
                    _ => return Err(GlifParserError::UnexpectedElement("glyph", b2s(e.name()))),
                },
                // The rest are expected to be empty element tags (exception: outline) with attributes.
                Event::Empty(e) => match e.name() {
                    b"outline" => {
                        if seen_outline {
                            return Err(GlifParserError::DuplicateElement(b2s(e.name())));
                        }
                        seen_outline = true;
                    }
                    b"advance" => {
                        if seen_advance {
                            return Err(GlifParserError::DuplicateElement(b2s(e.name())));
                        }
                        seen_advance = true;
                        self.parse_advance(reader, e)?
                    }
                    b"unicode" => self.parse_unicode(reader, e)?,
                    b"anchor" => {
                        if self.glyph.format == GlifVersion::V1 {
                            return Err(GlifParserError::UnexpectedV1Element("anchor"));
                        }
                        self.parse_anchor(reader, e)?
                    }
                    b"guideline" => {
                        if self.glyph.format == GlifVersion::V1 {
                            return Err(GlifParserError::UnexpectedV1Element("guideline"));
                        }
                        self.parse_guideline(reader, e)?
                    }
                    b"image" => {
                        if self.glyph.format == GlifVersion::V1 {
                            return Err(GlifParserError::UnexpectedV1Element("image"));
                        }
                        if seen_image {
                            return Err(GlifParserError::DuplicateElement(b2s(e.name())));
                        }
                        seen_image = true;
                        self.parse_image(reader, e)?
                    }
                    _ => return Err(GlifParserError::UnexpectedElement("glyph", b2s(e.name()))),
                },
                Event::End(ref end) if end.name() == b"glyph" => break,
                Event::Eof => return Err(GlifParserError::UnexpectedEof),
                _ => return Err(GlifParserError::MissingCloseTag("glyph")),
            }
        }

        move_object_libs(&mut self.glyph)?;

        Ok(self.glyph)
    }

    fn parse_outline(
        &mut self,
        reader: &mut Reader<&[u8]>,
        buf: &mut Vec<u8>,
    ) -> Result<(), GlifParserError> {
        let mut outline_builder = OutlineBuilder::new();

        // TODO: Not checking for attributes here because we'd need to pass through the
        // element data, but that'd clash with the mutable borrow of buf. Better way?

        loop {
            match reader.read_event(buf).map_err(GlifParserError::Xml)? {
                Event::Start(start) => {
                    let mut new_buf = Vec::new(); // borrowck :/
                    match start.name() {
                        b"contour" => {
                            self.parse_contour(start, reader, &mut new_buf, &mut outline_builder)?
                        }
                        _ => {
                            return Err(GlifParserError::UnexpectedElement(
                                "outline",
                                b2s(start.name()),
                            ))
                        }
                    }
                }
                Event::Empty(start) => {
                    match start.name() {
                        b"contour" => (), // Skip empty contours as meaningless.
                        b"component" => {
                            self.parse_component(reader, start, &mut outline_builder)?
                        }
                        _ => {
                            return Err(GlifParserError::UnexpectedElement(
                                "outline",
                                b2s(start.name()),
                            ))
                        }
                    }
                }
                Event::End(ref end) if end.name() == b"outline" => break,
                Event::Eof => return Err(GlifParserError::UnexpectedEof),
                _ => return Err(GlifParserError::MissingCloseTag("outline")),
            }
        }

        let (mut contours, components) = outline_builder.finish().map_err(GlifParserError::Draw)?;

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

    fn parse_contour(
        &mut self,
        data: BytesStart,
        reader: &mut Reader<&[u8]>,
        buf: &mut Vec<u8>,
        outline_builder: &mut OutlineBuilder,
    ) -> Result<(), GlifParserError> {
        let mut identifier = None;

        for attr in data.attributes() {
            if self.glyph.format == GlifVersion::V1 {
                return Err(GlifParserError::UnexpectedV1Attributes("contour".into()));
            }
            let attr = attr.map_err(GlifParserError::Xml)?;
            let value = attr.unescaped_value().map_err(GlifParserError::Xml)?;
            let value = reader.decode(&value).map_err(GlifParserError::Xml)?;
            match attr.key {
                b"identifier" => identifier = Some(self.parse_identifier(&value)?),
                _ => return Err(GlifParserError::UnexpectedAttribute("contour", b2s(attr.key))),
            }
        }

        outline_builder.begin_path(identifier).map_err(GlifParserError::Draw)?;
        loop {
            match reader.read_event(buf).map_err(GlifParserError::Xml)? {
                Event::End(ref end) if end.name() == b"contour" => break,
                Event::Empty(ref start) if start.name() == b"point" => {
                    self.parse_point(reader, start, outline_builder)?;
                }
                Event::Eof => return Err(GlifParserError::UnexpectedEof),
                _ => return Err(GlifParserError::MissingCloseTag("contour")),
            }
        }
        outline_builder.end_path().map_err(GlifParserError::Draw)?;

        Ok(())
    }

    fn parse_identifier(&mut self, value: &str) -> Result<Identifier, GlifParserError> {
        if self.glyph.format == GlifVersion::V1 {
            return Err(GlifParserError::UnexpectedV1Identifier);
        }

        let id =
            Identifier::new(value).map_err(|_| GlifParserError::InvalidIdentifier(value.into()))?;
        if !self.seen_identifiers.insert(id.clone()) {
            return Err(GlifParserError::DuplicateIdentifier(value.into()));
        }
        Ok(id)
    }

    fn parse_component(
        &mut self,
        reader: &mut Reader<&[u8]>,
        start: BytesStart,
        outline_builder: &mut OutlineBuilder,
    ) -> Result<(), GlifParserError> {
        let mut base: Option<GlyphName> = None;
        let mut identifier: Option<Identifier> = None;
        let mut transform = AffineTransform::default();

        for attr in start.attributes() {
            let attr = attr.map_err(GlifParserError::Xml)?;
            let value = attr.unescaped_value().map_err(GlifParserError::Xml)?;
            let value = reader.decode(&value).map_err(GlifParserError::Xml)?;
            let bad_transform =
                |e| GlifParserError::InvalidComponentTransformation(value.into(), e);
            match attr.key {
                b"xScale" => transform.x_scale = value.parse().map_err(bad_transform)?,
                b"xyScale" => transform.xy_scale = value.parse().map_err(bad_transform)?,
                b"yxScale" => transform.yx_scale = value.parse().map_err(bad_transform)?,
                b"yScale" => transform.y_scale = value.parse().map_err(bad_transform)?,
                b"xOffset" => transform.x_offset = value.parse().map_err(bad_transform)?,
                b"yOffset" => transform.y_offset = value.parse().map_err(bad_transform)?,
                b"base" => {
                    if value.is_empty() {
                        return Err(GlifParserError::ComponentEmptyBase);
                    }
                    let name: Arc<str> = value.into();
                    let name = match self.names.as_ref() {
                        Some(names) => names.get(&name),
                        None => name,
                    };
                    base = Some(name);
                }
                b"identifier" => {
                    identifier = Some(self.parse_identifier(&value)?);
                }
                _ => return Err(GlifParserError::UnexpectedAttribute("component", b2s(attr.key))),
            }
        }

        match base {
            Some(base) => {
                outline_builder.add_component(base, transform, identifier);
                Ok(())
            }
            None => return Err(GlifParserError::ComponentMissingBase),
        }
    }

    fn parse_lib(
        &mut self,
        reader: &mut Reader<&[u8]>,
        raw_xml: &[u8],
        buf: &mut Vec<u8>,
    ) -> Result<(), GlifParserError> {
        // The plist crate currently uses a different XML parsing library internally, so
        // we can't pass over control to it directly. Instead, pass it the precise slice
        // of the raw buffer to parse.
        let start = reader.buffer_position();
        let mut end = start;
        loop {
            match reader.read_event(buf).map_err(GlifParserError::Xml)? {
                Event::End(ref end) if end.name() == b"lib" => break,
                Event::Eof => return Err(GlifParserError::UnexpectedEof),
                _ => end = reader.buffer_position(),
            }
        }
        let plist_slice = &raw_xml[start..end];
        let dict = plist::Value::from_reader_xml(plist_slice)
            .map_err(GlifParserError::ParseLib)?
            .into_dictionary()
            .ok_or(GlifParserError::LibMustBeDictionary)?;

        self.glyph.lib = dict;

        Ok(())
    }

    fn parse_note(
        &mut self,
        reader: &mut Reader<&[u8]>,
        buf: &mut Vec<u8>,
    ) -> Result<(), GlifParserError> {
        loop {
            match reader.read_event(buf).map_err(GlifParserError::Xml)? {
                Event::End(ref end) if end.name() == b"note" => break,
                Event::Text(text) => {
                    self.glyph.note =
                        Some(text.unescape_and_decode(reader).map_err(GlifParserError::Xml)?);
                }
                Event::Eof => return Err(GlifParserError::UnexpectedEof),
                _ => (),
            }
        }
        Ok(())
    }

    fn parse_point<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: &BytesStart<'a>,
        outline_builder: &mut OutlineBuilder,
    ) -> Result<(), GlifParserError> {
        let mut name: Option<String> = None;
        let mut x: Option<f64> = None;
        let mut y: Option<f64> = None;
        let mut typ = PointType::OffCurve;
        let mut identifier: Option<Identifier> = None;
        let mut smooth = false;

        for attr in data.attributes() {
            let attr = attr.map_err(GlifParserError::Xml)?;
            let value = attr.unescaped_value().map_err(GlifParserError::Xml)?;
            let value = reader.decode(&value).map_err(GlifParserError::Xml)?;
            match attr.key {
                b"x" => {
                    x = Some(
                        value
                            .parse()
                            .map_err(|e| GlifParserError::InvalidCoordinate(value.into(), e))?,
                    );
                }
                b"y" => {
                    y = Some(
                        value
                            .parse()
                            .map_err(|e| GlifParserError::InvalidCoordinate(value.into(), e))?,
                    );
                }
                b"name" => name = Some(value.to_string()),
                b"type" => {
                    typ = value
                        .parse()
                        .map_err(|_| GlifParserError::UnknownPointType(value.into()))?;
                }
                b"smooth" => smooth = value == "yes",
                b"identifier" => {
                    identifier = Some(self.parse_identifier(&value)?);
                }
                _ => return Err(GlifParserError::UnexpectedAttribute("point", b2s(attr.key))),
            }
        }

        match (x, y) {
            (Some(x), Some(y)) => {
                outline_builder
                    .add_point((x, y), typ, smooth, name, identifier)
                    .map_err(GlifParserError::Draw)?;
                Ok(())
            }
            _ => return Err(GlifParserError::InvalidPoint),
        }
    }

    fn parse_advance<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), GlifParserError> {
        let mut width: f64 = 0.0;
        let mut height: f64 = 0.0;

        for attr in data.attributes() {
            let attr = attr.map_err(GlifParserError::Xml)?;
            match attr.key {
                b"width" | b"height" => {
                    let value = attr.unescaped_value().map_err(GlifParserError::Xml)?;
                    let value = reader.decode(&value).map_err(GlifParserError::Xml)?;
                    let value: f64 = value
                        .parse()
                        .map_err(|e| GlifParserError::InvalidAdvance(value.into(), e))?;
                    match attr.key {
                        b"width" => width = value,
                        b"height" => height = value,
                        _ => unreachable!(),
                    };
                }
                _ => return Err(GlifParserError::UnexpectedAttribute("advance", b2s(attr.key))),
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
    ) -> Result<(), GlifParserError> {
        for attr in data.attributes() {
            let attr = attr.map_err(GlifParserError::Xml)?;
            match attr.key {
                b"hex" => {
                    let value = attr.unescaped_value().map_err(GlifParserError::Xml)?;
                    let value = reader.decode(&value).map_err(GlifParserError::Xml)?;
                    let chr = u32::from_str_radix(value, 16)
                        .map_err(|_| value.to_string())
                        .and_then(|n| char::try_from(n).map_err(|_| value.to_string()))
                        .map_err(|_| GlifParserError::BadUnicodeValue(value.to_string()))?;
                    self.glyph.codepoints.push(chr);
                }
                _ => return Err(GlifParserError::UnexpectedAttribute("unicode", b2s(attr.key))),
            }
        }
        Ok(())
    }

    fn parse_anchor<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), GlifParserError> {
        let mut x: Option<f64> = None;
        let mut y: Option<f64> = None;
        let mut name: Option<String> = None;
        let mut color: Option<Color> = None;
        let mut identifier: Option<Identifier> = None;

        for attr in data.attributes() {
            let attr = attr.map_err(GlifParserError::Xml)?;
            let value = attr.unescaped_value().map_err(GlifParserError::Xml)?;
            let value = reader.decode(&value).map_err(GlifParserError::Xml)?;
            match attr.key {
                b"x" => {
                    x = Some(
                        value
                            .parse()
                            .map_err(|e| GlifParserError::InvalidCoordinate(value.into(), e))?,
                    );
                }
                b"y" => {
                    y = Some(
                        value
                            .parse()
                            .map_err(|e| GlifParserError::InvalidCoordinate(value.into(), e))?,
                    );
                }
                b"name" => name = Some(value.to_string()),
                b"color" => {
                    color = Some(
                        value.parse().map_err(|_| GlifParserError::InvalidColor(value.into()))?,
                    )
                }
                b"identifier" => {
                    identifier = Some(self.parse_identifier(&value)?);
                }
                _ => return Err(GlifParserError::UnexpectedAttribute("anchor", b2s(attr.key))),
            }
        }

        match (x, y) {
            (Some(x), Some(y)) => {
                self.glyph.anchors.push(Anchor::new(x, y, name, color, identifier, None));
                Ok(())
            }
            _ => return Err(GlifParserError::InvalidAnchor),
        }
    }

    fn parse_guideline<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), GlifParserError> {
        let mut x: Option<f64> = None;
        let mut y: Option<f64> = None;
        let mut angle: Option<f64> = None;
        let mut name: Option<String> = None;
        let mut color: Option<Color> = None;
        let mut identifier: Option<Identifier> = None;

        for attr in data.attributes() {
            let attr = attr.map_err(GlifParserError::Xml)?;
            let value = attr.unescaped_value().map_err(GlifParserError::Xml)?;
            let value = reader.decode(&value).map_err(GlifParserError::Xml)?;
            match attr.key {
                b"x" => {
                    x = Some(
                        value
                            .parse()
                            .map_err(|e| GlifParserError::InvalidCoordinate(value.into(), e))?,
                    );
                }
                b"y" => {
                    y = Some(
                        value
                            .parse()
                            .map_err(|e| GlifParserError::InvalidCoordinate(value.into(), e))?,
                    );
                }
                b"angle" => {
                    let angle_value = value
                        .parse()
                        .map_err(|e| GlifParserError::InvalidAngle(value.into(), e))?;
                    if !(0.0..=360.0).contains(&angle_value) {
                        return Err(GlifParserError::InvalidAngleBounds);
                    }
                    angle = Some(angle_value);
                }
                b"name" => name = Some(value.to_string()),
                b"color" => {
                    color = Some(
                        value.parse().map_err(|_| GlifParserError::InvalidColor(value.into()))?,
                    )
                }
                b"identifier" => {
                    identifier = Some(self.parse_identifier(&value)?);
                }
                _ => return Err(GlifParserError::UnexpectedAttribute("guideline", b2s(attr.key))),
            }
        }

        let line = match (x, y, angle) {
            (Some(x), None, None) => Line::Vertical(x),
            (None, Some(y), None) => Line::Horizontal(y),
            (Some(x), Some(y), Some(degrees)) => Line::Angle { x, y, degrees },
            _ => return Err(GlifParserError::InvalidGuideline),
        };
        self.glyph.guidelines.push(Guideline::new(line, name, color, identifier, None));

        Ok(())
    }

    fn parse_image<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), GlifParserError> {
        let mut filename: Option<PathBuf> = None;
        let mut color: Option<Color> = None;
        let mut transform = AffineTransform::default();

        for attr in data.attributes() {
            let attr = attr.map_err(GlifParserError::Xml)?;
            let value = attr.unescaped_value().map_err(GlifParserError::Xml)?;
            let value = reader.decode(&value).map_err(GlifParserError::Xml)?;
            let bad_transform = |e| GlifParserError::InvalidImageTransformation(value.into(), e);
            match attr.key {
                b"xScale" => transform.x_scale = value.parse().map_err(bad_transform)?,
                b"xyScale" => transform.xy_scale = value.parse().map_err(bad_transform)?,
                b"yxScale" => transform.yx_scale = value.parse().map_err(bad_transform)?,
                b"yScale" => transform.y_scale = value.parse().map_err(bad_transform)?,
                b"xOffset" => transform.x_offset = value.parse().map_err(bad_transform)?,
                b"yOffset" => transform.y_offset = value.parse().map_err(bad_transform)?,
                b"color" => {
                    color = Some(value.parse().map_err(GlifParserError::InvalidImageColor)?)
                }
                b"fileName" => filename = Some(PathBuf::from(value)),
                _ => return Err(GlifParserError::UnexpectedAttribute("image", b2s(attr.key))),
            }
        }

        match filename {
            Some(file_name) => {
                self.glyph.image = Some(Image { file_name, color, transform });
                Ok(())
            }
            None => Err(GlifParserError::ImageMissingFilename),
        }
    }
}

fn b2s(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Move libs from the lib's `public.objectLibs` into the actual objects.
/// The key will be removed from the glyph lib.
fn move_object_libs(glyph: &mut Glyph) -> Result<(), GlifParserError> {
    // Use a macro to reduce boilerplate, to avoid having to mess with the typing system.
    macro_rules! move_lib {
        ($object:expr, $object_libs:expr) => {
            if let Some(id) = $object.identifier().map(|v| v.as_str()) {
                if let Some(lib) = $object_libs.remove(id) {
                    let lib = lib
                        .into_dictionary()
                        .ok_or(GlifParserError::ObjectLibMustBeDictionary(id.into()))?;
                    $object.replace_lib(lib);
                }
            }
        };
    }

    let mut object_libs = match glyph.lib.remove(PUBLIC_OBJECT_LIBS_KEY) {
        Some(lib) => {
            lib.into_dictionary().ok_or(GlifParserError::PublicObjectLibsMustBeDictionary)?
        }
        None => return Ok(()),
    };

    for anchor in &mut glyph.anchors {
        move_lib!(anchor, object_libs);
    }
    for guideline in &mut glyph.guidelines {
        move_lib!(guideline, object_libs);
    }
    for contour in &mut glyph.contours {
        move_lib!(contour, object_libs);
        for point in &mut contour.points {
            move_lib!(point, object_libs);
        }
    }
    for component in &mut glyph.components {
        move_lib!(component, object_libs);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_glyph(xml: &[u8]) -> Result<Glyph, GlifParserError> {
        GlifParser::from_xml(xml, None)
    }

    #[test]
    fn serialize_full_glyph() {
        let source = include_str!("../../testdata/sample_period_normalized.glif");
        let glyph = parse_glyph(source.as_bytes()).unwrap();
        let glif = glyph.encode_xml().unwrap();
        let glif = String::from_utf8(glif).expect("xml is always valid UTF-8");
        pretty_assertions::assert_eq!(glif, source);
    }
}
