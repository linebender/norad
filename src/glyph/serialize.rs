//! Writing out .glif files

use std::io::{Cursor, Write};

use quick_xml::{
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
    Error as XmlError, Writer,
};

use super::PUBLIC_OBJECT_LIBS_KEY;
use crate::{
    AffineTransform, Anchor, Color, Component, Contour, ContourPoint, GlifVersion, Glyph,
    Guideline, Image, Line, Plist, PointType,
};

use crate::error::{GlifWriteError, WriteError};

impl Glyph {
    /// Serialize the glyph into an XML byte stream.
    ///
    /// The order of elements and attributes follows [ufonormalizer] where possible.
    ///
    /// [ufonormalizer]: https://github.com/unified-font-object/ufoNormalizer/
    pub fn encode_xml(&self) -> Result<Vec<u8>, GlifWriteError> {
        self.encode_xml_impl().map_err(|inner| GlifWriteError { name: self.name.clone(), inner })
    }

    fn encode_xml_impl(&self) -> Result<Vec<u8>, WriteError> {
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b'\t', 1);
        writer.write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"UTF-8"), None)))?;
        let mut start = BytesStart::borrowed_name(b"glyph");
        start.push_attribute(("name", &*self.name));
        start.push_attribute(("format", self.format.as_str()));
        writer.write_event(Event::Start(start))?;

        for codepoint in &self.codepoints {
            writer.write_event(char_to_event(*codepoint))?;
        }

        // Skip serializing advance if both values are zero, infinite, subnormal, or NaN.
        if self.width.is_normal() || self.height.is_normal() {
            let mut start = BytesStart::borrowed_name(b"advance");
            if self.width != 0. {
                start.push_attribute(("width", self.width.to_string().as_str()));
            }
            if self.height != 0. {
                start.push_attribute(("height", self.height.to_string().as_str()));
            }
            writer.write_event(Event::Empty(start))?;
        }

        if let Some(ref image) = self.image {
            writer.write_event(image.to_event())?;
        }

        if !self.contours.is_empty() || !self.components.is_empty() {
            writer.write_event(Event::Start(BytesStart::borrowed_name(b"outline")))?;
            for contour in &self.contours {
                contour.write_xml(&mut writer)?;
            }
            for component in &self.components {
                writer.write_event(component.to_event())?;
            }
            writer.write_event(Event::End(BytesEnd::borrowed(b"outline")))?;
        }

        for anchor in &self.anchors {
            writer.write_event(anchor.to_event())?;
        }

        for guide in &self.guidelines {
            writer.write_event(guide.to_event())?;
        }

        // Object libs are treated specially. The UFO v3 format won't allow us
        // to store them inline, so they have to be placed into the glyph's lib
        // under the public.objectLibs parent key. To avoid mutation behind the
        // client's back, object libs are written out but not stored in
        // glyph.lib in-memory. If there are object libs to serialize, clone the
        // existing lib and insert them there for serialization, otherwise avoid
        // cloning and write out the original.
        let object_libs = self.dump_object_libs();
        if !object_libs.is_empty() {
            let mut new_lib = self.lib.clone();
            new_lib.insert(PUBLIC_OBJECT_LIBS_KEY.into(), plist::Value::Dictionary(object_libs));
            write_lib_section(&new_lib, &mut writer)?;
        } else if !self.lib.is_empty() {
            write_lib_section(&self.lib, &mut writer)?;
        }

        if let Some(ref note) = self.note {
            writer.write_event(Event::Start(BytesStart::borrowed_name(b"note")))?;
            writer.write_event(Event::Text(BytesText::from_plain_str(note)))?;
            writer.write_event(Event::End(BytesEnd::borrowed(b"note")))?;
        }

        writer.write_event(Event::End(BytesEnd::borrowed(b"glyph")))?;
        writer.inner().write_all("\n".as_bytes())?;
        writer.inner().flush()?;

        Ok(writer.into_inner().into_inner())
    }
}

/// Writing out the embedded lib plist that a glif may have.
///
/// To write the lib section we write the lib as a plist to an empty buffer,
/// and then we strip out the leading and trailing bits that we don't need,
/// such as the xml declaration and the <plist> tag.
///
/// We then take this and write it into the middle of our active write session.
///
/// By a lovely coincidence the whitespace is the same in both places; if this
/// changes we will need to do custom whitespace handling.
fn write_lib_section<T: Write>(lib: &Plist, writer: &mut Writer<T>) -> Result<(), WriteError> {
    let as_value: plist::Value = lib.to_owned().into();
    let mut out_buffer = Vec::with_capacity(256); // a reasonable min size?
    as_value.to_writer_xml(&mut out_buffer)?;
    let lib_xml = String::from_utf8(out_buffer).expect("xml writer writs valid utf8");
    let header = "<plist version=\"1.0\">\n";
    let footer = "\n</plist>";
    let start_idx = lib_xml
        .find(header)
        .map(|pos| pos + header.len())
        .ok_or(WriteError::InternalLibWriteError)?;
    let end_idx = lib_xml.find(footer).ok_or(WriteError::InternalLibWriteError)?;
    let to_write = &lib_xml[start_idx..end_idx];

    writer.write_event(Event::Start(BytesStart::borrowed_name(b"lib")))?;
    for line in to_write.lines() {
        writer.inner().write_all("\n\t\t".as_bytes())?;
        writer.inner().write_all(line.as_bytes())?;
    }
    writer.write_event(Event::End(BytesEnd::borrowed(b"lib")))?;
    Ok(())
}

impl GlifVersion {
    fn as_str(&self) -> &str {
        match self {
            GlifVersion::V1 => "1",
            GlifVersion::V2 => "2",
        }
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

        if let Some(name) = &self.name {
            start.push_attribute(("name", name.as_str()));
        }

        if let Some(x) = x {
            start.push_attribute(("x", x.to_string().as_str()))
        }

        if let Some(y) = y {
            start.push_attribute(("y", y.to_string().as_str()))
        }

        if let Some(angle) = angle {
            start.push_attribute(("angle", angle.to_string().as_str()))
        }

        if let Some(color) = &self.color {
            start.push_attribute(("color", color.to_rgba_string().as_str()));
        }

        if let Some(id) = &self.identifier() {
            start.push_attribute(("identifier", id.as_str()));
        }
        Event::Empty(start)
    }
}

impl Anchor {
    fn to_event(&self) -> Event {
        let mut start = BytesStart::borrowed_name(b"anchor");

        if let Some(name) = &self.name {
            start.push_attribute(("name", name.as_str()));
        }

        start.push_attribute(("x", self.x.to_string().as_str()));
        start.push_attribute(("y", self.y.to_string().as_str()));

        if let Some(color) = &self.color {
            start.push_attribute(("color", color.to_rgba_string().as_str()));
        }

        if let Some(id) = &self.identifier {
            start.push_attribute(("identifier", id.as_str()));
        }

        Event::Empty(start)
    }
}

impl Component {
    fn to_event(&self) -> Event {
        let mut start = BytesStart::borrowed_name(b"component");
        start.push_attribute(("base", &*self.base));

        write_transform_attributes(&mut start, &self.transform);

        if let Some(id) = &self.identifier {
            start.push_attribute(("identifier", id.as_str()));
        }
        Event::Empty(start)
    }
}

impl Contour {
    fn write_xml<T: Write>(&self, writer: &mut Writer<T>) -> Result<(), XmlError> {
        let mut start = BytesStart::borrowed_name(b"contour");

        if let Some(id) = &self.identifier {
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

        if let Some(name) = &self.name {
            start.push_attribute(("name", name.as_str()));
        }

        start.push_attribute(("x", self.x.to_string().as_str()));
        start.push_attribute(("y", self.y.to_string().as_str()));

        match self.typ {
            PointType::OffCurve => {}
            _ => start.push_attribute(("type", self.typ.as_str())),
        }

        if self.smooth {
            start.push_attribute(("smooth", "yes"));
        }

        if let Some(id) = &self.identifier {
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
    /// Serializes the color into a string as defined by the [UFO specification][0].
    /// Precision is limited to three decimal places, which is enough to losslessly
    /// roundtrip to colors represented by `u8` tuples.
    ///
    /// [0]: https://unifiedfontobject.org/versions/ufo3/conventions/#colors
    pub fn to_rgba_string(&self) -> String {
        use std::fmt::Write;

        // TODO: Check that all channels are 0.0..=1.0
        let mut result = String::new();
        let mut scratch = String::new();
        let Color { red, green, blue, alpha } = self;
        for channel in &[red, green, blue, alpha] {
            if !result.is_empty() {
                result.push(',');
            }

            scratch.clear();
            // This can only fail on an allocation error, in which case we have other problems.
            let _ = write!(&mut scratch, "{:.3}", channel);
            result.push_str(scratch.trim_end_matches('0').trim_end_matches('.'));
        }
        result
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
