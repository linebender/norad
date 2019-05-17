use std::borrow::{Borrow, Cow};
use std::str::FromStr;
use std::convert::TryFrom;

use crate::ufo::{Anchor, Color, Contour, ContourPoint, Outline, GlifVersion, Glyph, Identifier, PointType};
use quick_xml::{Error as XmlError, events::{attributes::Attribute, BytesStart, Event}, Reader};

#[derive(Debug)]
pub enum Error {
    ParseError(XmlError),
    UnsupportedGlifVersion(String),
    UnknownPointType(String),
    WrongFirstElement,
    UnexpectedTag(String),
    BadHexValue(String),
    BadNumber(String),
    BadColor(String),
    BadAnchor(usize),
    BadPoint(usize),
    UnexpectedDuplicate(&'static str),
    UnexpectedXml(Event<'static>),
    UnexpectedEof,
}

impl From<XmlError> for Error {
    fn from(src: XmlError) -> Error {
        Error::ParseError(src)
    }
}

pub fn parse_glyph(xml: &[u8]) -> Result<Glyph, Error> {
    GlifParser::from_xml(xml)
}

struct GlifParser(Glyph);

impl GlifParser {
    fn from_xml(xml: &[u8]) -> Result<Glyph, Error> {
        let mut reader = Reader::from_reader(xml);
        let mut buf = Vec::new();
        reader.trim_text(true);

        let glyph = start(&mut reader, &mut buf)?;
        let mut this = GlifParser(glyph);
        this.parse_body(&mut reader, &mut buf)
    }

    fn parse_body(mut self, reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<Glyph, Error> {
        loop {
            match reader.read_event(buf)? {
                Event::Start(start) | Event::Empty(start) => {
                    let tag_name = reader.decode(&start.name());
                    match tag_name.borrow() {
                        "outline" => self.parse_outline(reader, buf)?,
                        "lib" => self.parse_lib(reader, buf)?, // do this at some point?
                        "note" => self.parse_note(reader, buf)?,
                        "advance" => self.parse_advance(reader, start)?,
                        "unicode" => self.parse_unicode(reader, start)?,
                        "anchor" => self.parse_anchor(reader, start)?,
                        "guideline" => self.parse_guideline(reader, start)?,
                        "image" => self.parse_image(reader, start)?,
                        other => return Err(Error::UnexpectedTag(other.to_string())),
                    }
                }
                _other => break,
            }
        }
        Ok(self.0)
    }

    fn parse_outline(&mut self, reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<(), Error> {
        if self.0.outline.is_some() {
            return Err(Error::UnexpectedDuplicate("outline"));
        }

        self.0.outline = Some(Outline { components: Vec::new(), contours: Vec::new() });

        loop {
            match reader.read_event(buf)? {
                Event::Start(start) | Event::Empty(start) => {
                    let tag_name = reader.decode(&start.name());
                    let mut new_buf = Vec::new(); // borrowck :/
                    match tag_name.borrow() {
                        "contour" => self.parse_contour(start, reader, &mut new_buf)?,
                        "component" => self.parse_component(reader, buf)?,
                        _other => eprintln!("unexpected tag in outline {}", tag_name),
                    }
                }
                Event::End(ref end) if end.name() == b"outline" => break,
                Event::Eof => return Err(Error::UnexpectedEof),
                _other => (),
            }
        }
        Ok(())
    }

    fn parse_contour(&mut self, data: BytesStart, reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<(), Error> {
        let mut identifier = None;
        for attr in data.attributes() {
            let attr = attr?;
            if attr.key == b"identifier" {
                identifier = Some(Identifier(attr.unescape_and_decode_value(reader)?));
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
                Event::Eof => return Err(Error::UnexpectedEof),
                _other => return Err(Error::UnexpectedXml(_other.into_owned())),
            }
        }
        self.0.outline.as_mut().unwrap().contours.push(Contour { identifier, points });
        Ok(())
    }

    fn parse_component(&mut self, reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<(), Error> {
        Ok(())
    }

    fn parse_lib(&mut self, reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<(), Error> {
        loop {
            match reader.read_event(buf)? {
                Event::End(ref end) if end.name() == b"lib" => break,
                Event::Eof => return Err(Error::UnexpectedEof),
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
                    self.0.note = Some(text.unescape_and_decode(reader)?);
                    break;
                },
                Event::Eof => return Err(Error::UnexpectedEof),
                _other => (),
            }
        }
        Ok(())
    }

    fn parse_point<'a>(&mut self, reader: &Reader<&[u8]>, data: &BytesStart<'a>) -> Result<ContourPoint, Error> {

        let mut name: Option<String> = None;
        let mut x: Option<f32> = None;
        let mut y: Option<f32> = None;
        let mut typ = PointType::OffCurve;
        let mut identifier: Option<Identifier> = None;
        let mut smooth = false;

        for attr in data.attributes() {
            let attr = attr?;
            let value = attr.unescaped_value()?;
            let value = reader.decode(&value);
            match attr.key {
                b"x" => {
                    x = Some(value.parse().map_err(|_| Error::BadNumber(value.to_string()))?);
                }
                b"y" => {
                    y = Some(value.parse().map_err(|_| Error::BadNumber(value.to_string()))?);
                }
                b"name" => name = Some(value.to_string()),
                b"typ" => typ = value.parse()?,
                b"smooth" => smooth = value == "yes",
                b"identifier" => identifier = Some(Identifier(value.to_string())),
                _other => eprintln!("unexpected point field {}", value),

            }
        }
        if x.is_none() || y.is_none() {
            return Err(Error::BadPoint(reader.buffer_position()));
        }
        Ok(ContourPoint {
            x: x.unwrap(),
            y: y.unwrap(),
            typ, name, identifier, smooth,
        })
    }

    fn parse_advance<'a>(&mut self, reader: &Reader<&[u8]>, data: BytesStart<'a>) -> Result<(), Error> {
        for attr in data.attributes() {
            let attr = attr?;
            if attr.key == b"width" || attr.key == b"height" {
                let value = attr.unescaped_value()?;
                let value = reader.decode(&value);
                let value: f64 = value.parse().map_err(|_| Error::BadNumber(value.to_string()))?;
                if attr.key == b"width" {
                    self.0.width = Some(value);
                } else {
                    self.0.height = Some(value);
                }
            }
        }
        Ok(())
    }

    fn parse_unicode<'a>(&mut self, reader: &Reader<&[u8]>, data: BytesStart<'a>) -> Result<(), Error> {
        for attr in data.attributes() {
            let attr = attr?;
            if attr.key == b"hex" {
                let value = attr.unescaped_value()?;
                let value = reader.decode(&value);
                let chr = u32::from_str_radix(&value, 16).map_err(|_| value.to_string())
                    .and_then(|n| char::try_from(n).map_err(|_| value.to_string()))
                    .map_err(|e| Error::BadHexValue(e))?;
                 self.0.codepoints.get_or_insert(Vec::new()).push(chr);
            }
        }
        Ok(())
    }

    fn parse_anchor<'a>(&mut self, reader: &Reader<&[u8]>, data: BytesStart<'a>) -> Result<(), Error> {
        let mut x: Option<f32> = None;
        let mut y: Option<f32> = None;
        let mut name: Option<String> = None;
        let mut color: Option<Color> = None;
        let mut identifier: Option<Identifier> = None;

        for attr in data.attributes() {
            let attr = attr?;
            let value = attr.unescaped_value()?;
            let value = reader.decode(&value);
            match attr.key {
                b"x" => {
                    x = Some(value.parse().map_err(|_| Error::BadNumber(value.to_string()))?);
                }
                b"y" => {
                    y = Some(value.parse().map_err(|_| Error::BadNumber(value.to_string()))?);
                }
                b"name" => name = Some(value.to_string()),
                b"color" => color = Some(value.parse()?),
                b"identifier" => identifier = Some(Identifier(value.to_string())),
                _other => eprintln!("unexpected anchor field {}", value),
            }
        }

        if x.is_none() || y.is_none() {
            return Err(Error::BadAnchor(reader.buffer_position()));
        }
        let anchors = self.0.anchors.get_or_insert(Vec::new());
        anchors.push(Anchor {
            x: x.unwrap(),
            y: y.unwrap(),
            name, color, identifier,
        });
        Ok(())
    }

    fn parse_guideline<'a>(&mut self, reader: &Reader<&[u8]>, data: BytesStart<'a>) -> Result<(), Error> {
        Ok(())
    }

    fn parse_image<'a>(&mut self, reader: &Reader<&[u8]>, data: BytesStart<'a>) -> Result<(), Error> {
        Ok(())
    }
}

fn start(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<Glyph, Error> {
    loop {
        match reader.read_event(buf) {
            Ok(Event::Decl(decl)) => (),
            Ok(Event::Start(ref start)) if start.name() == b"glyph" => {
                let mut name = String::new();
                let mut format: Option<GlifVersion> = None;
                for attr in start.attributes() {
                    let attr = attr?;
                    if attr.key == b"name" {
                        name = attr.unescape_and_decode_value(&reader)?;
                    } else if attr.key == b"format" {
                        let value = attr.unescaped_value()?;
                        let value = reader.decode(&value);
                        format = Some(value.parse()?);
                    }
                }
                if !name.is_empty() && format.is_some() {
                    return Ok(Glyph::new(name, format.take().unwrap()));
                } else {
                    eprintln!("name '{}', format {:?}", name, format);
                    return Err(Error::WrongFirstElement);
                }
            }
            Ok(_other) => {
                eprintln!("breaking for {:?}", _other);
                break;
            }
            Err(e) => return Err(Error::ParseError(e)),
        }
    }
    Err(Error::WrongFirstElement)
}

impl FromStr for GlifVersion {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(GlifVersion::V1),
            "2" => Ok(GlifVersion::V2),
            other => Err(Error::UnsupportedGlifVersion(other.to_string()))
        }
    }
}

impl FromStr for Color {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split(',').map(|s| s.parse::<f32>().map_err(|e| Error::BadColor(s.to_string())));
        Ok(Color {
            red: iter.next().ok_or_else(|| Error::BadColor(s.to_string())).and_then(|r| r)?,
            green: iter.next().ok_or_else(|| Error::BadColor(s.to_string())).and_then(|r| r)?,
            blue: iter.next().ok_or_else(|| Error::BadColor(s.to_string())).and_then(|r| r)?,
            alpha: iter.next().ok_or_else(|| Error::BadColor(s.to_string())).and_then(|r| r)?,
        })
    }
}

impl FromStr for PointType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "move" => Ok(PointType::Move),
            "line" => Ok(PointType::Line),
            "offcurve" => Ok(PointType::OffCurve),
            "curve" => Ok(PointType::Curve),
            "qcurve" => Ok(PointType::QCurve),
            other => Err(Error::UnknownPointType(other.to_string())),
        }
    }
}

