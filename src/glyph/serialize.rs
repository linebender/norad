//! Writing out .glif files

use std::io::{Cursor, Write};

use quick_xml::{
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
    Error as XmlError, Writer,
};

use super::{
    Advance, Anchor, Color, Component, Contour, ContourPoint, GlifVersion, Glyph, Guideline,
    Identifier, Image, Line, PointType,
};

impl Glyph {
    pub(crate) fn encode_xml(&self) -> Result<Vec<u8>, XmlError> {
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
        writer.write_event(Event::Decl(BytesDecl::new(b"1.1", Some(b"UTF-8"), None)))?;
        let mut start = BytesStart::borrowed_name(b"glyph");
        start.push_attribute(("name", self.name.as_str()));
        start.push_attribute(("format", self.format.as_str()));
        writer.write_event(Event::Start(start))?;

        if let Some(event) = self.advance.as_ref().map(Advance::to_event) {
            writer.write_event(event)?;
        }

        if let Some(codepoints) = self.codepoints.as_ref() {
            for codepoint in codepoints.iter() {
                writer.write_event(char_to_event(codepoint))?;
            }
        }

        if let Some(ref note) = self.note {
            writer.write_event(Event::Start(BytesStart::borrowed_name(b"note")))?;
            writer.write_event(Event::Text(BytesText::from_plain_str(note)))?;
            writer.write_event(Event::End(BytesEnd::borrowed(b"note")))?;
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
            for component in &outline.components {
                writer.write_event(component.to_event())?;
            }

            for contour in &outline.contours {
                contour.write_xml(&mut writer)?;
            }
            writer.write_event(Event::End(BytesEnd::borrowed(b"outline")))?;
        }

        if let Some(ref image) = self.image {
            writer.write_event(image.to_event())?;
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
        match self {
            Advance::Width(w) => start.push_attribute(("width", w.to_string().as_str())),
            Advance::Height(h) => start.push_attribute(("height", h.to_string().as_str())),
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

        x.map(|x| start.push_attribute(("x", x.to_string().as_str())));
        y.map(|y| start.push_attribute(("y", y.to_string().as_str())));
        angle.map(|angle| start.push_attribute(("angle", angle.to_string().as_str())));

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
        start.push_attribute(("base", self.base.as_str()));
        start.push_attribute(("xScale", self.transform.x_scale.to_string().as_str()));
        start.push_attribute(("yScale", self.transform.y_scale.to_string().as_str()));
        start.push_attribute(("xyScale", self.transform.xy_scale.to_string().as_str()));
        start.push_attribute(("yxScale", self.transform.yx_scale.to_string().as_str()));
        start.push_attribute(("xOffset", self.transform.x_offset.to_string().as_str()));
        start.push_attribute(("yOffset", self.transform.y_offset.to_string().as_str()));

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
        let smooth = if self.smooth { "yes" } else { "no" };
        start.push_attribute(("smooth", smooth));
        start.push_attribute(("type", self.typ.as_str()));

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
        start.push_attribute(("xScale", self.transform.x_scale.to_string().as_str()));
        start.push_attribute(("yScale", self.transform.y_scale.to_string().as_str()));
        start.push_attribute(("xyScale", self.transform.xy_scale.to_string().as_str()));
        start.push_attribute(("yxScale", self.transform.yx_scale.to_string().as_str()));
        start.push_attribute(("xOffset", self.transform.x_offset.to_string().as_str()));
        start.push_attribute(("yOffset", self.transform.y_offset.to_string().as_str()));

        if let Some(color) = &self.color {
            start.push_attribute(("color", color.to_rgba_string().as_str()));
        }
        Event::Empty(start)
    }
}

fn char_to_event(c: &char) -> Event<'static> {
    let mut start = BytesStart::borrowed_name(b"unicode");
    let hex = format!("{:04X}", *c as u32);
    start.push_attribute(("hex", hex.as_str()));
    Event::Empty(start)
}
