//! Customize serialization behaviour

use std::io::{prelude::*, Seek, SeekFrom};
use std::{borrow::Cow, fs::File, io::BufWriter, path::Path};

#[cfg(target_family = "unix")]
use std::os::unix::prelude::FileExt;

#[cfg(target_family = "windows")]
use std::os::windows::prelude::*;

use plist::XmlWriteOptions;

use crate::Error;

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
///
/// let use_linefeed_line_endings_on_win = WriteOptions::default()
///     .normalize_line_endings();
/// ```
#[derive(Debug, Clone)]
pub struct WriteOptions {
    // for annoying reasons we store three different representations.
    pub(crate) indent_str: Cow<'static, str>,
    xml_opts: XmlWriteOptions,
    pub(crate) whitespace_char: u8,
    pub(crate) whitespace_count: usize,
    pub(crate) quote_style: QuoteChar,
    pub(crate) line_ending_normalization: bool,
}

impl Default for WriteOptions {
    fn default() -> Self {
        WriteOptions {
            indent_str: "\t".into(),
            xml_opts: Default::default(),
            whitespace_char: b'\t',
            whitespace_count: 1,
            quote_style: QuoteChar::Double,
            line_ending_normalization: false,
        }
    }
}

impl WriteOptions {
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

    /// Builder-style method to normalize line endings to `\n` on all platforms
    ///
    /// By default, non-Windows platforms write line feed (`\n`) line endings and
    /// the Windows platform writes carriage return and line feed (`\r\n`) line endings.
    pub fn normalize_line_endings(mut self) -> Self {
        self.line_ending_normalization = true;
        self
    }

    /// Return a reference to [`XmlWriteOptions`] for use with the `plist` crate.
    pub fn xml_options(&self) -> &XmlWriteOptions {
        &self.xml_opts
    }
}

//
#[derive(Debug, Clone)]
struct CarriageReturnRemover<I> {
    iter: I,
    previous_was_cr: bool,
}

/// Iterator implementation for the CarriageReturnRemover struct
///
/// Iterates through u8 and removes all `\r` to normalize line endings
/// as `\n` only irrespective of the platform.  Returns u8 values.
///
/// This implementation is a u8 iterable derivative of
/// https://github.com/derekdreery/normalize-line-endings/blob/master/src/lib.rs
/// (Apache License v2)
impl<I> Iterator for CarriageReturnRemover<I>
where
    I: Iterator<Item = u8>,
{
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        match self.iter.next() {
            Some(0x000A) if self.previous_was_cr => {
                self.previous_was_cr = false;
                match self.iter.next() {
                    Some(0x000D) => {
                        self.previous_was_cr = true;
                        Some(0x000A)
                    }
                    any => {
                        self.previous_was_cr = false;
                        any
                    }
                }
            }
            Some(0x000D) => {
                self.previous_was_cr = true;
                Some(0x000A)
            }
            any => {
                self.previous_was_cr = false;
                any
            }
        }
    }
}

/// The quote character used to write the XML declaration.
///
/// This is exposed to allow the user to match the output of other tools.
#[derive(Debug, Clone)]
pub enum QuoteChar {
    /// Single quotes: 'UTF-8'.
    Single,
    /// Double quotes: "UTF-8".
    Double,
}

/// Write a `plist::Value` to file, providing custom options.
pub fn write_plist_value_to_file(
    path: &Path,
    value: &plist::Value,
    options: &WriteOptions,
) -> Result<(), Error> {
    let mut file = File::create(path)?;
    let writer = BufWriter::new(&mut file);
    value.to_writer_xml_with_options(writer, options.xml_options())?;
    write_quote_style(&file, options)?;
    normalize_plist_line_endings(&mut file, options)?;
    file.sync_all()?;
    Ok(())
}

/// Write any `Serialize` to file, providing custom options.
pub fn write_xml_to_file(
    path: &Path,
    value: &impl serde::Serialize,
    options: &WriteOptions,
) -> Result<(), Error> {
    let mut file = File::create(path)?;
    let buf_writer = BufWriter::new(&mut file);
    plist::to_writer_xml_with_options(buf_writer, value, options.xml_options())?;
    write_quote_style(&file, options)?;
    normalize_plist_line_endings(&mut file, options)?;
    file.sync_all()?;
    Ok(())
}

/// Write optional XML declaration single quote attribute format
pub fn write_quote_style(file: &File, options: &WriteOptions) -> Result<(), Error> {
    // Optionally modify the XML declaration quote style
    match options.quote_style {
        QuoteChar::Single => {
            // Unix platform specific write
            #[cfg(target_family = "unix")]
            file.write_at(b"<?xml version='1.0' encoding='UTF-8'?>", 0)?;
            // Windows platform specific write
            #[cfg(target_family = "windows")]
            file.seek_write(b"<?xml version='1.0' encoding='UTF-8'?>", 0)?;
        }
        QuoteChar::Double => (), // double quote is the default style
    }
    Ok(())
}

/// Write optional normalized `\n` line endings on all platforms
///
/// This is a Windows-only implementation that changes the line ending style from `\r\n` (default)
/// to `\n`.
pub fn normalize_plist_line_endings(file: &mut File, options: &WriteOptions) -> Result<(), Error> {
    // Optionally remove the platform-specific Win style `\r\n` line endings
    // serialized on Win platform only
    if !cfg!(windows) {
        // non-Windows platforms write line feed char line endings by default
        // nothing to do unless we are in a Win environment
        Ok(())
    } else {
        if options.line_ending_normalization {
            file.seek(SeekFrom::Start(0)).unwrap();
            let removed_cr_byte_vec =
                remove_carriage_returns(file.bytes().map(|b| b.unwrap())).collect::<Vec<u8>>();
            match file.write_all(&removed_cr_byte_vec) {
                Ok(_) => return Ok(()),
                Err(e) => return Err(Error::IoError(e)),
            }
        }
        Ok(())
    }
}

/// Returns an iterator over u8 with all carriage return (`\r`) u8 values removed from an
/// iterator over u8 argument
#[inline]
pub fn remove_carriage_returns(iter: impl Iterator<Item = u8>) -> impl Iterator<Item = u8> {
    CarriageReturnRemover { iter, previous_was_cr: false }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plist::Value;
    use std::fs;
    use tempdir::TempDir;

    #[test]
    fn write_lib_plist_default() -> Result<(), Error> {
        let opt = WriteOptions::default();
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/lib.plist")
            .expect("failed to read plist");
        let tmp = TempDir::new("test")?;
        let filepath = tmp.path().join("lib.plist");
        write_plist_value_to_file(&filepath, &plist_read, &opt)?;
        let plist_write = fs::read_to_string(filepath)?;
        let str_list = plist_write.split('\n').collect::<Vec<&str>>();
        assert_eq!(str_list[0], "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"); // default uses double quotes
        assert_eq!(str_list[3], "<dict>"); // no space char at first dict tag
        assert_eq!(str_list[4], "\t<key>com.defcon.sortDescriptor</key>"); // single tab spacing by default
        assert_eq!(str_list[6], "\t\t<dict>"); // second level should use two tab char
        tmp.close()?;
        Ok(())
    }

    #[test]
    fn write_lib_plist_with_custom_whitespace() -> Result<(), Error> {
        let opt = WriteOptions::default().whitespace("  ");
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/lib.plist")
            .expect("failed to read plist");
        let tmp = TempDir::new("test")?;
        let filepath = tmp.path().join("lib.plist");
        write_plist_value_to_file(&filepath, &plist_read, &opt)?;
        let plist_write = fs::read_to_string(filepath)?;
        let str_list = plist_write.split('\n').collect::<Vec<&str>>();
        assert_eq!(str_list[0], "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"); // default uses double quotes
        assert_eq!(str_list[3], "<dict>"); // no space char at first dict tag
        assert_eq!(str_list[4], "  <key>com.defcon.sortDescriptor</key>"); // should use two space char
        assert_eq!(str_list[6], "    <dict>"); // second level should use four space char
        tmp.close()?;
        Ok(())
    }

    #[test]
    fn write_lib_plist_with_custom_whitespace_and_single_quotes() -> Result<(), Error> {
        let opt = WriteOptions::default().whitespace("  ").quote_char(QuoteChar::Single);
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/lib.plist")
            .expect("failed to read plist");
        let tmp = TempDir::new("test")?;
        let filepath = tmp.path().join("lib.plist");
        write_plist_value_to_file(&filepath, &plist_read, &opt)?;
        let plist_write = fs::read_to_string(filepath)?;
        let str_list = plist_write.split('\n').collect::<Vec<&str>>();
        assert_eq!(str_list[0], "<?xml version='1.0' encoding='UTF-8'?>"); // should use single quotes
        assert_eq!(str_list[3], "<dict>"); // no space char at first dict tag
        assert_eq!(str_list[4], "  <key>com.defcon.sortDescriptor</key>"); // should use two space char
        assert_eq!(str_list[6], "    <dict>"); // second level should use four space char
        tmp.close()?;
        Ok(())
    }

    #[test]
    fn write_lib_plist_with_normalized_line_endings() -> Result<(), Error> {
        let opt = WriteOptions::default().normalize_line_endings();
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/lib.plist")
            .expect("failed to read plist");
        let tmp = TempDir::new("test")?;
        let filepath = tmp.path().join("lib.plist");
        write_plist_value_to_file(&filepath, &plist_read, &opt)?;
        let plist_write = fs::read_to_string(filepath)?;
        assert!(plist_write.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n"));
        tmp.close()?;
        Ok(())
    }

    #[test]
    fn write_fontinfo_plist_default() -> Result<(), Error> {
        let opt = WriteOptions::default();
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/fontinfo.plist")
            .expect("failed to read plist");
        let tmp = TempDir::new("test")?;
        let filepath = tmp.path().join("fontinfo.plist");
        write_plist_value_to_file(&filepath, &plist_read, &opt)?;
        let plist_write = fs::read_to_string(filepath)?;
        let str_list = plist_write.split('\n').collect::<Vec<&str>>();
        assert_eq!(str_list[0], "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"); // default uses double quotes
        assert_eq!(str_list[3], "<dict>"); // no space char at first dict tag
        assert_eq!(str_list[4], "\t<key>ascender</key>"); // single tab level spacing by default
        tmp.close()?;
        Ok(())
    }

    #[test]
    fn write_fontinfo_plist_with_custom_whitespace() -> Result<(), Error> {
        let opt = WriteOptions::default().whitespace("  ");
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/fontinfo.plist")
            .expect("failed to read plist");
        let tmp = TempDir::new("test")?;
        let filepath = tmp.path().join("fontinfo.plist");
        write_plist_value_to_file(&filepath, &plist_read, &opt)?;
        let plist_write = fs::read_to_string(filepath)?;
        let str_list = plist_write.split('\n').collect::<Vec<&str>>();
        assert_eq!(str_list[0], "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"); // default uses double quotes
        assert_eq!(str_list[3], "<dict>"); // no space char at first dict tag
        assert_eq!(str_list[4], "  <key>ascender</key>"); // should use two space char
        tmp.close()?;
        Ok(())
    }

    #[test]
    fn write_fontinfo_plist_with_custom_whitespace_and_single_quotes() -> Result<(), Error> {
        let opt = WriteOptions::default().whitespace("  ").quote_char(QuoteChar::Single);
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/fontinfo.plist")
            .expect("failed to read plist");
        let tmp = TempDir::new("test")?;
        let filepath = tmp.path().join("fontinfo.plist");
        write_plist_value_to_file(&filepath, &plist_read, &opt)?;
        let plist_write = fs::read_to_string(filepath)?;
        let str_list = plist_write.split('\n').collect::<Vec<&str>>();
        assert_eq!(str_list[0], "<?xml version='1.0' encoding='UTF-8'?>"); // should use single quotes
        assert_eq!(str_list[3], "<dict>"); // no space char at first dict tag
        assert_eq!(str_list[4], "  <key>ascender</key>"); // should use two space char
        tmp.close()?;
        Ok(())
    }

    #[test]
    fn write_fontinfo_plist_normalized_line_endings() -> Result<(), Error> {
        let opt = WriteOptions::default();
        let plist_read = Value::from_file("testdata/MutatorSansLightWide.ufo/fontinfo.plist")
            .expect("failed to read plist");
        let tmp = TempDir::new("test")?;
        let filepath = tmp.path().join("fontinfo.plist");
        write_plist_value_to_file(&filepath, &plist_read, &opt)?;
        let plist_write = fs::read_to_string(filepath)?;
        assert!(plist_write.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n"));
        tmp.close()?;
        Ok(())
    }

    #[test]
    fn remove_carriage_returns_pub_fn() {
        let input = b"This is a string \n with \r some \n\r\n random newlines\r\r\n\n";
        let res_vec =
            remove_carriage_returns(input.iter().map(|b| b.to_owned())).collect::<Vec<u8>>();
        assert_eq!(&res_vec, b"This is a string \n with \n some \n\n random newlines\n\n\n");
    }
}
