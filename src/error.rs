//! Errors, errors, errors

use std::io::Error as IoError;
use std::path::PathBuf;

use plist::Error as PlistError;
use quick_xml::Error as XmlError;

/// Errors that occur while working with font objects.
#[derive(Debug)]
pub enum Error {
    IoError(IoError),
    ParseError(XmlError),
    Glif(GlifError),
    PlistError(PlistError),
}

#[derive(Debug)]
pub struct GlifError {
    pub path: Option<PathBuf>,
    pub position: usize,
    pub kind: ErrorKind,
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
            Error::IoError(e) => e.fmt(f),
            Error::ParseError(e) => e.fmt(f),
            Error::Glif(GlifError { path, position, kind }) => {
                write!(f, "Glif error in {:?} index {}: '{}", path, position, kind)
            }
            Error::PlistError(e) => e.fmt(f),
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
