//! Writing out .glif files

use std::io::{Cursor, Write};

use quick_xml::{
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
    Error as XmlError, Writer,
};

use super::{
    Advance, AffineTransform, Anchor, Color, Component, Contour, ContourPoint, GlifVersion, Glyph,
    Guideline, Identifier, Image, Line, PointType,
};

use crate::error::GlifWriteError;

impl Glyph {
    pub(crate) fn encode_xml(&self) -> Result<Vec<u8>, GlifWriteError> {
        self.encode_xml_impl().map_err(|inner| GlifWriteError { name: self.name.clone(), inner })
    }

    fn encode_xml_impl(&self) -> Result<Vec<u8>, XmlError> {
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
        writer.write_event(Event::Decl(BytesDecl::new(b"1.1", Some(b"UTF-8"), None)))?;
        let mut start = BytesStart::borrowed_name(b"glyph");
        start.push_attribute(("name", &*self.name));
        start.push_attribute(("format", self.format.as_str()));
        writer.write_event(Event::Start(start))?;

        if let Some(event) = self.advance.as_ref().map(Advance::to_event) {
            writer.write_event(event)?;
        }

        if let Some(codepoints) = self.codepoints.as_ref() {
            for codepoint in codepoints.iter() {
                writer.write_event(char_to_event(*codepoint))?;
            }
        }

        if let Some(ref note) = self.note {
            writer.write_event(Event::Start(BytesStart::borrowed_name(b"note")))?;
            writer.write_event(Event::Text(BytesText::from_plain_str(note)))?;
            writer.write_event(Event::End(BytesEnd::borrowed(b"note")))?;
        }

        if let Some(ref image) = self.image {
            writer.write_event(image.to_event())?;
        }

        if let Some(guides) = self.guidelines.as_ref() {
            for guide in guides.iter() {
                writer.write_event(guide.to_event())?;
            }
        }

        if let Some(anchors) = self.anchors.as_ref() {
            for anchor in anchors.iter() {
                writer.write_event(anchor.to_event())?;
            }
        }

        if let Some(ref outline) = self.outline {
            writer.write_event(Event::Start(BytesStart::borrowed_name(b"outline")))?;
            for contour in &outline.contours {
                contour.write_xml(&mut writer)?;
            }

            for component in &outline.components {
                writer.write_event(component.to_event())?;
            }
            writer.write_event(Event::End(BytesEnd::borrowed(b"outline")))?;
        }
        writer.write_event(Event::End(BytesEnd::borrowed(b"glyph")))?;

        Ok(writer.into_inner().into_inner())
    }
}

impl GlifVersion {
    fn as_str(&self) -> &str {
        match self {
            GlifVersion::V1 => "1",
            GlifVersion::V2 => "2",
        }
    }
}

impl Advance {
    fn to_event(&self) -> Event {
        let mut start = BytesStart::borrowed_name(b"advance");
        if self.width != 0. {
            start.push_attribute(("width", self.width.to_string().as_str()));
        }

        if self.height != 0. {
            start.push_attribute(("height", self.height.to_string().as_str()));
        }
        Event::Empty(start)
    }
}

impl Guideline {
    fn to_event(&self) -> Event {
        let mut start = BytesStart::borrowed_name(b"guideline");
        let (x, y, angle) = match self.line {
            Line::Vertical(x) => (Some(x), None, None),
            Line::Horizontal(y) => (None, Some(y), None),
            Line::Angle { x, y, degrees } => (Some(x), Some(y), Some(degrees)),
        };

        if let Some(x) = x {
            start.push_attribute(("x", x.to_string().as_str()))
        }

        if let Some(y) = y {
            start.push_attribute(("y", y.to_string().as_str()))
        }

        if let Some(angle) = angle {
            start.push_attribute(("angle", angle.to_string().as_str()))
        }

        if let Some(name) = &self.name {
            start.push_attribute(("name", name.as_str()));
        }

        if let Some(color) = &self.color {
            start.push_attribute(("color", color.to_rgba_string().as_str()));
        }

        if let Some(Identifier(id)) = &self.identifier {
            start.push_attribute(("identifier", id.as_str()));
        }
        Event::Empty(start)
    }
}

impl Anchor {
    fn to_event(&self) -> Event {
        let mut start = BytesStart::borrowed_name(b"anchor");

        start.push_attribute(("x", self.x.to_string().as_str()));
        start.push_attribute(("y", self.y.to_string().as_str()));

        if let Some(name) = &self.name {
            start.push_attribute(("name", name.as_str()));
        }

        if let Some(Identifier(id)) = &self.identifier {
            start.push_attribute(("identifier", id.as_str()));
        }

        if let Some(color) = &self.color {
            start.push_attribute(("color", color.to_rgba_string().as_str()));
        }
        Event::Empty(start)
    }
}

impl Component {
    fn to_event(&self) -> Event {
        let mut start = BytesStart::borrowed_name(b"component");
        start.push_attribute(("base", &*self.base));

        write_transform_attributes(&mut start, &self.transform);

        if let Some(Identifier(id)) = &self.identifier {
            start.push_attribute(("identifier", id.as_str()));
        }
        Event::Empty(start)
    }
}

impl Contour {
    fn write_xml<T: Write>(&self, writer: &mut Writer<T>) -> Result<(), XmlError> {
        let mut start = BytesStart::borrowed_name(b"contour");

        if let Some(Identifier(id)) = &self.identifier {
            start.push_attribute(("identifier", id.as_str()));
        }

        writer.write_event(Event::Start(start))?;

        for point in &self.points {
            writer.write_event(point.to_event())?;
        }
        writer.write_event(Event::End(BytesEnd::borrowed(b"contour")))?;
        Ok(())
    }
}

impl ContourPoint {
    fn to_event(&self) -> Event {
        let mut start = BytesStart::borrowed_name(b"point");

        start.push_attribute(("x", self.x.to_string().as_str()));
        start.push_attribute(("y", self.y.to_string().as_str()));

        match self.typ {
            PointType::OffCurve => {}
            _ => start.push_attribute(("type", self.typ.as_str())),
        }

        if self.smooth {
            start.push_attribute(("smooth", "yes"));
        }

        if let Some(name) = &self.name {
            start.push_attribute(("name", name.as_str()));
        }

        if let Some(Identifier(id)) = &self.identifier {
            start.push_attribute(("identifier", id.as_str()));
        }
        Event::Empty(start)
    }
}

impl PointType {
    fn as_str(&self) -> &str {
        match self {
            PointType::Move => "move",
            PointType::Line => "line",
            PointType::OffCurve => "offcurve",
            PointType::Curve => "curve",
            PointType::QCurve => "qcurve",
        }
    }
}

impl Color {
    fn to_rgba_string(&self) -> String {
        format!("{},{},{},{}", self.red, self.green, self.blue, self.alpha)
    }
}

impl Image {
    fn to_event(&self) -> Event {
        let mut start = BytesStart::borrowed_name(b"image");
        start.push_attribute(("fileName", self.file_name.to_str().unwrap_or("missing path")));

        write_transform_attributes(&mut start, &self.transform);

        if let Some(color) = &self.color {
            start.push_attribute(("color", color.to_rgba_string().as_str()));
        }
        Event::Empty(start)
    }
}

fn char_to_event(c: char) -> Event<'static> {
    let mut start = BytesStart::borrowed_name(b"unicode");
    let hex = format!("{:04X}", c as u32);
    start.push_attribute(("hex", hex.as_str()));
    Event::Empty(start)
}

fn write_transform_attributes(element: &mut BytesStart, transform: &AffineTransform) {
    if (transform.x_scale - 1.0).abs() > std::f32::EPSILON {
        element.push_attribute(("xScale", transform.x_scale.to_string().as_str()));
    }

    if transform.xy_scale != 0.0 {
        element.push_attribute(("xyScale", transform.xy_scale.to_string().as_str()));
    }

    if transform.yx_scale != 0.0 {
        element.push_attribute(("yxScale", transform.yx_scale.to_string().as_str()));
    }

    if (transform.y_scale - 1.0).abs() > std::f32::EPSILON {
        element.push_attribute(("yScale", transform.y_scale.to_string().as_str()));
    }

    if transform.x_offset != 0.0 {
        element.push_attribute(("xOffset", transform.x_offset.to_string().as_str()));
    }

    if transform.y_offset != 0.0 {
        element.push_attribute(("yOffset", transform.y_offset.to_string().as_str()));
    }
}
