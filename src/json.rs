//! Loading fonts from the JSON format written by [ufoLib2].
//!
//! ufoLib2's `Font.json_dumps()` writes a whole font as one JSON document that
//! mirrors ufoLib2's objects rather than the on-disk UFO layout: there is no
//! metainfo, there are no file names, and libs, data and images are inlined.
//!
//! Loading happens in two steps. The document is deserialized into private
//! wire types that mirror its shape, and those are converted into norad's
//! types, with the same validation as loading a `.ufo`.
//!
//! This is behind the `unstable-json` feature, and is not stable API: it may
//! change or break in any release, including patch releases.
//!
//! [ufoLib2]: https://github.com/fonttools/ufoLib2

use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt;
use std::io::Error as IoError;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::de::{self, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use thiserror::Error;

use crate::datastore::{DataType, Store};
use crate::error::{
    ColorError, ErrorKind, FontInfoErrorKind, FontInfoLoadError, GlifLoadError,
    GroupsValidationError, NamingError, StoreEntryError,
};
use crate::glyph::builder::OutlineBuilder;
use crate::groups::{deserialize_groups_skipping_empty_members, validate_groups, Groups};
use crate::layer::DEFAULT_LAYER_NAME;
use crate::{
    AffineTransform, Anchor, Codepoints, Color, Font, FontInfo, Glyph, Guideline, Identifier,
    Image, Kerning, Layer, LayerContents, Line, MetaInfo, Name, Plist, PointType,
};

/// The `type` of the object ufoLib2 wraps binary lib data in.
static DATA_WRAPPER_TYPE: &str = "com.github.fonttools.ufoLib2.lib.plist.data";
/// The `type` of the object ufoLib2 wraps lib dates in.
static DATE_WRAPPER_TYPE: &str = "com.github.fonttools.ufoLib2.lib.plist.date";

impl Font {
    /// Load a font from a file in ufoLib2's JSON format.
    ///
    /// Requires the `unstable-json` feature. This is not stable API, and may
    /// change or break in any release, including patch releases.
    ///
    /// See [`Font::from_ufolib2_json`] for details.
    pub fn load_ufolib2_json(path: impl AsRef<Path>) -> Result<Font, JsonLoadError> {
        let bytes = std::fs::read(path.as_ref())?;
        Self::from_ufolib2_json(&bytes)
    }

    /// Load a font from bytes in ufoLib2's JSON format.
    ///
    /// Requires the `unstable-json` feature. This is not stable API, and may
    /// change or break in any release, including patch releases.
    ///
    /// This reads the output of ufoLib2's `Font.json_dumps()`, and runs the
    /// same validation as [`Font::load`]. Object libs are moved out of
    /// `public.objectLibs` onto their objects as [`Font::load`] does: always
    /// for glyphs, and for font guidelines only if the font has info, just as
    /// a `.ufo` without a `fontinfo.plist` keeps them in the font lib.
    ///
    /// The result is usually the same as loading the `.ufo` that ufoLib2
    /// would write for the same font, but the JSON carries less information
    /// than a `.ufo`, and ufoLib2's reader differs from norad's:
    ///
    /// - There is no `metainfo.plist`, so [`Font::meta`] is the default.
    /// - There are no file names. Layer directory and glyph file names are
    ///   derived from the names as for layers and glyphs created in memory,
    ///   so they only match the `.ufo` where norad's algorithm agrees with
    ///   the tool that wrote it.
    /// - `tempLib` on the font, layers and glyphs is discarded.
    /// - Plist dates written by older ufoLib2 versions arrive as strings:
    ///   these wrote them as bare ISO 8601 strings (and failed to write them
    ///   at all without orjson).
    /// - Lib integers too large for an `i64` or `u64` become reals.
    /// - Notes are trimmed like the `.glif` parser trims them, but features
    ///   are whatever ufoLib2 read: unlike norad, it normalizes line endings.
    /// - An image without a file name is no image, as ufoLib2 considers it.
    /// - ufoLib2 does not move the RoboFab data of UFO 1 fonts out of the lib
    ///   the way [`Font::load`] does, so a dump of such a font keeps it there.
    /// - Data and images are decoded and validated up front, instead of on
    ///   first access, so an image that isn't a PNG fails the load.
    ///
    /// Loading is also stricter than ufoLib2's: unknown keys (including in
    /// layers), `null` lib values and invalid base64 are errors.
    pub fn from_ufolib2_json(bytes: &[u8]) -> Result<Font, JsonLoadError> {
        let font: WireFont = serde_json::from_slice(bytes)?;
        font.into_font()
    }
}

/// An error that occurs while loading a font from ufoLib2's JSON format.
///
/// Requires the `unstable-json` feature. This is not stable API, and may
/// change or break in any release, including patch releases.
///
/// Anything the deserializer checks is reported as a [`Json`] error, with a
/// line and column: unknown or missing keys, values of the wrong type or out
/// of range, malformed lib data, invalid code points, invalid names in groups
/// and kerning, and everything in the font info, which goes through the same
/// deserializer as `fontinfo.plist`.
///
/// Values that are well-formed JSON but invalid in a UFO are checked
/// afterwards and get their own variants: layer names and colors, glyph
/// names, everything inside glyphs (reported as a [`Glyph`] error with the
/// same [`GlifLoadError`] a `.glif` file would give), store entries, and the
/// font info validation that `fontinfo.plist` also gets.
///
/// [`Json`]: JsonLoadError::Json
/// [`Glyph`]: JsonLoadError::Glyph
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum JsonLoadError {
    /// An [`std::io::Error`].
    #[error("failed to read file: {0}")]
    Io(#[from] IoError),
    /// The document is not valid JSON, or does not have the structure of a
    /// ufoLib2 font.
    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// The font info contains invalid data.
    #[error("font info contains invalid data: {0}")]
    InvalidFontInfo(FontInfoErrorKind),
    /// The font lib's `public.objectLibs` value is not a dictionary.
    #[error("the font lib's 'public.objectLibs' value must be a dictionary")]
    FontObjectLibsMustBeDictionary,
    /// The font lib's `public.objectLibs` entry for a font guideline is not a dictionary.
    #[error(
        "the font lib's 'public.objectLibs' entry for the guideline with identifier '{0}' must be a dictionary"
    )]
    GuidelineLibMustBeDictionary(String),
    /// The (kerning) groups fail validation.
    #[error("failed to load (kerning) groups")]
    InvalidGroups(#[source] GroupsValidationError),
    /// No layer is the default layer.
    #[error("no layer is named 'public.default' or marked as the default")]
    MissingDefaultLayer,
    /// More than one layer is the default layer.
    #[error("layers '{first}' and '{second}' are both marked as the default")]
    MultipleDefaultLayers {
        /// The name of the first default layer.
        first: String,
        /// The name of the second default layer.
        second: String,
    },
    /// The layer named `public.default` is marked as not being the default.
    #[error("the layer named 'public.default' must be the default layer")]
    PublicDefaultNotDefault,
    /// A layer name is invalid or duplicated.
    #[error("invalid layer name")]
    LayerName(#[source] NamingError),
    /// A layer's color is invalid.
    #[error("invalid color for layer '{layer}'")]
    LayerColor {
        /// The layer name.
        layer: String,
        /// The underlying error.
        source: ColorError,
    },
    /// A glyph appears twice in the same layer.
    #[error("glyph '{glyph}' appears twice in layer '{layer}'")]
    DuplicateGlyph {
        /// The layer name.
        layer: String,
        /// The glyph name.
        glyph: String,
    },
    /// A glyph's `name` does not match the key it is stored under.
    #[error("glyph '{key}' in layer '{layer}' has mismatched name '{name}'")]
    GlyphNameMismatch {
        /// The layer name.
        layer: String,
        /// The key the glyph is stored under.
        key: String,
        /// The name in the glyph itself.
        name: String,
    },
    /// A glyph contains invalid data.
    #[error("failed to load glyph '{glyph}' in layer '{layer}'")]
    Glyph {
        /// The layer name.
        layer: String,
        /// The glyph name.
        glyph: String,
        /// The underlying error.
        source: GlifLoadError,
    },
    /// An entry in the data store is not valid base64.
    #[error("data store entry '{path}' is not valid base64: {reason}")]
    DataStoreBase64 {
        /// The path of the entry.
        path: PathBuf,
        /// A description of the problem.
        reason: String,
    },
    /// An entry in the data store is invalid.
    #[error("failed to load data store")]
    DataStore(#[source] StoreEntryError),
    /// An entry in the images store is not valid base64.
    #[error("images store entry '{path}' is not valid base64: {reason}")]
    ImagesStoreBase64 {
        /// The path of the entry.
        path: PathBuf,
        /// A description of the problem.
        reason: String,
    },
    /// An entry in the images store is invalid.
    #[error("failed to load images store")]
    ImagesStore(#[source] StoreEntryError),
}

/// The root object of the document.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireFont<'a> {
    #[serde(borrow)]
    layers: Option<Vec<WireLayer<'a>>>,
    info: Option<JsonFontInfo>,
    #[serde(default)]
    features: String,
    #[serde(default, deserialize_with = "deserialize_groups_skipping_empty_members")]
    groups: Groups,
    #[serde(default)]
    kerning: Kerning,
    #[serde(default)]
    lib: JsonLib,
    #[serde(default, borrow, deserialize_with = "ordered_entries")]
    data: Vec<(JsonStr<'a>, JsonStr<'a>)>,
    #[serde(default, borrow, deserialize_with = "ordered_entries")]
    images: Vec<(JsonStr<'a>, JsonStr<'a>)>,
    #[serde(default, rename = "tempLib")]
    _temp_lib: IgnoredAny,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLayer<'a> {
    #[serde(borrow)]
    name: Option<JsonStr<'a>>,
    default: Option<bool>,
    #[serde(default, borrow, deserialize_with = "ordered_entries")]
    glyphs: Vec<(JsonStr<'a>, WireGlyph<'a>)>,
    #[serde(default)]
    lib: JsonLib,
    #[serde(default, rename = "tempLib")]
    _temp_lib: IgnoredAny,
    #[serde(borrow)]
    color: Option<JsonStr<'a>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireGlyph<'a> {
    #[serde(borrow)]
    name: Option<JsonStr<'a>>,
    #[serde(default)]
    width: f64,
    #[serde(default)]
    height: f64,
    #[serde(default, deserialize_with = "deserialize_codepoints")]
    unicodes: Codepoints,
    #[serde(borrow)]
    image: Option<WireImage<'a>>,
    #[serde(default)]
    lib: JsonLib,
    note: Option<String>,
    #[serde(default, borrow)]
    anchors: Vec<WireAnchor<'a>>,
    #[serde(default, borrow)]
    components: Vec<WireComponent<'a>>,
    #[serde(default, borrow)]
    contours: Vec<WireContour<'a>>,
    #[serde(default, borrow)]
    guidelines: Vec<WireGuideline<'a>>,
    #[serde(default, rename = "tempLib")]
    _temp_lib: IgnoredAny,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireAnchor<'a> {
    x: f64,
    y: f64,
    #[serde(borrow)]
    name: Option<JsonStr<'a>>,
    #[serde(borrow)]
    color: Option<JsonStr<'a>>,
    #[serde(borrow)]
    identifier: Option<JsonStr<'a>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireGuideline<'a> {
    x: Option<f64>,
    y: Option<f64>,
    angle: Option<f64>,
    #[serde(borrow)]
    name: Option<JsonStr<'a>>,
    #[serde(borrow)]
    color: Option<JsonStr<'a>>,
    #[serde(borrow)]
    identifier: Option<JsonStr<'a>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct WireComponent<'a> {
    #[serde(borrow)]
    base_glyph: JsonStr<'a>,
    transformation: Option<WireTransform>,
    #[serde(borrow)]
    identifier: Option<JsonStr<'a>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireContour<'a> {
    #[serde(default, borrow)]
    points: Vec<WirePoint<'a>>,
    #[serde(borrow)]
    identifier: Option<JsonStr<'a>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WirePoint<'a> {
    x: f64,
    y: f64,
    #[serde(rename = "type")]
    typ: Option<WirePointType>,
    #[serde(default)]
    smooth: bool,
    #[serde(borrow)]
    name: Option<JsonStr<'a>>,
    #[serde(borrow)]
    identifier: Option<JsonStr<'a>>,
}

/// A point type. ufoLib2 omits the type of off-curve points, but accepts `"offcurve"`.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum WirePointType {
    Move,
    Line,
    OffCurve,
    Curve,
    QCurve,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct WireImage<'a> {
    #[serde(borrow)]
    file_name: Option<JsonStr<'a>>,
    transformation: Option<WireTransform>,
    #[serde(borrow)]
    color: Option<JsonStr<'a>>,
}

/// An affine transformation, as `[xx, xy, yx, yy, dx, dy]`.
type WireTransform = [f64; 6];

/// A string that borrows from the input unless it contains escapes.
struct JsonStr<'a>(Cow<'a, str>);

/// Font info, deserialized with norad's [`FontInfo`] impl.
///
/// That impl ignores unknown keys, which are reported here instead, and it
/// requires a few keys that ufoLib2 leaves out when they have their default
/// value, which are filled in first.
struct JsonFontInfo(FontInfo);

/// A lib dictionary, with ufoLib2's wrapped binary data turned back into data.
#[derive(Default)]
struct JsonLib(Plist);

/// A single lib value.
struct JsonLibValue(plist::Value);

impl WireFont<'_> {
    fn into_font(self) -> Result<Font, JsonLoadError> {
        let mut lib = self.lib.0;
        // Like the .ufo loader, which only does this when there is a
        // fontinfo.plist; ufoLib2 leaves out both empty info and that file.
        let font_info = match self.info {
            Some(JsonFontInfo(mut font_info)) => {
                font_info.validate_and_load_object_libs(&mut lib).map_err(font_info_error)?;
                font_info
            }
            None => FontInfo::default(),
        };
        validate_groups(&self.groups).map_err(JsonLoadError::InvalidGroups)?;

        let layers = match self.layers {
            Some(layers) => load_layers(layers)?,
            None => LayerContents::default(),
        };
        let data = load_store(
            self.data,
            |path, reason| JsonLoadError::DataStoreBase64 { path, reason },
            JsonLoadError::DataStore,
        )?;
        let images = load_store(
            self.images,
            |path, reason| JsonLoadError::ImagesStoreBase64 { path, reason },
            JsonLoadError::ImagesStore,
        )?;

        Ok(Font {
            meta: MetaInfo::default(),
            font_info,
            layers,
            lib,
            groups: self.groups,
            kerning: self.kerning,
            features: self.features,
            data,
            images,
        })
    }
}

/// Convert an error from the font info code shared with the .ufo loader.
///
/// Its messages name the .ufo files, which would be confusing here.
fn font_info_error(error: FontInfoLoadError) -> JsonLoadError {
    match error {
        FontInfoLoadError::InvalidData(kind) | FontInfoLoadError::FontInfoUpconversion(kind) => {
            JsonLoadError::InvalidFontInfo(kind)
        }
        FontInfoLoadError::PublicObjectLibsMustBeDictionary => {
            JsonLoadError::FontObjectLibsMustBeDictionary
        }
        FontInfoLoadError::GlobalGuidelineLibMustBeDictionary(id) => {
            JsonLoadError::GuidelineLibMustBeDictionary(id)
        }
        // Only reading fontinfo.plist produces these.
        FontInfoLoadError::Io(e) => JsonLoadError::Io(e),
        FontInfoLoadError::ParsePlist(e) => JsonLoadError::Json(de::Error::custom(e)),
    }
}

/// Build the layer set, with the default layer first.
fn load_layers(mut wire_layers: Vec<WireLayer>) -> Result<LayerContents, JsonLoadError> {
    // Check for duplicates first: a second layer without a name is also
    // named public.default, and should be reported as a duplicate.
    let mut names = HashSet::new();
    for layer in &wire_layers {
        if !names.insert(layer.name()) {
            return Err(JsonLoadError::LayerName(NamingError::Duplicate(layer.name().into())));
        }
    }

    let mut default_idx: Option<usize> = None;
    for (idx, layer) in wire_layers.iter().enumerate() {
        let is_default = match (layer.name() == DEFAULT_LAYER_NAME, layer.default) {
            (true, Some(false)) => return Err(JsonLoadError::PublicDefaultNotDefault),
            (true, _) | (false, Some(true)) => true,
            (false, _) => false,
        };
        if is_default {
            if let Some(first) = default_idx {
                return Err(JsonLoadError::MultipleDefaultLayers {
                    first: wire_layers[first].name().to_string(),
                    second: layer.name().to_string(),
                });
            }
            default_idx = Some(idx);
        }
    }
    let default_idx = default_idx.ok_or(JsonLoadError::MissingDefaultLayer)?;

    // The .ufo loader rotates the layer order to bring the default layer to
    // the front, rather than moving it there; do the same.
    wire_layers.rotate_left(default_idx);
    let mut wire_layers = wire_layers.into_iter();
    let mut layers = LayerContents::default();

    let default = wire_layers.next().expect("default layer was found above");
    if default.name() != DEFAULT_LAYER_NAME {
        layers
            .rename_layer(DEFAULT_LAYER_NAME, default.name(), false)
            .map_err(JsonLoadError::LayerName)?;
    }
    default.load_into(layers.default_layer_mut())?;

    for wire_layer in wire_layers {
        let layer = layers.new_layer(wire_layer.name()).map_err(JsonLoadError::LayerName)?;
        wire_layer.load_into(layer)?;
    }
    Ok(layers)
}

impl WireLayer<'_> {
    fn name(&self) -> &str {
        self.name.as_deref().unwrap_or(DEFAULT_LAYER_NAME)
    }

    /// Fill an empty layer, which already has the right name and path.
    fn load_into(self, layer: &mut Layer) -> Result<(), JsonLoadError> {
        let layer_name = layer.name().clone();
        if let Some(color) = self.color {
            let color = color.parse().map_err(|source| JsonLoadError::LayerColor {
                layer: layer_name.to_string(),
                source,
            })?;
            layer.color = Some(color);
        }
        layer.lib = self.lib.0;

        // Insert in document order, which is the order ufoLib2 writes glyphs in.
        for (key, wire_glyph) in self.glyphs {
            if layer.contains_glyph(&key) {
                return Err(JsonLoadError::DuplicateGlyph {
                    layer: layer_name.to_string(),
                    glyph: key.to_string(),
                });
            }
            layer.insert_glyph(wire_glyph.into_glyph(&layer_name, &key)?);
        }
        Ok(())
    }
}

impl WireGlyph<'_> {
    fn into_glyph(self, layer: &Name, key: &str) -> Result<Glyph, JsonLoadError> {
        if let Some(name) = &self.name {
            if &**name != key {
                return Err(JsonLoadError::GlyphNameMismatch {
                    layer: layer.to_string(),
                    key: key.to_string(),
                    name: name.to_string(),
                });
            }
        }
        self.convert(key).map_err(|source| JsonLoadError::Glyph {
            layer: layer.to_string(),
            glyph: key.to_string(),
            source,
        })
    }

    /// Convert to a [`Glyph`], checking what the .glif parser checks.
    fn convert(self, key: &str) -> Result<Glyph, GlifLoadError> {
        let name = Name::new(key).map_err(|_| ErrorKind::InvalidName)?;
        let mut glyph = Glyph::new_impl(name);
        glyph.width = self.width;
        glyph.height = self.height;
        glyph.codepoints = self.unicodes;
        glyph.note = self.note.and_then(trim_note);
        glyph.lib = self.lib.0;

        let mut identifiers = Identifiers::default();
        for anchor in self.anchors {
            glyph.anchors.push(Anchor::new(
                anchor.x,
                anchor.y,
                parse_name(anchor.name)?,
                parse_color(anchor.color)?,
                identifiers.parse(anchor.identifier)?,
            ));
        }
        for guideline in self.guidelines {
            glyph.guidelines.push(guideline.into_guideline(&mut identifiers)?);
        }

        let mut builder = OutlineBuilder::new();
        let mut has_smooth_off_curve = false;
        for contour in self.contours {
            builder.begin_path(identifiers.parse(contour.identifier)?)?;
            for point in contour.points {
                let typ = point.typ.map_or(PointType::OffCurve, PointType::from);
                // Like the .glif parser, drop `smooth` from off-curve points.
                let smooth = point.smooth && typ != PointType::OffCurve;
                has_smooth_off_curve |= point.smooth && !smooth;
                builder.add_point(
                    (point.x, point.y),
                    typ,
                    smooth,
                    parse_name(point.name)?,
                    identifiers.parse(point.identifier)?,
                )?;
            }
            builder.end_path()?;
        }
        if has_smooth_off_curve {
            log::info!("glyph '{key}' has off-curve point with 'smooth' attribute set");
        }
        for component in self.components {
            if component.base_glyph.is_empty() {
                return Err(ErrorKind::ComponentEmptyBase.into());
            }
            let base = Name::new(&component.base_glyph).map_err(|_| ErrorKind::InvalidName)?;
            builder.add_component(
                base,
                to_affine(component.transformation),
                identifiers.parse(component.identifier)?,
            );
        }
        (glyph.contours, glyph.components) = builder.finish()?;

        if let Some(image) = self.image {
            glyph.image = image.into_image()?;
        }

        glyph.load_object_libs()?;
        Ok(glyph)
    }
}

impl WireGuideline<'_> {
    /// Convert to a [`Guideline`], checking what the .glif parser checks.
    fn into_guideline(self, identifiers: &mut Identifiers) -> Result<Guideline, ErrorKind> {
        if self.angle.is_some_and(|angle| !(0.0..=360.0).contains(&angle)) {
            return Err(ErrorKind::BadAngle);
        }
        let line = match (self.x, self.y, self.angle) {
            (Some(x), None, None) => Line::Vertical(x),
            (None, Some(y), None) => Line::Horizontal(y),
            (Some(x), Some(y), Some(degrees)) => Line::Angle { x, y, degrees },
            _ => return Err(ErrorKind::BadGuideline),
        };
        // Like the .glif parser, treat an empty name as no name.
        let name = parse_name(self.name.filter(|name| !name.is_empty()))?;
        let color = parse_color(self.color)?;
        Ok(Guideline::new(line, name, color, identifiers.parse(self.identifier)?))
    }
}

impl WireImage<'_> {
    fn into_image(self) -> Result<Option<Image>, ErrorKind> {
        // ufoLib2 treats an image without a file name as no image, and does
        // not write one to a .glif; match what the equivalent .ufo would load.
        let Some(file_name) = self.file_name else {
            return Ok(None);
        };
        let color = parse_color(self.color)?;
        Image::new(PathBuf::from(&*file_name), color, to_affine(self.transformation))
            .map(Some)
            .map_err(|_| ErrorKind::BadImage)
    }
}

impl From<WirePointType> for PointType {
    fn from(src: WirePointType) -> PointType {
        match src {
            WirePointType::Move => PointType::Move,
            WirePointType::Line => PointType::Line,
            WirePointType::OffCurve => PointType::OffCurve,
            WirePointType::Curve => PointType::Curve,
            WirePointType::QCurve => PointType::QCurve,
        }
    }
}

/// The identifiers used in a glyph so far, which must be unique.
#[derive(Default)]
struct Identifiers(HashSet<Identifier>);

impl Identifiers {
    fn parse(&mut self, raw: Option<JsonStr>) -> Result<Option<Identifier>, ErrorKind> {
        let Some(raw) = raw else {
            return Ok(None);
        };
        let id = Identifier::new(&raw)?;
        self.insert(id.clone())?;
        Ok(Some(id))
    }

    fn insert(&mut self, id: Identifier) -> Result<(), ErrorKind> {
        if self.0.insert(id) {
            Ok(())
        } else {
            Err(ErrorKind::DuplicateIdentifier)
        }
    }
}

fn parse_name(raw: Option<JsonStr>) -> Result<Option<Name>, ErrorKind> {
    raw.map(|name| Name::new(&name).map_err(|_| ErrorKind::InvalidName)).transpose()
}

fn parse_color(raw: Option<JsonStr>) -> Result<Option<Color>, ErrorKind> {
    raw.map(|color| color.parse().map_err(|_| ErrorKind::BadColor)).transpose()
}

/// Trim a note like the .glif parser does, which makes an empty note no note.
fn trim_note(note: String) -> Option<String> {
    let trimmed = note.trim_matches([' ', '\t', '\n', '\r']);
    if trimmed.is_empty() {
        None
    } else if trimmed.len() == note.len() {
        Some(note)
    } else {
        Some(trimmed.to_owned())
    }
}

fn to_affine(transform: Option<WireTransform>) -> AffineTransform {
    match transform {
        Some([x_scale, xy_scale, yx_scale, y_scale, x_offset, y_offset]) => {
            AffineTransform { x_scale, xy_scale, yx_scale, y_scale, x_offset, y_offset }
        }
        None => AffineTransform::default(),
    }
}

/// Decode base64 entries into a data or images store.
fn load_store<T: DataType>(
    entries: Vec<(JsonStr, JsonStr)>,
    base64_error: fn(PathBuf, String) -> JsonLoadError,
    store_error: fn(StoreEntryError) -> JsonLoadError,
) -> Result<Store<T>, JsonLoadError> {
    let mut store = Store::<T>::default();
    for (path, encoded) in entries {
        let path = PathBuf::from(&*path);
        let data = match BASE64.decode(encoded.as_bytes()) {
            Ok(data) => data,
            Err(e) => return Err(base64_error(path, e.to_string())),
        };
        if let Err(e) = store.insert(path.clone(), data) {
            return Err(store_error(StoreEntryError::new(path, e)));
        }
    }
    Ok(store)
}

impl Deref for JsonStr<'_> {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for JsonStr<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StrVisitor;

        impl<'de> Visitor<'de> for StrVisitor {
            type Value = Cow<'de, str>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string")
            }

            fn visit_borrowed_str<E: de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
                Ok(Cow::Borrowed(v))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(Cow::Owned(v.to_owned()))
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(Cow::Owned(v))
            }
        }

        deserializer.deserialize_str(StrVisitor).map(JsonStr)
    }
}

/// Deserialize a JSON object into its entries, in document order.
fn ordered_entries<'de, D, K, V>(deserializer: D) -> Result<Vec<(K, V)>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de>,
    V: Deserialize<'de>,
{
    struct EntriesVisitor<K, V>(PhantomData<(K, V)>);

    impl<'de, K: Deserialize<'de>, V: Deserialize<'de>> Visitor<'de> for EntriesVisitor<K, V> {
        type Value = Vec<(K, V)>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("an object")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
            while let Some(entry) = map.next_entry()? {
                entries.push(entry);
            }
            Ok(entries)
        }
    }

    deserializer.deserialize_map(EntriesVisitor(PhantomData))
}

/// Deserialize a list of code points, written as decimal integers.
fn deserialize_codepoints<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Codepoints, D::Error> {
    struct CodepointsVisitor;

    impl<'de> Visitor<'de> for CodepointsVisitor {
        type Value = Codepoints;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a list of Unicode code points")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut codepoints = Codepoints::default();
            while let Some(value) = seq.next_element::<u32>()? {
                let chr = char::from_u32(value).ok_or_else(|| {
                    de::Error::invalid_value(
                        de::Unexpected::Unsigned(value.into()),
                        &"a Unicode scalar value",
                    )
                })?;
                codepoints.insert(chr);
            }
            Ok(codepoints)
        }
    }

    deserializer.deserialize_seq(CodepointsVisitor)
}

impl<'de> Deserialize<'de> for JsonFontInfo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Font info is small, so going through a `Value` costs little, and makes
        // it easy to fill in omitted keys.
        let mut info = serde_json::Map::deserialize(deserializer)?;
        fill_omitted_info_defaults(&mut info);

        let mut unknown = None;
        let info: FontInfo = serde_ignored::deserialize(serde_json::Value::Object(info), |path| {
            unknown.get_or_insert_with(|| info_path(&path));
        })
        .map_err(de::Error::custom)?;
        match unknown {
            Some(path) => Err(de::Error::custom(format_args!("unknown field `{path}` in info"))),
            None => Ok(JsonFontInfo(info)),
        }
    }
}

/// Format a path in the font info like `woffMetadataExtensions[0].bogus`.
fn info_path(path: &serde_ignored::Path) -> String {
    use serde_ignored::Path;

    match path {
        Path::Root => String::new(),
        Path::Seq { parent, index } => format!("{}[{index}]", info_path(parent)),
        Path::Map { parent, key } => match info_path(parent) {
            parent if parent.is_empty() => key.clone(),
            parent => format!("{parent}.{key}"),
        },
        Path::Some { parent }
        | Path::NewtypeStruct { parent }
        | Path::NewtypeVariant { parent } => info_path(parent),
    }
}

/// Fill in keys that ufoLib2 omits when they are empty, but that [`FontInfo`] requires.
///
/// The equivalent `fontinfo.plist` written by ufoLib2 contains these keys.
fn fill_omitted_info_defaults(info: &mut serde_json::Map<String, serde_json::Value>) {
    use serde_json::Value;

    fn fill(object: Option<&mut Value>, key: &str, default: fn() -> Value) {
        if let Some(Value::Object(object)) = object {
            object.entry(key).or_insert_with(default);
        }
    }

    if let Some(Value::Array(records)) = info.get_mut("openTypeNameRecords") {
        for record in records {
            fill(Some(record), "string", || Value::String(String::new()));
        }
    }
    fill(info.get_mut("woffMetadataLicense"), "text", || Value::Array(Vec::new()));
    if let Some(Value::Array(extensions)) = info.get_mut("woffMetadataExtensions") {
        for extension in extensions {
            fill(Some(extension), "names", || Value::Array(Vec::new()));
        }
    }
}

impl<'de> Deserialize<'de> for JsonLib {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // The lib itself is always a dictionary, even if it looks like a data wrapper.
        deserializer.deserialize_map(LibVisitor).map(JsonLib)
    }
}

impl<'de> Deserialize<'de> for JsonLibValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(LibValueVisitor).map(JsonLibValue)
    }
}

struct LibVisitor;

impl<'de> Visitor<'de> for LibVisitor {
    type Value = Plist;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a lib dictionary")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut dict = Plist::new();
        while let Some((key, JsonLibValue(value))) = map.next_entry::<String, JsonLibValue>()? {
            dict.insert(key, value);
        }
        Ok(dict)
    }
}

struct LibValueVisitor;

impl<'de> Visitor<'de> for LibValueVisitor {
    type Value = plist::Value;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a lib value (not null)")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(plist::Value::Boolean(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(plist::Value::Integer(v.into()))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(plist::Value::Integer(v.into()))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        Ok(plist::Value::Real(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(plist::Value::String(v.to_owned()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(plist::Value::String(v))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut array = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(JsonLibValue(value)) = seq.next_element()? {
            array.push(value);
        }
        Ok(plist::Value::Array(array))
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        let dict = LibVisitor.visit_map(map)?;
        unwrap(dict).map_err(de::Error::custom)
    }
}

/// Turn ufoLib2's wrappers for binary data and dates back into data and dates.
///
/// A wrapper is an object with exactly the keys `type` and `data`, where
/// `type` is [`DATA_WRAPPER_TYPE`], or `type` and `date`, where `type` is
/// [`DATE_WRAPPER_TYPE`]; anything else is a dictionary.
fn unwrap(dict: Plist) -> Result<plist::Value, String> {
    let wrapper_type = match dict.get("type") {
        Some(plist::Value::String(t)) if dict.len() == 2 => t.as_str(),
        _ => return Ok(plist::Value::Dictionary(dict)),
    };
    if wrapper_type == DATA_WRAPPER_TYPE && dict.contains_key("data") {
        match dict.get("data") {
            Some(plist::Value::String(encoded)) => BASE64
                .decode(encoded)
                .map(plist::Value::Data)
                .map_err(|e| format!("invalid base64 in lib data: {e}")),
            _ => Err("lib data must be a base64 string".into()),
        }
    } else if wrapper_type == DATE_WRAPPER_TYPE && dict.contains_key("date") {
        match dict.get("date") {
            Some(plist::Value::String(date)) => plist::Date::from_xml_format(date)
                .map(plist::Value::Date)
                .map_err(|_| format!("invalid lib date '{date}'")),
            _ => Err("lib date must be a string".into()),
        }
    } else {
        Ok(plist::Value::Dictionary(dict))
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use super::*;
    use crate::Line;

    fn load(json: &str) -> Result<Font, JsonLoadError> {
        Font::from_ufolib2_json(json.as_bytes())
    }

    /// Wrap a glyph object in a font with a single default layer.
    fn font_with_glyph(glyph: &str) -> String {
        format!(r#"{{"layers": [{{"name": "public.default", "glyphs": {{"a": {glyph}}}}}]}}"#)
    }

    fn load_glyph(glyph: &str) -> Result<Glyph, JsonLoadError> {
        load(&font_with_glyph(glyph)).map(|font| font.get_glyph("a").unwrap().clone())
    }

    fn glyph_error(glyph: &str) -> GlifLoadError {
        match load_glyph(glyph) {
            Err(JsonLoadError::Glyph { source, .. }) => source,
            other => panic!("expected a glyph error, found {other:?}"),
        }
    }

    fn assert_json_error(result: Result<Font, JsonLoadError>, needle: &str) {
        match result {
            Err(JsonLoadError::Json(e)) => {
                assert!(e.to_string().contains(needle), "'{e}' does not contain '{needle}'")
            }
            other => panic!("expected a JSON error, found {other:?}"),
        }
    }

    fn data(bytes: &[u8]) -> plist::Value {
        plist::Value::Data(bytes.to_vec())
    }

    #[test]
    fn minimal() {
        let font = load("{}").unwrap();
        assert_eq!(font.layers.len(), 1);
        assert_eq!(font.default_layer().name().as_str(), DEFAULT_LAYER_NAME);
        assert_eq!(font.meta, MetaInfo::default());
        assert!(font.font_info.is_empty());
    }

    #[test]
    fn data_wrapper_at_any_depth() {
        let font = load(
            r#"{"lib": {
                "top": {"type": "com.github.fonttools.ufoLib2.lib.plist.data", "data": "AAE="},
                "nested": [{"deeper": {"data": "Ag==", "type": "com.github.fonttools.ufoLib2.lib.plist.data"}}],
                "empty": {"type": "com.github.fonttools.ufoLib2.lib.plist.data", "data": ""},
                "extra": {"type": "com.github.fonttools.ufoLib2.lib.plist.data", "data": "AAE=", "x": 1},
                "other": {"type": "something.else", "data": "AAE="}
            }}"#,
        )
        .unwrap();
        let lib = &font.lib;
        assert_eq!(lib["top"], data(&[0, 1]));
        let nested = lib["nested"].as_array().unwrap()[0].as_dictionary().unwrap();
        assert_eq!(nested["deeper"], data(&[2]));
        assert_eq!(lib["empty"], data(&[]));
        assert_eq!(lib["extra"].as_dictionary().unwrap().len(), 3);
        assert_eq!(lib["other"].as_dictionary().unwrap().len(), 2);

        // A glyph lib gets the same treatment.
        let glyph = load_glyph(
            r#"{"lib": {"k": {"type": "com.github.fonttools.ufoLib2.lib.plist.data", "data": "AAE="}}}"#,
        )
        .unwrap();
        assert_eq!(glyph.lib["k"], data(&[0, 1]));
    }

    #[test]
    fn date_wrapper_at_any_depth() {
        let date =
            plist::Value::Date(plist::Date::from_xml_format("2020-01-02T03:04:05Z").unwrap());
        let font = load(
            r#"{"lib": {
                "top": {"type": "com.github.fonttools.ufoLib2.lib.plist.date", "date": "2020-01-02T03:04:05Z"},
                "nested": [{"deeper": {"date": "2020-01-02T03:04:05Z", "type": "com.github.fonttools.ufoLib2.lib.plist.date"}}],
                "extra": {"type": "com.github.fonttools.ufoLib2.lib.plist.date", "date": "2020-01-02T03:04:05Z", "x": 1},
                "mixed": {"type": "com.github.fonttools.ufoLib2.lib.plist.date", "data": "AAE="},
                "bare": "2020-01-02T03:04:05Z"
            }}"#,
        )
        .unwrap();
        let lib = &font.lib;
        assert_eq!(lib["top"], date);
        let nested = lib["nested"].as_array().unwrap()[0].as_dictionary().unwrap();
        assert_eq!(nested["deeper"], date);
        assert_eq!(lib["extra"].as_dictionary().unwrap().len(), 3);
        assert_eq!(lib["mixed"].as_dictionary().unwrap().len(), 2);
        assert_eq!(lib["bare"], plist::Value::String("2020-01-02T03:04:05Z".into()));

        let glyph = load_glyph(
            r#"{"lib": {"k": {"type": "com.github.fonttools.ufoLib2.lib.plist.date", "date": "2020-01-02T03:04:05Z"}}}"#,
        )
        .unwrap();
        assert_eq!(glyph.lib["k"], date);
    }

    #[test]
    fn lib_itself_is_never_data() {
        let font = load(
            r#"{"lib": {"type": "com.github.fonttools.ufoLib2.lib.plist.data", "data": "AAE="}}"#,
        )
        .unwrap();
        assert_eq!(font.lib["data"], plist::Value::String("AAE=".into()));
    }

    #[test]
    fn lib_value_types() {
        let font = load(
            r#"{"lib": {"int": 1, "neg": -1, "big": 18446744073709551615, "real": 1.0,
                "exp": 1e3, "str": "s", "bool": true, "arr": [1, "a"], "dict": {"a": 1}}}"#,
        )
        .unwrap();
        let lib = &font.lib;
        assert_eq!(lib["int"], plist::Value::Integer(1.into()));
        assert_eq!(lib["neg"], plist::Value::Integer((-1).into()));
        assert_eq!(lib["big"], plist::Value::Integer(u64::MAX.into()));
        assert_eq!(lib["real"], plist::Value::Real(1.0));
        assert_eq!(lib["exp"], plist::Value::Real(1000.0));
        assert_eq!(lib["str"], plist::Value::String("s".into()));
        assert_eq!(lib["bool"], plist::Value::Boolean(true));
        assert_eq!(lib["arr"].as_array().unwrap().len(), 2);
        assert_eq!(lib["dict"].as_dictionary().unwrap()["a"], plist::Value::Integer(1.into()));
    }

    #[test]
    fn lib_errors() {
        assert_json_error(load(r#"{"lib": {"a": null}}"#), "null");
        assert_json_error(load(r#"{"lib": {"a": [1, null]}}"#), "null");
        assert_json_error(load(r#"{"lib": []}"#), "a lib dictionary");
        // Invalid characters, which Python's decoder silently drops.
        assert_json_error(
            load(
                r#"{"lib": {"a": {"type": "com.github.fonttools.ufoLib2.lib.plist.data", "data": "!!!"}}}"#,
            ),
            "invalid base64",
        );
        // Missing padding.
        assert_json_error(
            load(
                r#"{"lib": {"a": {"type": "com.github.fonttools.ufoLib2.lib.plist.data", "data": "AAE"}}}"#,
            ),
            "invalid base64",
        );
        assert_json_error(
            load(
                r#"{"lib": {"a": {"type": "com.github.fonttools.ufoLib2.lib.plist.data", "data": 1}}}"#,
            ),
            "must be a base64 string",
        );
        assert_json_error(
            load(
                r#"{"lib": {"a": {"type": "com.github.fonttools.ufoLib2.lib.plist.date", "date": "2020-01-02"}}}"#,
            ),
            "invalid lib date",
        );
        assert_json_error(
            load(
                r#"{"lib": {"a": {"type": "com.github.fonttools.ufoLib2.lib.plist.date", "date": 1}}}"#,
            ),
            "lib date must be a string",
        );
    }

    #[test]
    fn glyph_object_libs() {
        let glyph = load_glyph(
            r#"{
                "lib": {"public.objectLibs": {
                    "anchor": {"a": 1}, "contour": {"c": 1}, "point": {"p": 1},
                    "component": {"x": 1}, "guideline": {"g": 1}, "unused": {"u": 1}
                }},
                "anchors": [{"x": 0, "y": 0, "identifier": "anchor"}],
                "components": [{"baseGlyph": "b", "identifier": "component"}],
                "contours": [{"identifier": "contour", "points": [
                    {"x": 0, "y": 0, "type": "line", "identifier": "point"},
                    {"x": 1, "y": 1, "type": "line"}
                ]}],
                "guidelines": [{"x": 0, "identifier": "guideline"}]
            }"#,
        )
        .unwrap();
        assert!(glyph.lib.is_empty(), "public.objectLibs should be consumed");
        let one = |key: &str| {
            let mut lib = Plist::new();
            lib.insert(key.into(), plist::Value::Integer(1.into()));
            lib
        };
        assert_eq!(glyph.anchors[0].lib(), Some(&one("a")));
        assert_eq!(glyph.contours[0].lib(), Some(&one("c")));
        assert_eq!(glyph.contours[0].points[0].lib(), Some(&one("p")));
        assert_eq!(glyph.contours[0].points[1].lib(), None);
        assert_eq!(glyph.components[0].lib(), Some(&one("x")));
        assert_eq!(glyph.guidelines[0].lib(), Some(&one("g")));

        let err = glyph_error(r#"{"lib": {"public.objectLibs": []}}"#);
        assert!(matches!(err, GlifLoadError::PublicObjectLibsMustBeDictionary));
        let err = glyph_error(
            r#"{"lib": {"public.objectLibs": {"a": 1}}, "anchors": [{"x": 0, "y": 0, "identifier": "a"}]}"#,
        );
        assert!(matches!(err, GlifLoadError::ObjectLibMustBeDictionary(id) if id == "a"));
    }

    #[test]
    fn font_object_libs() {
        let font = load(
            r#"{
                "info": {"guidelines": [{"x": 1, "identifier": "g1"}, {"y": 2}]},
                "lib": {"public.objectLibs": {"g1": {"k": "v"}}, "other": 1}
            }"#,
        )
        .unwrap();
        assert!(!font.lib.contains_key("public.objectLibs"));
        assert!(font.lib.contains_key("other"));
        let guidelines = font.guidelines();
        assert_eq!(guidelines[0].lib().unwrap()["k"], plist::Value::String("v".into()));
        assert!(guidelines[1].lib().is_none());

        // Without info, the key stays in the lib, as it does when loading a
        // .ufo without a fontinfo.plist, which is what ufoLib2 writes then.
        let font = load(r#"{"lib": {"public.objectLibs": {"g1": {"k": "v"}}}}"#).unwrap();
        assert!(font.lib.contains_key("public.objectLibs"));
        let font = load(r#"{"lib": {"public.objectLibs": 1}}"#).unwrap();
        assert!(font.lib.contains_key("public.objectLibs"));

        let err = load(r#"{"info": {}, "lib": {"public.objectLibs": 1}}"#).unwrap_err();
        assert!(matches!(err, JsonLoadError::FontObjectLibsMustBeDictionary));
        let err = load(
            r#"{"info": {"guidelines": [{"x": 1, "identifier": "g1"}]}, "lib": {"public.objectLibs": {"g1": 1}}}"#,
        )
        .unwrap_err();
        assert!(matches!(err, JsonLoadError::GuidelineLibMustBeDictionary(id) if id == "g1"));
    }

    #[test]
    fn default_layer() {
        // Named public.default.
        let font =
            load(r#"{"layers": [{"name": "bg"}, {"name": "public.default"}, {"name": "fg"}]}"#)
                .unwrap();
        let names: Vec<_> = font.layers.iter().map(|l| l.name().as_str()).collect();
        // Rotated, like the .ufo loader does.
        assert_eq!(names, ["public.default", "fg", "bg"]);
        let paths: Vec<_> = font.layers.iter().map(|l| l.path().to_str().unwrap()).collect();
        assert_eq!(paths, ["glyphs", "glyphs.fg", "glyphs.bg"]);

        // Marked as default.
        let font =
            load(r#"{"layers": [{"name": "fg", "default": true}, {"name": "bg"}]}"#).unwrap();
        assert_eq!(font.default_layer().name().as_str(), "fg");
        assert_eq!(font.default_layer().path(), Path::new("glyphs"));
        assert_eq!(font.layers.get("bg").unwrap().path(), Path::new("glyphs.bg"));

        // A missing name means public.default.
        let font = load(r#"{"layers": [{"glyphs": {"a": {}}}]}"#).unwrap();
        assert_eq!(font.default_layer().name().as_str(), DEFAULT_LAYER_NAME);
        assert!(font.get_glyph("a").is_some());

        // Explicitly marked, redundantly.
        load(r#"{"layers": [{"name": "public.default", "default": true}]}"#).unwrap();
    }

    #[test]
    fn default_layer_errors() {
        let err = load(r#"{"layers": []}"#).unwrap_err();
        assert!(matches!(err, JsonLoadError::MissingDefaultLayer));
        let err = load(r#"{"layers": [{"name": "fg"}]}"#).unwrap_err();
        assert!(matches!(err, JsonLoadError::MissingDefaultLayer));
        let err = load(r#"{"layers": [{"name": "fg", "default": false}]}"#).unwrap_err();
        assert!(matches!(err, JsonLoadError::MissingDefaultLayer));

        let err =
            load(r#"{"layers": [{"name": "public.default"}, {"name": "fg", "default": true}]}"#)
                .unwrap_err();
        assert!(matches!(
            err,
            JsonLoadError::MultipleDefaultLayers { first, second }
                if first == "public.default" && second == "fg"
        ));
        let err =
            load(r#"{"layers": [{"name": "a", "default": true}, {"name": "b", "default": true}]}"#)
                .unwrap_err();
        assert!(matches!(err, JsonLoadError::MultipleDefaultLayers { .. }));

        let err =
            load(r#"{"layers": [{"name": "public.default", "default": false}]}"#).unwrap_err();
        assert!(matches!(err, JsonLoadError::PublicDefaultNotDefault));
    }

    #[test]
    fn layer_errors() {
        let err = load(r#"{"layers": [{"name": "public.default"}, {"name": "a"}, {"name": "a"}]}"#)
            .unwrap_err();
        assert!(matches!(err, JsonLoadError::LayerName(crate::error::NamingError::Duplicate(_))));
        // A second layer without a name is also public.default.
        let err = load(r#"{"layers": [{"name": "public.default"}, {}]}"#).unwrap_err();
        assert!(matches!(
            err,
            JsonLoadError::LayerName(crate::error::NamingError::Duplicate(name))
                if name == "public.default"
        ));
        let err = load(r#"{"layers": [{"name": "public.default"}, {"name": ""}]}"#).unwrap_err();
        assert!(matches!(err, JsonLoadError::LayerName(crate::error::NamingError::Invalid(_))));
        let err =
            load(r#"{"layers": [{"name": "fg", "default": true, "color": "1,0,0"}]}"#).unwrap_err();
        assert!(matches!(err, JsonLoadError::LayerColor { layer, .. } if layer == "fg"));
        let err = load(r#"{"layers": [{"name": "public.default", "glyphs": {"a": {}, "a": {}}}]}"#)
            .unwrap_err();
        assert!(matches!(err, JsonLoadError::DuplicateGlyph { glyph, .. } if glyph == "a"));
    }

    #[test]
    fn layer_contents() {
        let font = load(
            r#"{"layers": [{
                "name": "public.default",
                "color": "1,0.75,0,0.7",
                "lib": {"k": "v"},
                "glyphs": {"A": {}, "a": {}, "B": {"name": "B"}}
            }]}"#,
        )
        .unwrap();
        let layer = font.default_layer();
        assert_eq!(layer.color, Some(Color::new(1.0, 0.75, 0.0, 0.7).unwrap()));
        assert_eq!(layer.lib["k"], plist::Value::String("v".into()));
        assert_eq!(layer.get_path("A"), Some(Path::new("A_.glif")));
        assert_eq!(layer.get_path("a"), Some(Path::new("a.glif")));
        assert_eq!(layer.get_glyph("B").unwrap().name().as_str(), "B");
    }

    #[test]
    fn point_defaults() {
        let glyph = load_glyph(
            r#"{"contours": [{"points": [
                {"x": 0, "y": 0},
                {"x": 1, "y": 1, "type": "offcurve"},
                {"x": 2, "y": 2, "type": "curve", "smooth": true, "name": "p"},
                {"x": 3, "y": 3, "type": "line"},
                {"x": 4, "y": 4, "type": "qcurve"}
            ]}, {"points": [{"x": 0, "y": 0, "type": "move"}, {"x": 1, "y": 1, "type": "line"}]}]}"#,
        )
        .unwrap();
        let points = &glyph.contours[0].points;
        let types: Vec<_> = points.iter().map(|p| p.typ).collect();
        assert_eq!(
            types,
            [
                PointType::OffCurve,
                PointType::OffCurve,
                PointType::Curve,
                PointType::Line,
                PointType::QCurve
            ]
        );
        assert!(!points[0].smooth);
        assert!(points[2].smooth);
        assert_eq!(points[2].name.as_deref(), Some("p"));
        assert!(points[0].name.is_none() && points[0].identifier().is_none());
        assert!(!glyph.contours[1].is_closed());

        // Like the .glif parser, smooth off-curves are not smooth.
        let glyph = load_glyph(
            r#"{"contours": [{"points": [{"x": 0, "y": 0, "smooth": true}, {"x": 1, "y": 1, "type": "qcurve"}]}]}"#,
        )
        .unwrap();
        assert!(!glyph.contours[0].points[0].smooth);

        // Empty contours are dropped, as by the .glif parser.
        let glyph = load_glyph(r#"{"contours": [{}, {"points": []}]}"#).unwrap();
        assert!(glyph.contours.is_empty());

        assert_json_error(
            load(&font_with_glyph(
                r#"{"contours": [{"points": [{"x": 0, "y": 0, "type": "bogus"}]}]}"#,
            )),
            "unknown variant `bogus`",
        );
        assert_json_error(
            load(&font_with_glyph(r#"{"contours": [{"points": [{"x": 0, "type": "line"}]}]}"#)),
            "missing field `y`",
        );
    }

    #[test]
    fn outline_validation() {
        let err = glyph_error(
            r#"{"contours": [{"points": [{"x": 0, "y": 0, "type": "line"}, {"x": 0, "y": 0, "type": "move"}]}]}"#,
        );
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::UnexpectedMove)));
        let err = glyph_error(
            r#"{"contours": [{"points": [{"x": 0, "y": 0, "type": "move"}, {"x": 0, "y": 0}]}]}"#,
        );
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::TrailingOffCurves)));
    }

    #[test]
    fn transform_defaults() {
        let glyph = load_glyph(
            r#"{
                "components": [{"baseGlyph": "b"}, {"baseGlyph": "c", "transformation": [2, 0.5, 0, 3, 10, -20.5]}],
                "image": {"fileName": "i.png"}
            }"#,
        )
        .unwrap();
        assert_eq!(glyph.components[0].transform, AffineTransform::default());
        assert_eq!(
            glyph.components[1].transform,
            AffineTransform {
                x_scale: 2.0,
                xy_scale: 0.5,
                yx_scale: 0.0,
                y_scale: 3.0,
                x_offset: 10.0,
                y_offset: -20.5,
            }
        );
        let image = glyph.image.unwrap();
        assert_eq!(image.transform, AffineTransform::default());
        assert_eq!(image.file_name(), Path::new("i.png"));

        assert_json_error(
            load(&font_with_glyph(
                r#"{"components": [{"baseGlyph": "b", "transformation": [1, 0, 0, 1, 0]}]}"#,
            )),
            "invalid length 5",
        );
    }

    #[test]
    fn image() {
        // Without a file name, ufoLib2 considers there to be no image.
        let glyph = load_glyph(r#"{"image": {"color": "1,0,0,1"}}"#).unwrap();
        assert!(glyph.image.is_none());

        let glyph =
            load_glyph(r#"{"image": {"fileName": "i.png", "color": "1,0,0,1", "transformation": [1, 0, 0, 1, 5, 5]}}"#)
                .unwrap();
        let image = glyph.image.unwrap();
        assert_eq!(image.color, Some(Color::new(1.0, 0.0, 0.0, 1.0).unwrap()));
        assert_eq!(image.transform.x_offset, 5.0);

        let err = glyph_error(r#"{"image": {"fileName": "dir/i.png"}}"#);
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::BadImage)));
    }

    #[test]
    fn glyph_name() {
        let err = load(&font_with_glyph(r#"{"name": "b"}"#)).unwrap_err();
        assert!(matches!(
            err,
            JsonLoadError::GlyphNameMismatch { layer, key, name }
                if layer == "public.default" && key == "a" && name == "b"
        ));
        let glyph = load_glyph(r#"{"name": "a"}"#).unwrap();
        assert_eq!(glyph.name().as_str(), "a");

        let err =
            load(r#"{"layers": [{"name": "public.default", "glyphs": {"": {}}}]}"#).unwrap_err();
        assert!(matches!(
            err,
            JsonLoadError::Glyph { source: GlifLoadError::Parse(ErrorKind::InvalidName), .. }
        ));
    }

    #[test]
    fn glyph_fields() {
        let glyph = load_glyph(
            r#"{
                "width": 500, "height": 250.5, "unicodes": [97, 65, 97], "note": "n",
                "anchors": [{"x": 1, "y": 2.5, "name": "top", "color": "0,0,0,1"}],
                "guidelines": [{"x": 1, "y": 2, "angle": 90, "name": "g"}, {"y": 3}]
            }"#,
        )
        .unwrap();
        assert_eq!(glyph.width, 500.0);
        assert_eq!(glyph.height, 250.5);
        assert_eq!(glyph.codepoints, Codepoints::new(['a', 'A']));
        assert_eq!(glyph.note.as_deref(), Some("n"));
        let anchor = &glyph.anchors[0];
        assert_eq!((anchor.x, anchor.y), (1.0, 2.5));
        assert_eq!(anchor.name.as_deref(), Some("top"));
        assert_eq!(anchor.color, Some(Color::new(0.0, 0.0, 0.0, 1.0).unwrap()));
        assert_eq!(glyph.guidelines[0].line, Line::Angle { x: 1.0, y: 2.0, degrees: 90.0 });
        assert_eq!(glyph.guidelines[1].line, Line::Horizontal(3.0));

        assert_json_error(
            load(&font_with_glyph(r#"{"unicodes": [55296]}"#)),
            "Unicode scalar value",
        );
        assert_json_error(load(&font_with_glyph(r#"{"unicodes": ["41"]}"#)), "invalid type");
    }

    #[test]
    fn glyph_notes() {
        // Trimmed like the .glif parser trims them.
        let glyph = load_glyph(r#"{"note": "\n  line 1\n  line 2 \t\r\n"}"#).unwrap();
        assert_eq!(glyph.note.as_deref(), Some("line 1\n  line 2"));
        let glyph = load_glyph(r#"{"note": "\u00a0note\u00a0"}"#).unwrap();
        assert_eq!(glyph.note.as_deref(), Some("\u{a0}note\u{a0}"));
        assert_eq!(load_glyph(r#"{"note": ""}"#).unwrap().note, None);
        assert_eq!(load_glyph(r#"{"note": " \n "}"#).unwrap().note, None);
    }

    #[test]
    fn glyph_value_errors() {
        let err = glyph_error(r#"{"anchors": [{"x": 0, "y": 0, "identifier": "é"}]}"#);
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::BadIdentifier)));
        let err = glyph_error(
            r#"{"anchors": [{"x": 0, "y": 0, "identifier": "a"}], "contours": [{"identifier": "a", "points": [{"x": 0, "y": 0, "type": "line"}]}]}"#,
        );
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::DuplicateIdentifier)));
        let err = glyph_error(
            r#"{"anchors": [{"x": 0, "y": 0, "identifier": "a"}], "guidelines": [{"x": 0, "identifier": "a"}]}"#,
        );
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::DuplicateIdentifier)));
        let err = glyph_error(r#"{"anchors": [{"x": 0, "y": 0, "color": "red"}]}"#);
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::BadColor)));
        let err = glyph_error(r#"{"anchors": [{"x": 0, "y": 0, "name": ""}]}"#);
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::InvalidName)));
        let err = glyph_error(r#"{"components": [{"baseGlyph": ""}]}"#);
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::ComponentEmptyBase)));

        // Guidelines get the same errors as anchors, and as in a .glif.
        let err = glyph_error(r#"{"guidelines": [{"x": 0, "identifier": "é"}]}"#);
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::BadIdentifier)));
        let err = glyph_error(r#"{"guidelines": [{"x": 0, "color": "red"}]}"#);
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::BadColor)));
        let err = glyph_error(r#"{"guidelines": [{"x": 1, "y": 2}]}"#);
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::BadGuideline)));
        let err = glyph_error(r#"{"guidelines": [{"x": 1, "angle": 90}]}"#);
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::BadGuideline)));
        let err = glyph_error(r#"{"guidelines": [{"x": 1, "y": 2, "angle": 400}]}"#);
        assert!(matches!(err, GlifLoadError::Parse(ErrorKind::BadAngle)));
        let glyph = load_glyph(r#"{"guidelines": [{"x": 1, "name": ""}]}"#).unwrap();
        assert_eq!(glyph.guidelines[0].name, None);
    }

    #[test]
    fn unknown_keys() {
        assert_json_error(load(r#"{"bogus": 1}"#), "unknown field `bogus`");
        assert_json_error(
            load(r#"{"layers": [{"name": "public.default", "bogus": 1}]}"#),
            "unknown field `bogus`",
        );
        assert_json_error(load(&font_with_glyph(r#"{"bogus": 1}"#)), "unknown field `bogus`");
        assert_json_error(
            load(&font_with_glyph(r#"{"contours": [{"points": [{"x": 0, "y": 0, "bogus": 1}]}]}"#)),
            "unknown field `bogus`",
        );
        assert_json_error(
            load(&font_with_glyph(r#"{"image": {"fileName": "a.png", "bogus": 1}}"#)),
            "unknown field `bogus`",
        );
        assert_json_error(load(r#"{"info": {"bogus": 1}}"#), "unknown field `bogus` in info");
        assert_json_error(
            load(r#"{"info": {"woffMetadataVendor": {"name": "a", "url": "b", "bogus": 1}}}"#),
            "unknown field `woffMetadataVendor.bogus` in info",
        );
        assert_json_error(
            load(r#"{"info": {"woffMetadataExtensions": [{"items": [], "bogus": 1}]}}"#),
            "unknown field `woffMetadataExtensions[0].bogus` in info",
        );
        assert_json_error(
            load(r#"{"info": {"guidelines": [{"x": 1, "bogus": 1}]}}"#),
            "unknown field `bogus`",
        );
    }

    #[test]
    fn temp_lib_is_ignored() {
        let font = load(
            r#"{
                "tempLib": {"a": null, "b": {"type": "com.github.fonttools.ufoLib2.lib.plist.data", "data": "!"}},
                "layers": [{"name": "public.default", "tempLib": {"a": 1}, "glyphs": {"a": {"tempLib": {"a": 1}}}}]
            }"#,
        )
        .unwrap();
        assert!(font.lib.is_empty());
        assert!(font.default_layer().lib.is_empty());
        assert!(font.get_glyph("a").unwrap().lib.is_empty());
    }

    #[test]
    fn stores() {
        let font = load(
            r#"{
                "data": {"a.txt": "SGVsbG8=", "dir/b.bin": ""},
                "images": {"i.png": "iVBORw0KGgo="}
            }"#,
        )
        .unwrap();
        assert_eq!(font.data.get(Path::new("a.txt")).unwrap().unwrap().deref(), b"Hello");
        assert_eq!(font.data.get(Path::new("dir/b.bin")).unwrap().unwrap().deref(), b"");
        assert_eq!(font.images.len(), 1);

        let err = load(r#"{"data": {"a.txt": "SGVsbG8"}}"#).unwrap_err();
        assert!(matches!(
            err,
            JsonLoadError::DataStoreBase64 { path, .. } if path == Path::new("a.txt")
        ));
        let err = load(r#"{"images": {"i.png": "!!!!"}}"#).unwrap_err();
        assert!(matches!(err, JsonLoadError::ImagesStoreBase64 { .. }));
        // Not a PNG.
        let err = load(r#"{"images": {"i.png": "SGVsbG8="}}"#).unwrap_err();
        assert!(matches!(err, JsonLoadError::ImagesStore(_)));
        let err = load(r#"{"images": {"dir/i.png": "iVBORw0KGgo="}}"#).unwrap_err();
        assert!(matches!(err, JsonLoadError::ImagesStore(_)));
        let err = load(r#"{"data": {"../a.txt": ""}}"#).unwrap_err();
        assert!(matches!(err, JsonLoadError::DataStore(_)));
    }

    #[test]
    fn font_fields() {
        let font = load(
            r#"{
                "features": "feature liga {} liga;",
                "groups": {"public.kern1.A": ["A", "", "Aacute"], "other": []},
                "kerning": {"public.kern1.A": {"V": -20, "W": -10.5}}
            }"#,
        )
        .unwrap();
        assert_eq!(font.features, "feature liga {} liga;");
        // Empty members are dropped, as when loading groups.plist.
        assert_eq!(font.groups["public.kern1.A"], ["A", "Aacute"]);
        assert!(font.groups["other"].is_empty());
        assert_eq!(font.kerning["public.kern1.A"]["V"], -20.0);
        assert_eq!(font.kerning["public.kern1.A"]["W"], -10.5);

        let err =
            load(r#"{"groups": {"public.kern1.A": ["A"], "public.kern1.B": ["A"]}}"#).unwrap_err();
        assert!(matches!(err, JsonLoadError::InvalidGroups(_)));
        assert_json_error(load(r#"{"kerning": {"a": {"b": null}}}"#), "invalid type: null");
    }

    #[test]
    fn info() {
        let font = load(
            r#"{"info": {
                "familyName": "F", "unitsPerEm": 1000, "ascender": 750.0, "versionMajor": 1,
                "openTypeOS2WidthClass": 5, "postscriptIsFixedPitch": true,
                "openTypeHeadCreated": "2019/11/07 21:14:44",
                "openTypeGaspRangeRecords": [{"rangeMaxPPEM": 8, "rangeGaspBehavior": [0, 1]}],
                "guidelines": [],
                "woffMetadataExtensions": [{"id": null, "items": [
                    {"names": [{"text": "n"}], "values": [{"text": "v"}]}
                ]}]
            }}"#,
        )
        .unwrap();
        let info = &font.font_info;
        assert_eq!(info.family_name.as_deref(), Some("F"));
        assert_eq!(info.units_per_em.map(|u| u.as_f64()), Some(1000.0));
        assert_eq!(info.ascender, Some(750.0));
        assert_eq!(info.version_major, Some(1));
        assert_eq!(info.guidelines, Some(Vec::new()));
        assert_eq!(info.woff_metadata_extensions.as_ref().unwrap()[0].id, None);

        // Integer fields reject floats, even integral ones.
        assert_json_error(load(r#"{"info": {"versionMajor": 1.0}}"#), "expected i32");
        assert_json_error(load(r#"{"info": {"openTypeOS2WidthClass": 10}}"#), "10");

        // The same validation as fontinfo.plist.
        let err = load(r#"{"info": {"openTypeHeadCreated": "yesterday"}}"#).unwrap_err();
        assert!(matches!(
            err,
            JsonLoadError::InvalidFontInfo(
                crate::error::FontInfoErrorKind::InvalidOpenTypeHeadCreatedDate
            )
        ));
    }

    #[test]
    fn info_omitted_defaults() {
        // ufoLib2 leaves these out when they are empty.
        let font = load(
            r#"{"info": {
                "openTypeNameRecords": [{"nameID": 1, "platformID": 3, "encodingID": 1, "languageID": 1033}],
                "woffMetadataLicense": {"url": "u"},
                "woffMetadataExtensions": [{"id": null, "items": [
                    {"names": [{"text": "n"}], "values": [{"text": "v"}]}
                ]}]
            }}"#,
        )
        .unwrap();
        let info = &font.font_info;
        assert_eq!(info.open_type_name_records.as_ref().unwrap()[0].string, "");
        assert!(info.woff_metadata_license.as_ref().unwrap().text.is_empty());
        assert!(info.woff_metadata_extensions.as_ref().unwrap()[0].names.is_empty());
    }

    #[test]
    fn matches_glif() {
        // The same glyph, as .glif and as JSON.
        let glif = br#"<?xml version="1.0" encoding="UTF-8"?>
            <glyph name="a" format="2">
              <advance width="500"/>
              <unicode hex="0061"/>
              <anchor x="1" y="2" name="top" identifier="an"/>
              <outline>
                <contour identifier="c">
                  <point x="0" y="0" type="line" smooth="yes" identifier="p"/>
                  <point x="10" y="0"/>
                  <point x="10" y="10" type="qcurve"/>
                </contour>
                <component base="b" xOffset="5" identifier="co"/>
              </outline>
              <lib>
                <dict>
                  <key>public.objectLibs</key>
                  <dict>
                    <key>p</key>
                    <dict><key>k</key><integer>1</integer></dict>
                  </dict>
                  <key>x</key>
                  <data>AAE=</data>
                </dict>
              </lib>
            </glyph>"#;
        let from_glif = Glyph::parse_raw(glif).unwrap();
        let from_json = load_glyph(
            r#"{
                "width": 500, "unicodes": [97],
                "lib": {
                    "public.objectLibs": {"p": {"k": 1}},
                    "x": {"type": "com.github.fonttools.ufoLib2.lib.plist.data", "data": "AAE="}
                },
                "anchors": [{"x": 1, "y": 2, "name": "top", "identifier": "an"}],
                "components": [{"baseGlyph": "b", "transformation": [1, 0, 0, 1, 5, 0], "identifier": "co"}],
                "contours": [{"identifier": "c", "points": [
                    {"x": 0, "y": 0, "type": "line", "smooth": true, "identifier": "p"},
                    {"x": 10, "y": 0},
                    {"x": 10, "y": 10, "type": "qcurve"}
                ]}]
            }"#,
        )
        .unwrap();
        assert_eq!(from_glif, from_json);
    }
}
