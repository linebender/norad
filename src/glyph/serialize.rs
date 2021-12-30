//! Writing out .glif files

use std::io::{Cursor, Write};

use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Error as XmlError, Writer,
};

use super::PUBLIC_OBJECT_LIBS_KEY;
use crate::{
    util, AffineTransform, Anchor, Color, Component, Contour, ContourPoint, GlifVersion, Glyph,
    Guideline, Image, Line, Plist, PointType, WriteOptions,
};

use crate::error::GlifWriteError;
use crate::write::QuoteChar;

impl Glyph {
    /// Serialize the glyph into an XML byte stream.
    ///
    /// The order of elements and attributes follows [ufonormalizer] where possible.
    ///
    /// [ufonormalizer]: https://github.com/unified-font-object/ufoNormalizer/
    pub fn encode_xml(&self) -> Result<Vec<u8>, GlifWriteError> {
        let options = WriteOptions::default();
        self.encode_xml_with_options(&options)
    }

    /// Serialize the glyph into an XML byte stream with custom string formatting.
    ///
    /// The order of elements and attributes follows [ufonormalizer] where possible.
    ///
    /// [ufonormalizer]: https://github.com/unified-font-object/ufoNormalizer/
    pub fn encode_xml_with_options(&self, opts: &WriteOptions) -> Result<Vec<u8>, GlifWriteError> {
        self.encode_xml_impl(opts)
    }

    fn encode_xml_impl(&self, options: &WriteOptions) -> Result<Vec<u8>, GlifWriteError> {
        let mut writer = Writer::new_with_indent(
            Cursor::new(Vec::new()),
            options.whitespace_char,
            options.whitespace_count,
        );
        match options.quote_style {
            QuoteChar::Double => writer
                .write(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")
                .map_err(GlifWriteError::Xml)?,
            QuoteChar::Single => writer
                .write(b"<?xml version='1.0' encoding='UTF-8'?>\n")
                .map_err(GlifWriteError::Xml)?,
        }
        let mut start = BytesStart::borrowed_name(b"glyph");
        start.push_attribute(("name", &*self.name));
        start.push_attribute(("format", self.format.as_str()));
        writer.write_event(Event::Start(start)).map_err(GlifWriteError::Xml)?;

        for codepoint in &self.codepoints {
            writer.write_event(char_to_event(*codepoint)).map_err(GlifWriteError::Xml)?;
        }

        // Skip serializing advance if both values are zero, infinite, subnormal, or NaN.
        if self.width.is_normal() || self.height.is_normal() {
            let mut start = BytesStart::borrowed_name(b"advance");
            if self.height != 0. {
                start.push_attribute(("height", self.height.to_string().as_str()));
            }
            if self.width != 0. {
                start.push_attribute(("width", self.width.to_string().as_str()));
            }
            writer.write_event(Event::Empty(start)).map_err(GlifWriteError::Xml)?;
        }

        if let Some(ref image) = self.image {
            writer.write_event(image.to_event()).map_err(GlifWriteError::Xml)?;
        }

        if !self.contours.is_empty() || !self.components.is_empty() {
            writer
                .write_event(Event::Start(BytesStart::borrowed_name(b"outline")))
                .map_err(GlifWriteError::Xml)?;
            for contour in &self.contours {
                contour.write_xml(&mut writer).map_err(GlifWriteError::Xml)?;
            }
            for component in &self.components {
                writer.write_event(component.to_event()).map_err(GlifWriteError::Xml)?;
            }
            writer
                .write_event(Event::End(BytesEnd::borrowed(b"outline")))
                .map_err(GlifWriteError::Xml)?;
        }

        for anchor in &self.anchors {
            writer.write_event(anchor.to_event()).map_err(GlifWriteError::Xml)?;
        }

        for guide in &self.guidelines {
            writer.write_event(guide.to_event()).map_err(GlifWriteError::Xml)?;
        }

        // Object libs are treated specially. The UFO v3 format won't allow us
        // to store them inline, so they have to be placed into the glyph's lib
        // under the public.objectLibs parent key. To avoid mutation behind the
        // client's back, object libs are written out but not stored in
        // glyph.lib in-memory. If there are object libs to serialize, clone the
        // existing lib and insert them there for serialization, otherwise avoid
        // cloning and write out the original.
        let mut lib = self.lib.clone();
        let object_libs = self.dump_object_libs();
        if !object_libs.is_empty() {
            lib.insert(PUBLIC_OBJECT_LIBS_KEY.into(), object_libs.into());
        }

        if !lib.is_empty() {
            util::recursive_sort_plist_keys(&mut lib);
            write_lib_section(lib, &mut writer, options)?;
        }

        if let Some(ref note) = self.note {
            writer
                .write_event(Event::Start(BytesStart::borrowed_name(b"note")))
                .map_err(GlifWriteError::Xml)?;
            writer
                .write_event(Event::Text(BytesText::from_plain_str(note)))
                .map_err(GlifWriteError::Xml)?;
            writer
                .write_event(Event::End(BytesEnd::borrowed(b"note")))
                .map_err(GlifWriteError::Xml)?;
        }

        writer
            .write_event(Event::End(BytesEnd::borrowed(b"glyph")))
            .map_err(GlifWriteError::Xml)?;
        writer.inner().write_all("\n".as_bytes()).map_err(GlifWriteError::Buffer)?;
        writer.inner().flush().map_err(GlifWriteError::Buffer)?;

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
fn write_lib_section<T: Write>(
    lib: Plist,
    writer: &mut Writer<T>,
    options: &WriteOptions,
) -> Result<(), GlifWriteError> {
    let as_value: plist::Value = lib.into();
    let mut out_buffer = Vec::with_capacity(256); // a reasonable min size?
    as_value
        .to_writer_xml_with_options(&mut out_buffer, options.xml_options())
        .map_err(GlifWriteError::Plist)?;
    let lib_xml = String::from_utf8(out_buffer).expect("XML writer wrote invalid UTF-8");
    let header = "<plist version=\"1.0\">\n";
    let footer = "\n</plist>";
    let start_idx = lib_xml
        .find(header)
        .map(|pos| pos + header.len())
        .ok_or(GlifWriteError::InternalLibWriteError)?;
    let end_idx = lib_xml.find(footer).ok_or(GlifWriteError::InternalLibWriteError)?;
    let to_write = &lib_xml[start_idx..end_idx];

    writer
        .write_event(Event::Start(BytesStart::borrowed_name(b"lib")))
        .map_err(GlifWriteError::Xml)?;
    for line in to_write.lines() {
        writer.inner().write_all("\n".as_bytes()).map_err(GlifWriteError::Buffer)?;
        writer.inner().write_all(options.indent_str.as_bytes()).map_err(GlifWriteError::Buffer)?;
        writer.inner().write_all(options.indent_str.as_bytes()).map_err(GlifWriteError::Buffer)?;
        writer.inner().write_all(line.as_bytes()).map_err(GlifWriteError::Buffer)?;
    }
    writer.write_event(Event::End(BytesEnd::borrowed(b"lib"))).map_err(GlifWriteError::Xml)?;
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
    if (transform.x_scale - 1.0).abs() > std::f64::EPSILON {
        element.push_attribute(("xScale", transform.x_scale.to_string().as_str()));
    }

    if transform.xy_scale != 0.0 {
        element.push_attribute(("xyScale", transform.xy_scale.to_string().as_str()));
    }

    if transform.yx_scale != 0.0 {
        element.push_attribute(("yxScale", transform.yx_scale.to_string().as_str()));
    }

    if (transform.y_scale - 1.0).abs() > std::f64::EPSILON {
        element.push_attribute(("yScale", transform.y_scale.to_string().as_str()));
    }

    if transform.x_offset != 0.0 {
        element.push_attribute(("xOffset", transform.x_offset.to_string().as_str()));
    }

    if transform.y_offset != 0.0 {
        element.push_attribute(("yOffset", transform.y_offset.to_string().as_str()));
    }
}
