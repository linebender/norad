use std::io::Error as IoError;
use std::rc::Rc;

use plist::Error as PlistError;
use quick_xml::Error as XmlError;

#[derive(Debug)]
pub enum Error {
    IoError(IoError),
    ParseError(XmlError),
    ParseGlif(ParseGlifError),
    MissingFile(&'static str),
    PlistError(PlistError),
    MissingGlyph,
    /// A wrapper for stashing errors for later use.
    SavedError(Rc<Error>),
}

impl From<XmlError> for Error {
    fn from(src: XmlError) -> Error {
        Error::ParseError(src)
    }
}

impl From<PlistError> for Error {
    fn from(src: PlistError) -> Error {
        Error::PlistError(src)
    }
}

impl From<IoError> for Error {
    fn from(src: IoError) -> Error {
        Error::IoError(src)
    }
}

#[derive(Debug, Clone)]
pub struct ParseGlifError {
    kind: ErrorKind,
    position: usize,
}

impl ParseGlifError {
    pub fn new(kind: ErrorKind, position: usize) -> Self {
        ParseGlifError { kind, position }
    }
}

#[derive(Debug, Clone)]
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

impl ErrorKind {
    pub(crate) fn to_error(self, position: usize) -> ParseGlifError {
        ParseGlifError { kind: self, position }
    }
}

impl From<ParseGlifError> for Error {
    fn from(src: ParseGlifError) -> Error {
        Error::ParseGlif(src)
    }
}

#[macro_export]
macro_rules! err {
    ($r:expr, $errtype:expr) => {
        ParseGlifError { kind: $errtype, position: $r.buffer_position() }
    };
}
