//! Error types.

use std::io::Error as IoError;
use std::path::PathBuf;

use plist::Error as PlistError;
use quick_xml::Error as XmlError;

use crate::GlyphName;

/// Errors that occur while working with font objects.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// An error returned when trying to save an UFO in anything less than the latest version.
    DowngradeUnsupported,
    /// An error returned when trying to save a Glyph that contains a `public.objectLibs`
    /// lib key already (the key is automatically managed by Norad).
    PreexistingPublicObjectLibsKey,
    /// An error returned when there is no default layer in the UFO directory.
    MissingDefaultLayer,
    /// An error returned when an expected layer is missing.
    MissingLayer(String),
    /// An error returned when a layer is duplicated.
    DuplicateLayer(String),
    /// An error returned when there is an invalid color definition.
    InvalidColor(InvalidColorString),
    /// An error returned when there is a duplicate glyph.
    DuplicateGlyph {
        /// The layer name.
        layer: String,
        /// The glyph name.
        glyph: String,
    },
    /// An error returned when there is a missing expected glyph
    MissingGlyph {
        /// The layer name.
        layer: String,
        /// The glyph name.
        glyph: String,
    },
    /// An error returned when there is an input/output problem during processing
    Io(IoError),
    /// A `.glif` file could not be loaded.
    GlifLoad {
        /// The path of the relevant `.glif` file.
        path: PathBuf,
        /// The underlying error.
        inner: GlifLoadError,
    },
    /// An error that occurs when attempting to write a [`Glyph`] to disk.
    ///
    /// [`Glyph`]: crate::Glyph
    GlifWrite(GlifWriteError),
    /// A plist file could not be read.
    PlistLoad {
        /// The path of the relevant file.
        path: PathBuf,
        /// The underlying error.
        error: PlistError,
    },
    /// A plist file could not be written.
    PlistWrite {
        /// The path of the relevant file.
        path: PathBuf,
        /// The underlying error.
        error: PlistError,
    },
    /// An error returned when there is invalid fontinfo.plist data.
    InvalidFontInfo,
    /// An error returned when there is a problem during fontinfo.plist version up-conversion.
    FontInfoUpconversion,
    /// An error returned when there is invalid groups.plist data.
    InvalidGroups(GroupsValidationError),
    /// An error returned when there is a problem during groups.plist version up-conversion.
    GroupsUpconversionFailure(GroupsValidationError),
    /// An error returned when there is a problem parsing plist data into
    /// [`plist::Dictionary`] types.
    ///
    /// The string is the dictionary key.
    ExpectedPlistDictionary(String),
    /// An error returned when there is an unexpected plist string.
    ExpectedPlistString,
    /// An error returned when there is an inappropriate negative sign on a value.
    ExpectedPositiveValue,
    /// An error returned when there is a problem with kurbo contour conversion.
    #[cfg(feature = "kurbo")]
    ConvertContour(ErrorKind),
    /// An error returned when there is a missing mandatory file.
    MissingFile(String),
    /// An error returned when the requested UFO directory path is not present.
    MissingUfoDir(String),
    /// An error returned when there is an invalid entry in an image or data store.
    ///
    /// This error wraps a [`StoreError`] type and provides additional path data.
    InvalidStoreEntry(PathBuf, StoreError),
}

/// An error that occurs while attempting to read a .glif file from disk.
#[derive(Debug)]
pub enum GlifLoadError {
    /// An [`std::io::Error`].
    Io(IoError),
    /// A [`quick_xml::Error`].
    Xml(XmlError),
    /// The .glif file was malformed.
    Parse(ErrorKind),
}

/// An error representing a failure to insert content into a [`crate::datastore::Store`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum StoreError {
    /// Tried to insert a path whose ancestor is in the store already, implying nesting a file under a file.
    DirUnderFile,
    /// The path was empty.
    EmptyPath,
    /// The path was neither plain file nor directory, but e.g. a symlink.
    NotPlainFileOrDir,
    /// The path was absolute; only relative paths are allowed.
    PathIsAbsolute,
    /// The path was not a plain file, but e.g. a directory or symlink.
    NotPlainFile,
    /// The path contained a subdirectory; `images` is a flat directory.
    Subdir,
    /// The image did not have a valid PNG header.
    InvalidImage,
    /// Encountered an IO error while trying to load data
    Io(std::sync::Arc<std::io::Error>),
}

/// An error representing a failure to validate UFO groups.
#[derive(Debug)]
pub enum GroupsValidationError {
    /// An error returned when there is an invalid groups name.
    InvalidName,
    /// An error returned when there are overlapping kerning groups.
    OverlappingKerningGroups {
        /// The glyph name.
        glyph_name: String,
        /// The group name.
        group_name: String,
    },
}

/// An error representing an invalid [`Color`] string.
///
/// [`Color`]: crate::Color
#[derive(Debug)]
pub struct InvalidColorString {
    /// The source string that caused the error.
    source: String,
}

impl InvalidColorString {
    pub(crate) fn new(source: String) -> Self {
        InvalidColorString { source }
    }
}

/// An error representing a failure during .glif file parsing.
#[derive(Debug)]
pub struct GlifError {
    /// The glif file path.
    pub path: Option<PathBuf>,
    /// The buffer position.
    pub position: usize,
    /// The kind of error.
    pub kind: ErrorKind,
}

/// An error when attempting to write a .glif file.
#[derive(Debug)]
pub struct GlifWriteError {
    /// The name of the glif where the error occured.
    pub name: GlyphName,
    /// The actual error.
    pub inner: WriteError,
}

/// The possible inner error types that can occur when attempting to write
/// out a .glif type.
#[derive(Debug)]
pub enum WriteError {
    /// XML serialzation error. Wraps a [XmlError].
    Xml(XmlError),
    /// When writing out the 'lib' section, we use the plist crate to generate
    /// the plist xml, and then strip the preface and closing </plist> tag.
    ///
    /// If for some reason the implementation of that crate changes, we could
    /// be affected, although this is very unlikely.
    InternalLibWriteError,
    /// An error originating in [`std::io`].
    Io(IoError),
    /// Plist serialization error. Wraps a [PlistError].
    Plist(PlistError),
}

/// The reason for a glif parse failure.
#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    /// The glif version is not supported by this library.
    UnsupportedGlifVersion,
    /// An unknown point type.
    UnknownPointType,
    /// The first XML element of a glif file is invalid.
    WrongFirstElement,
    /// Missing a close tag.
    MissingCloseTag,
    /// Has an unexpected tag.
    UnexpectedTag,
    /// Has an invalid hexadecimal value.
    BadHexValue,
    /// Has an invalid numeric value.
    BadNumber,
    /// Has an invalid color value.
    BadColor,
    /// Has an invalid anchor definition.
    BadAnchor,
    /// Has an invalid point definition.
    BadPoint,
    /// Has an invalid guideline definition.
    BadGuideline,
    /// Has an invalid component definition.
    BadComponent,
    /// Has an invalid image definition.
    BadImage,
    /// Has an invalid identifier.
    BadIdentifier,
    /// Has an invalid lib.
    BadLib,
    /// Has an unexected duplicate value.
    UnexpectedDuplicate,
    /// Has an unexpected move definition.
    UnexpectedMove,
    /// Has an unexpected smooth definition.
    UnexpectedSmooth,
    /// Has an unexpected element definition.
    UnexpectedElement,
    /// Has an unexpected attribute definition.
    UnexpectedAttribute,
    /// Has an unexpected end of file definition.
    UnexpectedEof,
    /// Has an unexpected point following an off curve point definition.
    UnexpectedPointAfterOffCurve,
    /// Has too many off curve points in sequence.
    TooManyOffCurves,
    /// The contour pen path was not started
    PenPathNotStarted,
    /// Has trailing off curve points defined.
    TrailingOffCurves,
    /// Has duplicate identifiers.
    DuplicateIdentifier,
    /// Has unexepected drawing data.
    UnexpectedDrawing,
    /// Has incomplete drawing data.
    UnfinishedDrawing,
    /// Has an unexpected point field.
    UnexpectedPointField,
    /// Has an unexpected component field.
    UnexpectedComponentField,
    /// Has an unexpected anchor field.
    UnexpectedAnchorField,
    /// Has an unexpected guideline field.
    UnexpectedGuidelineField,
    /// Has an unexpected image field.
    UnexpectedImageField,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::DowngradeUnsupported => {
                write!(f, "Downgrading below UFO v3 is not currently supported.")
            }
            Error::PreexistingPublicObjectLibsKey => write!(
                f,
                "The `public.objectLibs` lib key is managed by Norad and must not be set manually."
            ),
            Error::MissingDefaultLayer => write!(f, "Missing default ('glyphs') layer."),
            Error::DuplicateLayer(name) => write!(f, "Layer name '{}' already exists.", name),
            Error::MissingLayer(name) => write!(f, "Layer name '{}' does not exist.", name),
            Error::DuplicateGlyph { layer, glyph } => {
                write!(f, "Glyph named '{}' already exists in layer '{}'", glyph, layer)
            }
            Error::MissingGlyph { layer, glyph } => {
                write!(f, "Glyph '{}' missing from layer '{}'", glyph, layer)
            }
            Error::Io(e) => e.fmt(f),
            Error::InvalidColor(e) => e.fmt(f),
            Error::GlifLoad { path, inner } => {
                write!(f, "Error reading glif '{}': '{}'", path.display(), inner)
            }
            Error::GlifWrite(GlifWriteError { name, inner }) => {
                write!(f, "Failed to save glyph {}, error: '{}'", name, inner)
            }
            Error::PlistLoad { path, error } => {
                write!(f, "Error reading plist at path '{}': {}", path.display(), error)
            }
            Error::PlistWrite { path, error } => {
                write!(f, "Error writing plist to path '{}': {}", path.display(), error)
            }
            Error::InvalidFontInfo => write!(f, "FontInfo contains invalid data"),
            Error::FontInfoUpconversion => {
                write!(f, "FontInfo contains invalid data after upconversion")
            }
            Error::InvalidGroups(ge) => ge.fmt(f),
            Error::GroupsUpconversionFailure(ge) => {
                write!(f, "Upconverting UFO v1 or v2 kerning data to v3 failed: {}", ge)
            }
            Error::ExpectedPlistDictionary(key) => {
                write!(f, "Expected a Plist dictionary at '{}'", key)
            }
            Error::ExpectedPlistString => write!(f, "Expected a Plist string."),
            Error::ExpectedPositiveValue => {
                write!(f, "PositiveIntegerOrFloat expects a positive value.")
            }
            Error::MissingFile(path) => {
                write!(f, "missing required {} file", path)
            }
            Error::MissingUfoDir(path) => {
                write!(f, "{} directory was not found", path)
            }
            Error::InvalidStoreEntry(path, e) => {
                write!(f, "Store entry '{}' error: {}", path.display(), e)
            }
            #[cfg(feature = "kurbo")]
            Error::ConvertContour(cause) => write!(f, "Failed to convert contour: '{}'", cause),
        }
    }
}

impl std::fmt::Display for GlifLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GlifLoadError::Xml(err) => err.fmt(f),
            GlifLoadError::Io(err) => err.fmt(f),
            GlifLoadError::Parse(err) => err.fmt(f),
        }
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use StoreError::*;

        match self {
            DirUnderFile => write!(f, "The parent of the file is a file itself."),
            NotPlainFileOrDir => {
                write!(f, "Only plain files and directories are allowed, no symlinks.")
            }
            PathIsAbsolute => write!(f, "The path must be relative."),
            EmptyPath => {
                write!(f, "An empty path cannot be used as a key in the store.")
            }
            NotPlainFile => write!(f, "Only plain files are allowed, no symlinks."),
            Subdir => write!(f, "Subdirectories are not allowed in the image store."),
            InvalidImage => write!(f, "An image must be a valid PNG."),
            Io(e) => {
                write!(f, "Encountered an IO error while trying to load content: {}.", e)
            }
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
            ErrorKind::WrongFirstElement => write!(f, "Wrong first XML element in glif file"),
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
            ErrorKind::BadIdentifier => write!(f, "Bad identifier"),
            ErrorKind::BadLib => write!(f, "Bad lib"),
            ErrorKind::UnexpectedDuplicate => write!(f, "Unexpected duplicate"),
            ErrorKind::UnexpectedMove => {
                write!(f, "Unexpected move point, can only occur at start of contour")
            }
            ErrorKind::UnexpectedSmooth => {
                write!(f, "Unexpected smooth attribute on an off-curve point")
            }
            ErrorKind::UnexpectedElement => write!(f, "Unexpected element"),
            ErrorKind::UnexpectedAttribute => write!(f, "Unexpected attribute"),
            ErrorKind::UnexpectedEof => write!(f, "Unexpected EOF"),
            ErrorKind::UnexpectedPointAfterOffCurve => {
                write!(f, "An off-curve point must be followed by a curve or qcurve")
            }
            ErrorKind::TooManyOffCurves => {
                write!(f, "At most two off-curve points can precede a curve")
            }
            ErrorKind::PenPathNotStarted => {
                write!(f, "Must call begin_path() before calling add_point() or end_path()")
            }
            ErrorKind::TrailingOffCurves => {
                write!(f, "Open contours must not have trailing off-curves")
            }
            ErrorKind::DuplicateIdentifier => write!(f, "Duplicate identifier"),
            ErrorKind::UnexpectedDrawing => write!(f, "Unexpected drawing without an outline"),
            ErrorKind::UnfinishedDrawing => write!(f, "Unfinished drawing, you must call end_path"),
            ErrorKind::UnexpectedPointField => write!(f, "Unexpected point field"),
            ErrorKind::UnexpectedComponentField => write!(f, "Unexpected component field "),
            ErrorKind::UnexpectedAnchorField => write!(f, "Unexpected anchor field "),
            ErrorKind::UnexpectedGuidelineField => write!(f, "Unexpected guideline field "),
            ErrorKind::UnexpectedImageField => write!(f, "Unexpected image field "),
        }
    }
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WriteError::Io(err) => err.fmt(f),
            WriteError::Xml(err) => err.fmt(f),
            WriteError::Plist(err) => err.fmt(f),
            WriteError::InternalLibWriteError => {
                write!(f, "Internal error while writing lib data. Please open an issue.")
            }
        }
    }
}

impl std::fmt::Display for GlifWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Failed to write glyph '{}': {}", self.name, self.inner)
    }
}

impl std::error::Error for WriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WriteError::Io(inner) => Some(inner),
            WriteError::Xml(inner) => Some(inner),
            WriteError::Plist(inner) => Some(inner),
            WriteError::InternalLibWriteError => None,
        }
    }
}

impl std::error::Error for GlifWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(inner) => Some(inner),
            Error::PlistLoad { error, .. } => Some(error),
            Error::GlifWrite(inner) => Some(&inner.inner),
            _ => None,
        }
    }
}

impl std::fmt::Display for InvalidColorString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Invalid color string '{}'", self.source)
    }
}

impl std::error::Error for InvalidColorString {}

#[doc(hidden)]
impl From<InvalidColorString> for Error {
    fn from(src: InvalidColorString) -> Error {
        Error::InvalidColor(src)
    }
}

#[doc(hidden)]
impl From<GlifWriteError> for Error {
    fn from(src: GlifWriteError) -> Error {
        Error::GlifWrite(src)
    }
}

#[doc(hidden)]
impl From<IoError> for Error {
    fn from(src: IoError) -> Error {
        Error::Io(src)
    }
}

#[doc(hidden)]
impl From<XmlError> for WriteError {
    fn from(src: XmlError) -> WriteError {
        WriteError::Xml(src)
    }
}

#[doc(hidden)]
impl From<IoError> for WriteError {
    fn from(src: IoError) -> WriteError {
        WriteError::Io(src)
    }
}

#[doc(hidden)]
impl From<IoError> for StoreError {
    fn from(src: IoError) -> StoreError {
        StoreError::Io(std::sync::Arc::new(src))
    }
}

#[doc(hidden)]
impl From<PlistError> for WriteError {
    fn from(src: PlistError) -> WriteError {
        WriteError::Plist(src)
    }
}
