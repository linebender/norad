//! Customize serialization behaviour

use std::{borrow::Cow, path::Path};

use plist::XmlWriteOptions;

use crate::font_sink::FontSink;

/// Options that can be set when writing the UFO to disk.
///
/// You construct `WriteOptions` using builder semantics:
///
/// ```
/// # use norad::{QuoteChar, WriteOptions};
/// let single_tab = WriteOptions::default();
///
/// let two_tabs = WriteOptions::default()
///     .whitespace("\t\t");
///
/// let spaces = WriteOptions::default()
///     .whitespace("  ");
///
/// let spaces_and_singlequotes = WriteOptions::default()
///     .whitespace("  ")
///     .quote_char(QuoteChar::Single);
/// ```
#[derive(Debug, Clone)]
pub struct WriteOptions {
    xml_opts: XmlWriteOptions,
    pub(crate) indent_char: u8,
    pub(crate) indent_count: usize,
    pub(crate) quote_style: QuoteChar,
}

impl Default for WriteOptions {
    fn default() -> Self {
        WriteOptions {
            xml_opts: Default::default(),
            indent_char: WriteOptions::TAB,
            indent_count: 1,
            quote_style: QuoteChar::Double,
        }
    }
}

impl WriteOptions {
    /// The ASCII value of the space character (0x20)
    pub const SPACE: u8 = b' ';
    /// The ASCII value of the tab character (0x09)
    pub const TAB: u8 = b'\t';

    /// Create new, default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// **deprecated**. Use [`WriteOptions::indent`] instead.
    ///
    /// Builder-style method to customize the whitespace.
    ///
    /// By default, we indent with a single tab ("\t").
    ///
    /// The argument, may be either a `'static str` or a `String`. You should
    /// prefer to use a `'static str` where possible.
    ///
    /// The string can contain any number of *a single ASCII character*, but must
    /// not contain multiple different characters. As an example, "\t\t" is
    /// fine, but "\t  \t" is not, because it contains both tabs and spaces.
    ///
    // This is not good API, but is a work around for the fact that the quick-xml
    // and plist crates both represent whitespace in different ways.
    ///
    /// # Panics
    ///
    /// Panics if the provided string is empty, or if it contains multiple
    /// different characters.
    #[deprecated(since = "0.8.1", note = "use 'indent' method instead")]
    pub fn whitespace(self, indent_str: impl Into<Cow<'static, str>>) -> Self {
        let indent_str = indent_str.into();
        let indent_char = indent_str.bytes().next().expect("whitespace str must not be empty");
        assert!(indent_str.bytes().all(|c| c == indent_char), "invalid whitespace");
        let indent_count = indent_str.len();
        self.indent(indent_char, indent_count)
    }

    /// Customize the indent whitespace.
    ///
    /// By default, we indent with a single tab (`\t`). You can use this method
    /// to specify alternative indent behaviour.
    ///
    /// # Panics
    ///
    /// Panics if the provided `indent_char` is not one of 0x09 (tab) or 0x20 (space).
    ///
    /// # Example
    ///
    /// Indenting with four spaces:
    /// ```
    /// use norad::WriteOptions;
    /// let options = WriteOptions::new().indent(WriteOptions::SPACE, 4);
    /// ```
    pub fn indent(mut self, indent_char: u8, indent_count: usize) -> Self {
        assert!([WriteOptions::TAB, WriteOptions::SPACE].contains(&indent_char));
        self.indent_char = indent_char;
        self.indent_count = indent_count;
        self.xml_opts = XmlWriteOptions::default().indent(indent_char, indent_count);
        self
    }

    /// Builder-style method to customize the XML declaration attribute definition quote
    /// char.
    ///
    /// By default, we indent with double quotes.
    ///
    /// The quote style is defined with a [`QuoteChar`] enum argument.
    pub fn quote_char(mut self, quote_style: QuoteChar) -> Self {
        self.quote_style = quote_style;
        self
    }

    /// Return a reference to [`XmlWriteOptions`] for use with the `plist` crate.
    pub fn xml_options(&self) -> &XmlWriteOptions {
        &self.xml_opts
    }

    pub(crate) fn write_indent(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        for _ in 0..self.indent_count {
            writer.write_all(&[self.indent_char])?;
        }
        Ok(())
    }
}

/// The quote character used to write the XML declaration.
///
/// This is exposed to allow the user to match the output of other tools.
#[derive(Debug, Clone, Copy)]
pub enum QuoteChar {
    /// Single quotes: 'UTF-8'.
    Single,
    /// Double quotes: "UTF-8".
    Double,
}

/// Dump any `Serialize` to an in-memory XML plist, providing custom options.
pub(crate) fn serialize_xml(
    value: &impl serde::Serialize,
    options: &WriteOptions,
) -> Result<Vec<u8>, CustomSerializationError> {
    let mut buf = Vec::new();
    plist::to_writer_xml_with_options(&mut buf, value, options.xml_options())
        .map_err(CustomSerializationError::SerializePlist)?;
    apply_quote_style(&mut buf, options)?;
    Ok(buf)
}

/// Dump any `Serialize` to an XML plist and write it to the sink.
pub(crate) fn write_xml_to_sink(
    sink: &dyn FontSink,
    path: &Path,
    value: &impl serde::Serialize,
    options: &WriteOptions,
) -> Result<(), CustomSerializationError> {
    let data = serialize_xml(value, options)?;
    sink.write(path, &data).map_err(CustomSerializationError::Write)
}

/// Rewrite the XML declaration with custom quote formatting options.
fn apply_quote_style(
    xml: &mut [u8],
    options: &WriteOptions,
) -> Result<(), CustomSerializationError> {
    const DOUBLE_QUOTE_DECL: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>";
    const SINGLE_QUOTE_DECL: &[u8] = b"<?xml version='1.0' encoding='UTF-8'?>";
    match options.quote_style {
        QuoteChar::Single => {
            // the plist crate always writes the double-quoted declaration,
            // which is the same length as the single-quoted one; but don't
            // overwrite it blindly in case that ever changes
            if !xml.starts_with(DOUBLE_QUOTE_DECL) {
                return Err(CustomSerializationError::UnexpectedXmlDeclaration);
            }
            xml[..SINGLE_QUOTE_DECL.len()].copy_from_slice(SINGLE_QUOTE_DECL);
        }
        QuoteChar::Double => (), // double quote is the default style
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CustomSerializationError {
    #[error("failed to serialize Plist")]
    SerializePlist(#[source] plist::Error),
    #[error("plist serialization emitted an unrecognized XML declaration, please open an issue")]
    UnexpectedXmlDeclaration,
    #[error("failed to write file")]
    Write(#[source] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use plist::Value;

    /// Write `value` to a file on disk through the `&Path` sink, and read it
    /// back, so that these tests exercise the actual save-to-disk path.
    fn write_and_read_back(value: &Value, options: &WriteOptions) -> String {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        write_xml_to_sink(&root, Path::new("test.plist"), value, options).unwrap();
        std::fs::read_to_string(root.join("test.plist")).unwrap()
    }

    #[test]
    fn write_lib_plist_default() {
        let opt = WriteOptions::default();
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/lib.plist")
            .expect("failed to read plist");
        let plist_write = write_and_read_back(&plist_read, &opt);
        let str_list = plist_write.split('\n').collect::<Vec<&str>>();
        assert_eq!(str_list[0], "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"); // default uses double quotes
        assert_eq!(str_list[3], "<dict>"); // no space char at first dict tag
        assert_eq!(str_list[4], "\t<key>com.defcon.sortDescriptor</key>"); // single tab spacing by default
        assert_eq!(str_list[6], "\t\t<dict>"); // second level should use two tab char
    }

    #[test]
    fn write_lib_plist_with_custom_whitespace() {
        let opt = WriteOptions::default().indent(WriteOptions::SPACE, 2);
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/lib.plist")
            .expect("failed to read plist");
        let plist_write = write_and_read_back(&plist_read, &opt);
        let str_list = plist_write.split('\n').collect::<Vec<&str>>();
        assert_eq!(str_list[0], "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"); // default uses double quotes
        assert_eq!(str_list[3], "<dict>"); // no space char at first dict tag
        assert_eq!(str_list[4], "  <key>com.defcon.sortDescriptor</key>"); // should use two space char
        assert_eq!(str_list[6], "    <dict>"); // second level should use four space char
    }

    #[test]
    fn write_lib_plist_with_custom_whitespace_and_single_quotes() {
        let opt =
            WriteOptions::default().indent(WriteOptions::SPACE, 2).quote_char(QuoteChar::Single);
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/lib.plist")
            .expect("failed to read plist");
        let plist_write = write_and_read_back(&plist_read, &opt);
        let str_list = plist_write.split('\n').collect::<Vec<&str>>();
        assert_eq!(str_list[0], "<?xml version='1.0' encoding='UTF-8'?>"); // should use single quotes
        assert_eq!(str_list[3], "<dict>"); // no space char at first dict tag
        assert_eq!(str_list[4], "  <key>com.defcon.sortDescriptor</key>"); // should use two space char
        assert_eq!(str_list[6], "    <dict>"); // second level should use four space char
    }

    #[test]
    fn write_lib_plist_line_endings() {
        let opt = WriteOptions::default();
        let plist_read = Value::from_file("testdata/lineendings/Tester-LineEndings.ufo/lib.plist")
            .expect("failed to read plist");
        let plist_write = write_and_read_back(&plist_read, &opt);
        assert!(plist_write.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n"));
    }

    #[test]
    fn write_fontinfo_plist_default() {
        let opt = WriteOptions::default();
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/fontinfo.plist")
            .expect("failed to read plist");
        let plist_write = write_and_read_back(&plist_read, &opt);
        let str_list = plist_write.split('\n').collect::<Vec<&str>>();
        assert_eq!(str_list[0], "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"); // default uses double quotes
        assert_eq!(str_list[3], "<dict>"); // no space char at first dict tag
        assert_eq!(str_list[4], "\t<key>ascender</key>"); // single tab level spacing by default
    }

    #[test]
    fn write_fontinfo_plist_with_custom_whitespace() {
        let opt = WriteOptions::default().indent(WriteOptions::SPACE, 2);
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/fontinfo.plist")
            .expect("failed to read plist");
        let plist_write = write_and_read_back(&plist_read, &opt);
        let str_list = plist_write.split('\n').collect::<Vec<&str>>();
        assert_eq!(str_list[0], "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"); // default uses double quotes
        assert_eq!(str_list[3], "<dict>"); // no space char at first dict tag
        assert_eq!(str_list[4], "  <key>ascender</key>"); // should use two space char
    }

    #[test]
    fn write_fontinfo_plist_with_custom_whitespace_and_single_quotes() {
        let opt =
            WriteOptions::default().indent(WriteOptions::SPACE, 2).quote_char(QuoteChar::Single);
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/fontinfo.plist")
            .expect("failed to read plist");
        let plist_write = write_and_read_back(&plist_read, &opt);
        let str_list = plist_write.split('\n').collect::<Vec<&str>>();
        assert_eq!(str_list[0], "<?xml version='1.0' encoding='UTF-8'?>"); // should use single quotes
        assert_eq!(str_list[3], "<dict>"); // no space char at first dict tag
        assert_eq!(str_list[4], "  <key>ascender</key>"); // should use two space char
    }

    #[test]
    fn write_fontinfo_plist_line_endings() {
        let opt = WriteOptions::default();
        let plist_read =
            Value::from_file("testdata/lineendings/Tester-LineEndings.ufo/fontinfo.plist")
                .expect("failed to read plist");
        let plist_write = write_and_read_back(&plist_read, &opt);
        assert!(plist_write.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n"));
    }
}
