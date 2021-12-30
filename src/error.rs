//! Error types.

use std::io::Error as IoError;
use std::path::PathBuf;

use plist::Error as PlistError;
use quick_xml::Error as XmlError;
use thiserror::Error;

use crate::write::CustomSerializationError;

/// Errors that occur while working with font objects.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
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
    #[error("failed to load font")]
    UfoLoad(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
    /// An error returned when there is an output problem during processing
    #[error("failed to write font")]
    UfoWrite(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
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
    #[error("failed to parse glyph data")]
    Parse(#[from] ErrorKind),
    /// ...
    #[error("the glyph lib's 'public.objectLibs' value must be a dictionary")]
    PublicObjectLibsMustBeDictionary,
    /// ...
    #[error("the glyph lib's 'public.objectLibs' entry for the object with identifier '{0}' must be a dictionary")]
    ObjectLibMustBeDictionary(String),
}

/// An error that occurs while attempting to read a UFO package from disk.
#[derive(Debug, Error)]
pub(crate) enum UfoLoadError {
    #[error("cannot find a font package")]
    MissingUfoDir,
    #[error("cannot find the metainfo.plist file")]
    MissingMetaInfoFile,
    #[error("failed to parse metainfo.plist file")]
    ParsingMetaInfoFile(#[source] PlistError),
    #[error("failed to parse lib.plist file")]
    ParsingLibFile(#[source] PlistError),
    #[error("the lib.plist file must contain a dictionary (<dict>...</dict>)")]
    LibFileMustBeDictionary,
    #[error("failed to load font info data")]
    LoadingFontInfo(#[source] FontInfoError),
    #[error("failed to upgrade old lib.plist to current fontinfo.plist data: {0}")]
    FontInfoV1Upconversion(FontInfoErrorKind),
    #[error("failed to parse groups.plist file")]
    ParsingGroupsFile(#[source] PlistError),
    #[error("failed to load (kerning) groups")]
    InvalidGroups(#[source] GroupsValidationError),
    #[error("failed to parse kerning.plist file")]
    ParsingKerningFile(#[source] PlistError),
    #[error("failed to read features.fea file")]
    LoadingFeatureFile(#[source] IoError),
    #[error("failed to upconvert groups to the latest supported format")]
    GroupsUpconversionFailure(#[source] GroupsValidationError),
    #[error("failed to load data store")]
    LoadingDataStore(#[source] StoreEntryError),
    #[error("failed to load images store")]
    LoadingImagesStore(#[source] StoreEntryError),
    #[error("failed to load layer set")]
    LoadingLayerSet(#[source] LayerSetLoadError),
}

#[derive(Debug, Error)]
pub(crate) enum LayerSetLoadError {
    /// ...
    #[error("cannot find the layercontents.plist file")]
    MissingLayerContentsFile,
    /// ...
    #[error("failed to parse layercontents.plist file")]
    ParsingLayerContentsFile(#[source] PlistError),
    /// ...
    #[error("failed to load layer '{0}' from '{1}'")]
    LoadingLayer(String, PathBuf, #[source] LayerLoadError),
    /// ...
    #[error("missing the default layer ('glyphs' subdirectory)")]
    MissingDefaultLayer,
}

#[derive(Debug, Error)]
pub(crate) enum LayerLoadError {
    /// ...
    #[error("cannot find the contents.plist file")]
    MissingContentsFile,
    /// ...
    #[error("failed to parse contents.plist file")]
    ParsingContentsFile(#[source] PlistError),
    /// ...
    #[error("failed to parse layerinfo.plist file")]
    ParsingLayerInfoFile(#[source] PlistError),
    /// ...
    #[error("failed to load glyph '{0}' from '{1}'")]
    LoadingGlyph(String, PathBuf, #[source] GlifLoadError),
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub(crate) enum FontInfoError {
    /// ...
    #[error("failed to parse fontinfo.plist file")]
    ParsingFontInfoFile(#[source] PlistError),
    /// ...
    #[error("placeholder for any invalid data: {0}")]
    InvalidData(FontInfoErrorKind),
    /// ...
    #[error("failed to upgrade fontinfo.plist contents to latest UFO version data: {0}")]
    FontInfoUpconversion(FontInfoErrorKind),
    /// ...
    #[error("the lib.plist file's 'public.objectLibs' value must be a dictionary")]
    PublicObjectLibsMustBeDictionary,
    /// ...
    #[error("the lib.plist file's 'public.objectLibs' entry for the global guideline with identifier '{0}' in the fontinfo.plist file must be a dictionary")]
    GlobalGuidelineLibMustBeDictionary(String),
}

/// An error pointing to invalid data in the font's info.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FontInfoErrorKind {
    /// ...
    #[error("unrecognized OS/2 width class '{0}'")]
    UnknownWidthClass(String),
    /// ...
    #[error("unrecognized msCharSet '{0}'")]
    UnknownMsCharSet(i32),
    /// ...
    #[error("unrecognized fontStyle '{0}'")]
    UnknownFontStyle(i32),
    /// ...
    #[error("openTypeHeadCreated must be of format 'YYYY/MM/DD HH:MM:SS'")]
    InvalidOpenTypeHeadCreatedDate,
    /// ...
    #[error("openTypeGaspRangeRecords must be sorted by their rangeMaxPPEM values")]
    UnsortedGaspEntries,
    /// ...
    #[error("guideline identifiers must be unique within fontinfo.plist")]
    DuplicateGuidelineIdentifiers,
    /// ...
    #[error("openTypeOS2Selection must not contain bits 0, 5 or 6")]
    DisallowedSelectionBits,
    /// ...
    #[error("openTypeOS2FamilyClass must be two numbers in the range 0-14 and 0-15, respectively")]
    InvalidOs2FamilyClass,
    /// ...
    #[error("the Postscript field '{0}' must contain at most {1} items but found {2}")]
    InvalidPostscriptListLength(String, u8, usize),
    /// ...
    #[error("a '{0}' element must not be empty")]
    EmptyWoffAttribute(String),
}

/// An error representing a failure with a particular [`crate::datastore::Store`] entry.
#[derive(Debug, Error)]
#[error("store entry '{path}' is invalid")]
pub struct StoreEntryError {
    path: PathBuf,
    source: StoreError,
}

impl StoreEntryError {
    /// Returns a new [`StoreEntryError`].
    pub(crate) fn new(path: PathBuf, source: StoreError) -> Self {
        Self { path, source }
    }
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

#[derive(Debug, Error)]
pub(crate) enum UfoWriteError {
    #[error("failed to remove target directory before overwriting")]
    Cleanup(#[source] IoError),
    #[error("failed to create target data directory '{0}'")]
    CreateDataDir(PathBuf, #[source] IoError),
    #[error("failed to create target font directory")]
    CreateUfoDir(#[source] IoError),
    #[error("failed to create target image directory '{0}'")]
    CreateImageDir(PathBuf, #[source] IoError),
    #[error("downgrading below UFO v3 is not currently supported")]
    Downgrade,
    #[error("font info contains invalid data: {0}")]
    InvalidFontInfo(FontInfoErrorKind),
    #[error("failed to write (kerning) groups")]
    InvalidGroups(#[source] GroupsValidationError),
    #[error("store entry '{0}' is invalid")]
    InvalidStoreEntry(PathBuf, #[source] StoreError),
    #[error("the `public.objectLibs` lib key is managed by norad and must not be set manually")]
    PreexistingPublicObjectLibsKey,
    #[error("failed to write data file")]
    WriteData(PathBuf, #[source] IoError),
    #[error("failed to write feature file")]
    WriteFeatureFile(#[source] IoError),
    #[error("failed to write metainfo.plist file")]
    WriteMetaInfo(#[source] CustomSerializationError),
    #[error("failed to write fontinfo.plist file")]
    WriteFontInfo(#[source] CustomSerializationError),
    #[error("failed to write groups.plist file")]
    WriteGroups(#[source] CustomSerializationError),
    #[error("failed to write image file")]
    WriteImage(PathBuf, #[source] IoError),
    #[error("failed to write kerning.plist file")]
    WriteKerning(#[source] CustomSerializationError),
    #[error("failed to write layer '{0}' to '{1}'")]
    WriteLayer(String, PathBuf, #[source] LayerWriteError),
    #[error("failed to write lib.plist file")]
    WriteLib(#[source] CustomSerializationError),
    #[error("failed to write layercontents.plist file")]
    WriteLayerContents(#[source] CustomSerializationError),
}

/// An error that occurs while attempting to read a UFO layer from disk.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LayerWriteError {
    /// ...
    #[error("cannot create layer directory")]
    CreateDir(#[source] IoError),
    /// ...
    #[error("failed to write contents.plist file")]
    WriteContents(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
    /// ...
    #[error("failed to write glyph '{0}' to '{1}'")]
    WriteGlyph(String, PathBuf, #[source] GlifWriteError),
    /// ...
    #[error("failed to write layerinfo.plist file")]
    WriteLayerInfo(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

/// An error when attempting to write a .glif file.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GlifWriteError {
    /// ...
    #[error("failed to serialize glyph to an internal buffer")]
    Buffer(#[source] IoError),
    /// ...
    #[error("downgrading below glyph format version 2 is unsupported")]
    Downgrade,
    /// When writing out the 'lib' section, we use the plist crate to generate
    /// the plist xml, and then strip the preface and closing </plist> tag.
    ///
    /// If for some reason the implementation of that crate changes, we could
    /// be affected, although this is very unlikely.
    #[error("internal error while writing lib data, please open an issue")]
    InternalLibWriteError,
    /// ...
    #[error("failed to write .glif file")]
    Io(#[source] IoError),
    /// Plist serialization error. Wraps a [PlistError].
    #[error("error serializing glyph lib data internally")]
    Plist(#[source] PlistError),
    /// ...
    #[error("the `public.objectLibs` lib key is managed by norad and must not be set manually")]
    PreexistingPublicObjectLibsKey,
    /// XML serialzation error. Wraps a [XmlError].
    #[error("error serializing glyph to XML")]
    Xml(#[source] XmlError),
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
