use std::borrow::Borrow;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::str::FromStr;

use super::*;
use crate::error::{ErrorKind, GlifErrorInternal};
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
    glyph: Glyph,
    /// Optional set of glyph names to be reused between glyphs.
    names: Option<&'names NameList>,
}

impl<'names> GlifParser<'names> {
    pub(crate) fn from_xml(xml: &[u8], names: Option<&'names NameList>) -> Result<Glyph, Error> {
        let mut reader = Reader::from_reader(xml);
        let mut buf = Vec::new();
        reader.trim_text(true);

        let glyph = start(&mut reader, &mut buf)?;
        let this = GlifParser { glyph, names };
        this.parse_body(&mut reader, &mut buf)
    }

    fn parse_body(mut self, reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<Glyph, Error> {
        loop {
            match reader.read_event(buf)? {
                Event::Start(start) | Event::Empty(start) => {
                    let tag_name = reader.decode(&start.name())?;
                    match tag_name.borrow() {
                        "outline" => self.parse_outline(reader, buf)?,
                        "lib" => self.parse_lib(reader, buf)?, // do this at some point?
                        "note" => self.parse_note(reader, buf)?,
                        "advance" => self.parse_advance(reader, start)?,
                        "unicode" => self.parse_unicode(reader, start)?,
                        "anchor" => match &self.glyph.format {
                            GlifVersion::V1 => return Err(err!(reader, ErrorKind::UnexpectedTag)),
                            GlifVersion::V2 => self.parse_anchor(reader, start)?,
                        },
                        "guideline" => match &self.glyph.format {
                            GlifVersion::V1 => return Err(err!(reader, ErrorKind::UnexpectedTag)),
                            GlifVersion::V2 => self.parse_guideline(reader, start)?,
                        },
                        "image" => match &self.glyph.format {
                            GlifVersion::V1 => return Err(err!(reader, ErrorKind::UnexpectedTag)),
                            GlifVersion::V2 => self.parse_image(reader, start)?,
                        },
                        _other => return Err(err!(reader, ErrorKind::UnexpectedTag)),
                    }
                }
                Event::End(ref end) if end.name() == b"glyph" => break,
                _other => return Err(err!(reader, ErrorKind::MissingCloseTag)),
            }
        }
        self.glyph.format = GlifVersion::V2;
        Ok(self.glyph)
    }

    fn parse_outline(
        &mut self,
        reader: &mut Reader<&[u8]>,
        buf: &mut Vec<u8>,
    ) -> Result<(), Error> {
        if self.glyph.outline.is_some() {
            return Err(err!(reader, ErrorKind::UnexpectedDuplicate));
        }

        self.glyph.outline = Some(Outline { components: Vec::new(), contours: Vec::new() });

        loop {
            match reader.read_event(buf)? {
                Event::Start(start) | Event::Empty(start) => {
                    let tag_name = reader.decode(&start.name())?;
                    let mut new_buf = Vec::new(); // borrowck :/
                    match tag_name.borrow() {
                        "contour" => self.parse_contour(start, reader, &mut new_buf)?,
                        "component" => self.parse_component(reader, start)?,
                        _other => eprintln!("unexpected tag in outline {}", tag_name),
                    }
                }
                Event::End(ref end) if end.name() == b"outline" => break,
                Event::Eof => return Err(err!(reader, ErrorKind::UnexpectedEof)),
                _other => return Err(err!(reader, ErrorKind::UnexpectedElement)),
            }
        }
        Ok(())
    }

    fn parse_contour(
        &mut self,
        data: BytesStart,
        reader: &mut Reader<&[u8]>,
        buf: &mut Vec<u8>,
    ) -> Result<(), Error> {
        let mut identifier = None;
        for attr in data.attributes() {
            if self.glyph.format == GlifVersion::V1 {
                return Err(err!(reader, ErrorKind::UnexpectedAttribute));
            }
            let attr = attr?;
            if attr.key == b"identifier" {
                let ident = attr.unescape_and_decode_value(reader)?;
                identifier = Some(Identifier::new(ident).map_err(|kind| err!(reader, kind))?);
            }
        }

        let mut points = Vec::new();
        loop {
            match reader.read_event(buf)? {
                Event::End(ref end) if end.name() == b"contour" => break,
                Event::Empty(ref start) if start.name() == b"point" => {
                    let point = self.parse_point(reader, start)?;
                    points.push(point);
                }
                Event::Eof => return Err(err!(reader, ErrorKind::UnexpectedEof)),
                _other => return Err(err!(reader, ErrorKind::UnexpectedElement)),
            }
        }

        // In the Glif v1 spec, single-point contours that have a "move" type and a name should
        // be treated as anchors and upgraded.
        if contour_is_v1_anchor(&self.glyph.format, &points) {
            let anchor_point = points.remove(0);
            let anchor = Anchor {
                name: anchor_point.name,
                x: anchor_point.x,
                y: anchor_point.y,
                identifier: None,
                color: None,
            };

            self.glyph.anchors.get_or_insert(Vec::new()).push(anchor);
        } else {
            self.glyph.outline.as_mut().unwrap().contours.push(Contour { identifier, points });
        }

        Ok(())
    }

    fn parse_component(
        &mut self,
        reader: &mut Reader<&[u8]>,
        start: BytesStart,
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
                _other => eprintln!("unexpected component field {}", value),
            }
        }

        if base.is_none() {
            return Err(err!(reader, ErrorKind::BadComponent));
        }

        let component = Component { base: base.unwrap(), transform, identifier };
        self.glyph.outline.as_mut().unwrap().components.push(component);
        Ok(())
    }

    fn parse_lib(&mut self, reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<(), Error> {
        if self.glyph.lib.is_some() {
            return Err(err!(reader, ErrorKind::UnexpectedDuplicate));
        }

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
        if self.glyph.note.is_some() {
            return Err(err!(reader, ErrorKind::UnexpectedDuplicate));
        }

        loop {
            match reader.read_event(buf)? {
                Event::End(ref end) if end.name() == b"note" => break,
                Event::Text(text) => {
                    self.glyph.note = Some(text.unescape_and_decode(reader)?);
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
    ) -> Result<ContourPoint, Error> {
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
                _other => eprintln!("unexpected point field {}", String::from_utf8_lossy(_other)),
            }
        }
        if x.is_none() || y.is_none() {
            return Err(err!(reader, ErrorKind::BadPoint));
        }
        Ok(ContourPoint { x: x.unwrap(), y: y.unwrap(), typ, name, identifier, smooth })
    }

    fn parse_advance<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), Error> {
        if self.glyph.advance.is_some() {
            return Err(err!(reader, ErrorKind::UnexpectedDuplicate));
        }

        let mut advance = Advance::default();
        for attr in data.attributes() {
            let attr = attr?;
            if attr.key == b"width" || attr.key == b"height" {
                let value = attr.unescaped_value()?;
                let value = reader.decode(&value)?;
                let value: f32 = value.parse().map_err(|_| err!(reader, ErrorKind::BadNumber))?;
                match attr.key {
                    b"width" => advance.width = value,
                    b"height" => advance.height = value,
                    _ => unreachable!(),
                };
            }
        }
        self.glyph.advance = Some(advance);
        Ok(())
    }

    fn parse_unicode<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), Error> {
        for attr in data.attributes() {
            let attr = attr?;
            if attr.key == b"hex" {
                let value = attr.unescaped_value()?;
                let value = reader.decode(&value)?;
                let chr = u32::from_str_radix(&value, 16)
                    .map_err(|_| value.to_string())
                    .and_then(|n| char::try_from(n).map_err(|_| value.to_string()))
                    .map_err(|_| err!(reader, ErrorKind::BadHexValue))?;
                self.glyph.codepoints.get_or_insert(Vec::new()).push(chr);
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
                _other => eprintln!("unexpected anchor field {}", value),
            }
        }

        if x.is_none() || y.is_none() {
            return Err(err!(reader, ErrorKind::BadAnchor));
        }
        let anchors = self.glyph.anchors.get_or_insert(Vec::new());
        anchors.push(Anchor { x: x.unwrap(), y: y.unwrap(), name, color, identifier });
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
                _other => eprintln!("unexpected guideline field {}", value),
            }
        }

        let line = match (x, y, angle) {
            (Some(x), None, None) => Line::Vertical(x),
            (None, Some(y), None) => Line::Horizontal(y),
            (Some(x), Some(y), Some(degrees)) => Line::Angle { x, y, degrees },
            _other => return Err(err!(reader, ErrorKind::BadGuideline)),
        };

        let guideline = Guideline { line, name, color, identifier };
        self.glyph.guidelines.get_or_insert(Vec::new()).push(guideline);

        Ok(())
    }

    fn parse_image<'a>(
        &mut self,
        reader: &Reader<&[u8]>,
        data: BytesStart<'a>,
    ) -> Result<(), Error> {
        if self.glyph.image.is_some() {
            return Err(err!(reader, ErrorKind::UnexpectedDuplicate));
        }

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
                _other => eprintln!("unexpected image field {}", value),
            }
        }

        if filename.is_none() {
            return Err(err!(reader, ErrorKind::BadImage));
        }

        let image = Image { file_name: filename.unwrap(), color, transform };
        self.glyph.image = Some(image);

        Ok(())
    }
}

fn start(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<Glyph, Error> {
    loop {
        match reader.read_event(buf)? {
            Event::Comment(_) => (),
            Event::Decl(_decl) => (),
            Event::Start(ref start) if start.name() == b"glyph" => {
                let mut name = String::new();
                let mut format: Option<GlifVersion> = None;
                for attr in start.attributes() {
                    let attr = attr?;
                    if attr.key == b"name" {
                        name = attr.unescape_and_decode_value(&reader)?;
                    } else if attr.key == b"format" {
                        let value = attr.unescaped_value()?;
                        let value = reader.decode(&value)?;
                        format = Some(
                            value
                                .parse()
                                .map_err(|e: ErrorKind| e.to_error(reader.buffer_position()))?,
                        );
                    }
                }
                if !name.is_empty() && format.is_some() {
                    return Ok(Glyph::new(name.into(), format.take().unwrap()));
                } else {
                    eprintln!("name '{}', format {:?}", name, format);
                    return Err(err!(reader, ErrorKind::WrongFirstElement));
                }
            }
            other => {
                eprintln!("breaking for {:?}", other);
                break;
            }
        }
    }
    Err(err!(reader, ErrorKind::WrongFirstElement))
}

/// Check if a contour is really an informal anchor according to the Glif v2 specification.
fn contour_is_v1_anchor(format: &GlifVersion, points: &[ContourPoint]) -> bool {
    *format == GlifVersion::V1
        && points.len() == 1
        && points[0].typ == PointType::Move
        && points[0].name.is_some()
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
