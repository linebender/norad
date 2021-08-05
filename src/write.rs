//! Customize serialization behaviour

use std::{borrow::Cow, fs::File, io::BufWriter, path::Path};

use plist::XmlWriteOptions;

use crate::Error;

/// Options that can be set when writing the UFO to disk.
///
/// You construct `WriteOptions` using builder semantics:
///
/// ```
/// # use norad::WriteOptions;
/// let single_tab = WriteOptions::default();
///
/// let two_tabs = WriteOptions::default()
///     .whitespace("\t\t");
///
/// let spaces = WriteOptions::default()
///     .whitespace("  ");
/// ```
#[derive(Debug, Clone)]
pub struct WriteOptions {
    // for annoying reasons we store three different representations.
    pub(crate) indent_str: Cow<'static, str>,
    xml_opts: XmlWriteOptions,
    pub(crate) whitespace_char: u8,
    pub(crate) whitespace_count: usize,
}

impl Default for WriteOptions {
    fn default() -> Self {
        WriteOptions {
            indent_str: "\t".into(),
            xml_opts: Default::default(),
            whitespace_char: b'\t',
            whitespace_count: 1,
        }
    }
}

impl WriteOptions {
    /// Builder-style method to customize the whitespace.
    ///
    /// By default, we intent with a single tab ("\t").
    ///
    /// The argument, may be either a `'static str` or a `String`. You should
    /// prefer to use a `'static str` where possible.
    ///
    /// The string can contain any number of *a single ASCII character*, but must
    /// not contain multiple different characters. As an example, "\t\t" is
    /// fine, but "\t  \t" is not, because it contains both tabs and spaces.
    ///
    /// This is not good API, but is a work around for the fact that the quick-xml
    /// and plist crates both represent whitespace in different ways.
    ///
    /// # Panics
    ///
    /// Panics if the provided string is empty, or if it contains multiple
    /// different characters.
    pub fn whitespace(mut self, indent_str: impl Into<Cow<'static, str>>) -> Self {
        let indent_str = indent_str.into();
        self.whitespace_char = indent_str.bytes().next().expect("whitespace str must not be empty");
        assert!(indent_str.bytes().all(|c| c == self.whitespace_char), "invalid whitespace");
        self.whitespace_count = indent_str.len();
        self.indent_str = indent_str;
        self.xml_opts = XmlWriteOptions::default().indent_string(self.indent_str.clone());
        self
    }

    /// Return a reference to [`XmlWriteOptions`] for use with the `plist` crate.
    pub fn xml_options(&self) -> &XmlWriteOptions {
        &self.xml_opts
    }
}

/// Write a `plist::Value` to file, providing custom options.
pub fn write_plist_value_to_file(
    path: &Path,
    value: &plist::Value,
    options: &XmlWriteOptions,
) -> Result<(), Error> {
    let mut file = File::create(path)?;
    let writer = BufWriter::new(&mut file);
    value.to_writer_xml_with_options(writer, options)?;
    file.sync_all()?;
    Ok(())
}

/// Write any `Serialize` to file, providing custom options.
pub fn write_xml_to_file(
    path: &Path,
    value: &impl serde::Serialize,
    options: &XmlWriteOptions,
) -> Result<(), Error> {
    let mut file = File::create(path)?;
    {
        let buf_writer = BufWriter::new(&mut file);
        let writer = plist::stream::XmlWriter::new_with_options(buf_writer, options);
        let mut ser = plist::Serializer::new(writer);
        value.serialize(&mut ser)?;
    }
    file.sync_all()?;
    Ok(())
}
