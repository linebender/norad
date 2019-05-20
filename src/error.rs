use quick_xml::Error as XmlError;

#[derive(Debug)]
pub enum Error {
    ParseError(XmlError),
    ParseGlif(ParseGlifError),
}

impl From<XmlError> for Error {
    fn from(src: XmlError) -> Error {
        Error::ParseError(src)
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
