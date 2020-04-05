//! Errors, errors, errors

use std::io::Error as IoError;
use std::path::PathBuf;

use plist::Error as PlistError;
use quick_xml::Error as XmlError;

use crate::GlyphName;

/// Errors that occur while working with font objects.
#[derive(Debug)]
pub enum Error {
    /// An error representing our refusal to save a UFO file that was
    /// not originally created by norad.
    NotCreatedHere,
    /// An error returned when trying to save an UFO in anything less than the latest version.
    DowngradeUnsupported,
    IoError(IoError),
    ParseError(XmlError),
    Glif(GlifError),
    GlifWrite(GlifWriteError),
    PlistError(PlistError),
    FontInfoError,
    GroupsError(GroupsValidationError),
    GroupsUpconversionError(GroupsValidationError),
    ExpectedPlistDictionaryError,
}

/// An error representing a failure to validate UFO groups.
#[derive(Debug)]
pub enum GroupsValidationError {
    InvalidName,
    OverlappingKerningGroups { glyph_name: String, group_name: String },
}

/// An error that occurs while parsing a .glif file
#[derive(Debug)]
pub struct GlifError {
    pub path: Option<PathBuf>,
    pub position: usize,
    pub kind: ErrorKind,
}

/// An error when attempting to write a .glif file
#[derive(Debug)]
pub struct GlifWriteError {
    pub name: GlyphName,
    pub inner: XmlError,
}

/// Errors that happen when parsing `glif` files. This is converted into either
/// `Error::Xml` or `Error::Glif` at the parse boundary.
#[derive(Debug)]
pub(crate) enum GlifErrorInternal {
    /// A problem with the xml data.
    Xml(XmlError),
    /// A violation of the ufo spec.
    Spec { kind: ErrorKind, position: usize },
}

/// The reason for a glif parse failure.
#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    UnsupportedGlifVersion,
    UnknownPointType,
    WrongFirstElement,
    MissingCloseTag,
    UnexpectedTag,
    BadHexValue,
    BadNumber,
    BadColor,
    BadAnchor,
    BadPoint,
    BadGuideline,
    BadComponent,
    BadImage,
    UnexpectedDuplicate,
    UnexpectedElement,
    UnexpectedEof,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::NotCreatedHere => {
                write!(f, "To prevent data loss, norad will not save files created elsewhere.")
            }
            Error::DowngradeUnsupported => {
                write!(f, "Downgrading below UFO v3 is not currently supported.")
            }
            Error::IoError(e) => e.fmt(f),
            Error::ParseError(e) => e.fmt(f),
            Error::Glif(GlifError { path, position, kind }) => {
                write!(f, "Glif error in {:?} index {}: '{}", path, position, kind)
            }
            Error::GlifWrite(GlifWriteError { name, inner }) => {
                write!(f, "Failed to save glyph {}, error: '{}'", name, inner)
            }
            Error::PlistError(e) => e.fmt(f),
            Error::FontInfoError => write!(f, "FontInfo contains invalid data"),
            Error::GroupsError(ge) => ge.fmt(f),
            Error::GroupsUpconversionError(ge) => write!(f, "Upconverting UFO v1 or v2 kerning data to v3 failed: {}", ge),
            Error::ExpectedPlistDictionaryError => write!(f, "The files groups.plist, kerning.plist and lib.plist must contain plist dictionaries."),
        }
    }
}

impl std::fmt::Display for GroupsValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GroupsValidationError::InvalidName => write!(f, "A kerning group name must have at least one character after the common 'public.kernN.' prefix."),
            GroupsValidationError::OverlappingKerningGroups {glyph_name, group_name} => write!(f, "The glyph '{}' appears in more than one kerning group. Last found in '{}'", glyph_name, group_name)
        }
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ErrorKind::UnsupportedGlifVersion => write!(f, "Unsupported glif version"),
            ErrorKind::UnknownPointType => write!(f, "Unknown point type"),
            ErrorKind::WrongFirstElement => write!(f, "Wrong first element"),
            ErrorKind::MissingCloseTag => write!(f, "Missing close tag"),
            ErrorKind::UnexpectedTag => write!(f, "Unexpected tag"),
            ErrorKind::BadHexValue => write!(f, "Bad hex value"),
            ErrorKind::BadNumber => write!(f, "Bad number"),
            ErrorKind::BadColor => write!(f, "Bad color"),
            ErrorKind::BadAnchor => write!(f, "Bad anchor"),
            ErrorKind::BadPoint => write!(f, "Bad point"),
            ErrorKind::BadGuideline => write!(f, "Bad guideline"),
            ErrorKind::BadComponent => write!(f, "Bad component"),
            ErrorKind::BadImage => write!(f, "Bad image"),
            ErrorKind::UnexpectedDuplicate => write!(f, "Unexpected duplicate"),
            ErrorKind::UnexpectedElement => write!(f, "Unexpected element"),
            ErrorKind::UnexpectedEof => write!(f, "Unexpected EOF"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IoError(inner) => Some(inner),
            Error::PlistError(inner) => Some(inner),
            Error::GlifWrite(inner) => Some(&inner.inner),
            _ => None,
        }
    }
}

impl ErrorKind {
    pub(crate) fn to_error(self, position: usize) -> GlifErrorInternal {
        GlifErrorInternal::Spec { kind: self, position }
    }
}

#[doc(hidden)]
impl From<GlifWriteError> for Error {
    fn from(src: GlifWriteError) -> Error {
        Error::GlifWrite(src)
    }
}

#[doc(hidden)]
impl From<(ErrorKind, usize)> for GlifErrorInternal {
    fn from(src: (ErrorKind, usize)) -> GlifErrorInternal {
        GlifErrorInternal::Spec { kind: src.0, position: src.1 }
    }
}

#[doc(hidden)]
impl From<XmlError> for Error {
    fn from(src: XmlError) -> Error {
        Error::ParseError(src)
    }
}

#[doc(hidden)]
impl From<PlistError> for Error {
    fn from(src: PlistError) -> Error {
        Error::PlistError(src)
    }
}

#[doc(hidden)]
impl From<IoError> for Error {
    fn from(src: IoError) -> Error {
        Error::IoError(src)
    }
}

#[doc(hidden)]
impl From<GlifError> for Error {
    fn from(src: GlifError) -> Error {
        Error::Glif(src)
    }
}

#[doc(hidden)]
impl From<XmlError> for GlifErrorInternal {
    fn from(src: XmlError) -> GlifErrorInternal {
        GlifErrorInternal::Xml(src)
    }
}
