//! Error types.

use std::io::Error as IoError;
use std::path::PathBuf;

use plist::Error as PlistError;
use quick_xml::Error as XmlError;
use thiserror::Error;

use crate::GlyphName;

/// Errors that occur while working with font objects.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// An error returned when trying to save an UFO in anything less than the latest version.
    #[error("downgrading below UFO v3 is not currently supported")]
    DowngradeUnsupported,
    /// An error returned when trying to save a Glyph that contains a `public.objectLibs`
    /// lib key already (the key is automatically managed by Norad).
    #[error("the `public.objectLibs` lib key is managed by Norad and must not be set manually")]
    PreexistingPublicObjectLibsKey,
    /// An error returned when there is no default layer in the UFO directory.
    #[error("missing default ('glyphs') layer")]
    MissingDefaultLayer,
    /// An error returned when an expected layer is missing.
    #[error("layer name '{0}' does not exist")]
    MissingLayer(String),
    /// An error returned when a layer is duplicated.
    #[error("layer name '{0}' already exists")]
    DuplicateLayer(String),
    /// An error returned when there is an invalid color definition.
    #[error(transparent)]
    InvalidColor(InvalidColorString),
    /// An error returned when there is a duplicate glyph.
    #[error("glyph named '{glyph}' already exists in layer '{layer}'")]
    DuplicateGlyph {
        /// The layer name.
        layer: String,
        /// The glyph name.
        glyph: String,
    },
    /// An error returned when there is a missing expected glyph
    #[error("glyph '{glyph}' missing from layer '{layer}'")]
    MissingGlyph {
        /// The layer name.
        layer: String,
        /// The glyph name.
        glyph: String,
    },
    /// An error returned when there is an input problem during processing
    #[error("failed to read file or directory '{path}'")]
    UfoLoad {
        /// The path of the relevant file.
        path: PathBuf,
        /// The underlying error.
        source: IoError,
    },
    /// An error returned when there is an output problem during processing
    #[error("failed to write file or directory '{path}'")]
    UfoWrite {
        /// The path of the relevant file.
        path: PathBuf,
        /// The underlying error.
        source: IoError,
    },
    /// A `.glif` file could not be loaded.
    #[error("failed to read glyph file from '{path}'")]
    GlifLoad {
        /// The path of the relevant `.glif` file.
        path: PathBuf,
        /// The underlying error.
        source: GlifLoadError,
    },
    /// An error that occurs when attempting to write a [`Glyph`] to disk.
    ///
    /// [`Glyph`]: crate::Glyph
    #[error("failed to write out glyph '{}'", .0.name)]
    GlifWrite(#[from] GlifWriteError),
    /// A plist file could not be read.
    #[error("failed to read Plist file from '{path}'")]
    PlistLoad {
        /// The path of the relevant file.
        path: PathBuf,
        /// The underlying error.
        source: PlistError,
    },
    /// A plist file could not be written.
    #[error("failed to write Plist file to '{path}'")]
    PlistWrite {
        /// The path of the relevant file.
        path: PathBuf,
        /// The underlying error.
        source: PlistError,
    },
    /// An error returned when there is invalid fontinfo.plist data.
    #[error("fontInfo contains invalid data")]
    InvalidFontInfo,
    /// An error returned when there is a problem during fontinfo.plist version up-conversion.
    #[error("fontInfo contains invalid data after upconversion")]
    FontInfoUpconversion,
    /// An error returned when there is invalid groups.plist data.
    #[error(transparent)]
    InvalidGroups(#[from] GroupsValidationError),
    /// An error returned when there is a problem during groups.plist version up-conversion.
    #[error("upconverting UFO v1 or v2 kerning data to v3 failed")]
    GroupsUpconversionFailure(GroupsValidationError),
    /// An error returned when there is a problem parsing plist data into
    /// [`plist::Dictionary`] types.
    ///
    /// The string is the dictionary key.
    #[error("expected a Plist dictionary at '{0}'")]
    ExpectedPlistDictionary(String),
    /// An error returned when there is an inappropriate negative sign on a value.
    #[error("positiveIntegerOrFloat expects a positive value")]
    ExpectedPositiveValue,
    /// An error returned when there is a problem with kurbo contour conversion.
    #[cfg(feature = "kurbo")]
    #[error("failed to convert contour: '{0}'")]
    ConvertContour(ErrorKind),
    /// An error returned when there is a missing mandatory file.
    #[error("missing required {0} file")]
    MissingFile(String),
    /// An error returned when the requested UFO directory path is not present.
    #[error("{0} directory was not found")]
    MissingUfoDir(String),
    /// An error returned when there is an invalid entry in an image or data store.
    ///
    /// This error wraps a [`StoreError`] type and provides additional path data.
    #[error("store entry '{0}' is invalid")]
    InvalidStoreEntry(PathBuf, #[source] StoreError),
}

/// An error that occurs while attempting to read a .glif file from disk.
#[derive(Debug, Error)]
pub enum GlifLoadError {
    /// An [`std::io::Error`].
    #[error("failed to read file")]
    Io(#[from] IoError),
    /// A [`quick_xml::Error`].
    #[error("failed to read or parse XML structure")]
    Xml(#[from] XmlError),
    /// The .glif file was malformed.
    #[error("failed to parse glyph data: {0}")]
    Parse(#[from] ErrorKind),
}

/// An error representing a failure to insert content into a [`crate::datastore::Store`].
#[derive(Clone, Debug, Error)]
#[non_exhaustive]
pub enum StoreError {
    /// Tried to insert a path whose ancestor is in the store already, implying nesting a file under a file.
    #[error("the parent of the file is a file itself")]
    DirUnderFile,
    /// The path was empty.
    #[error("an empty path cannot be used as a key in the store")]
    EmptyPath,
    /// The path was neither plain file nor directory, but e.g. a symlink.
    #[error("only plain files and directories are allowed, no symlinks")]
    NotPlainFileOrDir,
    /// The path was absolute; only relative paths are allowed.
    #[error("the path must be relative")]
    PathIsAbsolute,
    /// The path was not a plain file, but e.g. a directory or symlink.
    #[error("only plain files are allowed, no symlinks")]
    NotPlainFile,
    /// The path contained a subdirectory; `images` is a flat directory.
    #[error("subdirectories are not allowed in the image store")]
    Subdir,
    /// The image did not have a valid PNG header.
    #[error("an image must be a valid PNG")]
    InvalidImage,
    /// Encountered an IO error while trying to load data
    #[error("encountered an IO error while trying to load content")]
    Io(#[from] std::sync::Arc<std::io::Error>),
}

/// An error representing a failure to validate UFO groups.
#[derive(Debug, Error)]
pub enum GroupsValidationError {
    /// An error returned when there is an invalid groups name.
    #[error("a kerning group name must have at least one character after the common 'public.kernN.' prefix.")]
    InvalidName,
    /// An error returned when there are overlapping kerning groups.
    #[error("the glyph '{glyph_name}' appears in more than one kerning group. Last found in '{group_name}'")]
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
#[derive(Debug, Error)]
#[error("invalid color string '{string}'")]
pub struct InvalidColorString {
    /// The source string that caused the error.
    string: String,
}

impl InvalidColorString {
    pub(crate) fn new(source: String) -> Self {
        InvalidColorString { string: source }
    }
}

/// An error when attempting to write a .glif file.
#[derive(Debug, Error)]
#[error("failed to write glyph '{name}'")]
pub struct GlifWriteError {
    /// The name of the glif where the error occured.
    pub name: GlyphName,
    /// The actual error.
    pub source: WriteError,
}

/// The possible inner error types that can occur when attempting to write
/// out a .glif type.
#[derive(Debug, Error)]
pub enum WriteError {
    /// When writing out the 'lib' section, we use the plist crate to generate
    /// the plist xml, and then strip the preface and closing </plist> tag.
    ///
    /// If for some reason the implementation of that crate changes, we could
    /// be affected, although this is very unlikely.
    #[error("internal error while writing lib data, please open an issue")]
    InternalLibWriteError,
    /// An error originating in [`std::io`].
    #[error("error writing to disk")]
    Io(#[from] IoError),
    /// Plist serialization error. Wraps a [PlistError].
    #[error("error writing a Plist file to disk")]
    Plist(#[from] PlistError),
    /// XML serialzation error. Wraps a [XmlError].
    #[error("error writing an XML file to disk")]
    Xml(#[from] XmlError),
}

/// The reason for a glif parse failure.
#[derive(Debug, Clone, Copy, Error)]
pub enum ErrorKind {
    /// The glif version is not supported by this library.
    #[error("unsupported glif version")]
    UnsupportedGlifVersion,
    /// An unknown point type.
    #[error("unknown point type")]
    UnknownPointType,
    /// The first XML element of a glif file is invalid.
    #[error("wrong first XML element in glif file")]
    WrongFirstElement,
    /// Missing a close tag.
    #[error("missing close tag")]
    MissingCloseTag,
    /// Has an unexpected tag.
    #[error("unexpected tag")]
    UnexpectedTag,
    /// Has an invalid hexadecimal value.
    #[error("bad hex value")]
    BadHexValue,
    /// Has an invalid numeric value.
    #[error("bad number")]
    BadNumber,
    /// Has an invalid color value.
    #[error("bad color")]
    BadColor,
    /// Has an invalid anchor definition.
    #[error("bad anchor")]
    BadAnchor,
    /// Has an invalid point definition.
    #[error("bad point")]
    BadPoint,
    /// Has an invalid guideline definition.
    #[error("bad guideline")]
    BadGuideline,
    /// Has an invalid component definition.
    #[error("bad component")]
    BadComponent,
    /// Has an invalid image definition.
    #[error("bad image")]
    BadImage,
    /// Has an invalid identifier.
    #[error("bad identifier")]
    BadIdentifier,
    /// Has an invalid lib.
    #[error("bad lib")]
    BadLib,
    /// Has an unexected duplicate value.
    #[error("unexpected duplicate")]
    UnexpectedDuplicate,
    /// Has an unexpected move definition.
    #[error("unexpected move point, can only occur at start of contour")]
    UnexpectedMove,
    /// Has an unexpected smooth definition.
    #[error("unexpected smooth attribute on an off-curve point")]
    UnexpectedSmooth,
    /// Has an unexpected element definition.
    #[error("unexpected element")]
    UnexpectedElement,
    /// Has an unexpected attribute definition.
    #[error("unexpected attribute")]
    UnexpectedAttribute,
    /// Has an unexpected end of file definition.
    #[error("unexpected EOF")]
    UnexpectedEof,
    /// Has an unexpected point following an off curve point definition.
    #[error("an off-curve point must be followed by a curve or qcurve")]
    UnexpectedPointAfterOffCurve,
    /// Has too many off curve points in sequence.
    #[error("at most two off-curve points can precede a curve")]
    TooManyOffCurves,
    /// The contour pen path was not started
    #[error("must call begin_path() before calling add_point() or end_path()")]
    PenPathNotStarted,
    /// Has trailing off curve points defined.
    #[error("open contours must not have trailing off-curves")]
    TrailingOffCurves,
    /// Has duplicate identifiers.
    #[error("duplicate identifier")]
    DuplicateIdentifier,
    /// Has unexepected drawing data.
    #[error("unexpected drawing without an outline")]
    UnexpectedDrawing,
    /// Has incomplete drawing data.
    #[error("unfinished drawing, you must call end_path")]
    UnfinishedDrawing,
    /// Has an unexpected point field.
    #[error("unexpected point field")]
    UnexpectedPointField,
    /// Has an unexpected component field.
    #[error("unexpected component field")]
    UnexpectedComponentField,
    /// Has an unexpected anchor field.
    #[error("unexpected anchor field")]
    UnexpectedAnchorField,
    /// Has an unexpected guideline field.
    #[error("unexpected guideline field")]
    UnexpectedGuidelineField,
    /// Has an unexpected image field.
    #[error("unexpected image field")]
    UnexpectedImageField,
}

#[doc(hidden)]
impl From<IoError> for StoreError {
    fn from(src: IoError) -> StoreError {
        StoreError::Io(std::sync::Arc::new(src))
    }
}
