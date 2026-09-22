//! Read-only, on-demand access to a UFO.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::data_request::LayerFilter;
use crate::error::{FontLoadError, GlifLoadError, LayerLoadError};
use crate::font::{self, Font, MetaInfo};
use crate::font_source::FontSource;
use crate::layer::{self, Layer, DEFAULT_GLYPHS_DIRNAME};
use crate::{DataRequest, Glyph, Name};

/// Read-only, on-demand access to a [UFO].
///
/// Opening a reader parses only the `metainfo.plist`, the
/// `layercontents.plist` and each layer's `contents.plist`: that is, the list
/// of layers and the names of the glyphs in each. Everything else, including
/// the glyphs themselves, is read when asked for.
///
/// This is useful when you want to parse glyphs individually (for instance
/// from several threads, or only some of them) instead of loading a whole
/// [`Font`] up front. It works the same for UFO directories, zipped UFOs (with
/// the `ufoz` feature) and any other [`FontSource`].
///
/// # Examples
///
/// ```no_run
/// use norad::FontReader;
///
/// let reader = FontReader::open("path/to/font.ufo").expect("failed to open");
/// let layer = reader.default_layer().expect("default layer is indexed");
/// for name in layer.glyph_names() {
///     let glyph = layer.load_glyph(name).unwrap().expect("failed to parse glyph");
///     println!("{}: {} contours", glyph.name(), glyph.contours.len());
/// }
/// ```
///
/// [UFO]: https://unifiedfontobject.org/versions/ufo3/
#[doc(hidden)] // this interface is not stable, and we may break it in patch updates
pub struct FontReader {
    source: Box<dyn FontSource + Send>,
    meta: MetaInfo,
    /// In `layercontents.plist` order.
    layers: Vec<LayerIndex>,
}

/// One layer's entry in the index: its name, directory, and `contents.plist`.
#[derive(Debug)]
pub(crate) struct LayerIndex {
    pub(crate) name: Name,
    pub(crate) path: PathBuf,
    pub(crate) contents: BTreeMap<Name, PathBuf>,
}

/// A single layer of a [`FontReader`].
///
/// This provides the names of the layer's glyphs, and parses glyphs on demand.
#[derive(Clone, Copy)]
#[doc(hidden)] // this interface is not stable, and we may break it in patch updates
pub struct LayerReader<'a> {
    font: &'a FontReader,
    index: &'a LayerIndex,
}

impl FontReader {
    /// Returns a [`FontReader`] for the UFO at `path`.
    ///
    /// `path` is a UFO directory or, with the `ufoz` feature, a zip archive.
    ///
    /// This is equivalent to [`FontReader::open_requested`] with
    /// [`DataRequest::all`].
    pub fn open(path: impl AsRef<Path>) -> Result<Self, FontLoadError> {
        Self::open_requested(path, &DataRequest::all())
    }

    /// Returns a [`FontReader`] for the UFO at `path`, indexing only the layers
    /// admitted by `request`.
    ///
    /// For a zip archive, only the entries that `request` may read are
    /// decompressed; see [`FontReader::load`] for what that means for later
    /// requests.
    pub fn open_requested(
        path: impl AsRef<Path>,
        request: &DataRequest,
    ) -> Result<Self, FontLoadError> {
        let path = path.as_ref();
        let metadata = path.metadata().map_err(FontLoadError::AccessUfoDir)?;
        if metadata.is_dir() {
            return Self::from_source_requested(path.to_path_buf(), request);
        }

        #[cfg(feature = "ufoz")]
        if metadata.is_file() {
            let source = crate::zip_source::ZipSource::open(path, request)?;
            return Self::from_source_requested(source, request);
        }

        Err(FontLoadError::UfoNotADir)
    }

    /// Returns a [`FontReader`] over the given [`FontSource`].
    pub fn from_source(source: impl FontSource + Send + 'static) -> Result<Self, FontLoadError> {
        Self::from_source_requested(source, &DataRequest::all())
    }

    /// Returns a [`FontReader`] over the given [`FontSource`], indexing only
    /// the layers admitted by `request`.
    pub fn from_source_requested(
        source: impl FontSource + Send + 'static,
        request: &DataRequest,
    ) -> Result<Self, FontLoadError> {
        let (meta, layers) = read_index(&source, request)?;
        Ok(FontReader { source: Box::new(source), meta, layers })
    }

    /// Returns the font's metainfo, as found in its `metainfo.plist`.
    ///
    /// Unlike [`Font::meta`], the format version is not upconverted.
    pub fn meta(&self) -> &MetaInfo {
        &self.meta
    }

    /// Returns the path of the UFO directory, if the source is one.
    ///
    /// This is `None` for zipped UFOs and other non-directory sources.
    pub fn path(&self) -> Option<&Path> {
        self.source.as_path()
    }

    /// Returns the underlying [`FontSource`], for reading files directly.
    pub fn source(&self) -> &dyn FontSource {
        &*self.source
    }

    /// Returns an iterator over the indexed layers, starting with the default layer.
    ///
    /// The layers are in the same order as in a loaded [`Font`].
    pub fn layers(&self) -> impl Iterator<Item = LayerReader<'_>> {
        let default_idx = self.layers.iter().position(LayerIndex::is_default).unwrap_or(0);
        let (before, after) = self.layers.split_at(default_idx);
        after.iter().chain(before).map(|index| LayerReader { font: self, index })
    }

    /// Returns the default layer.
    ///
    /// This is `None` if the reader was opened with a request that excludes
    /// the default layer.
    pub fn default_layer(&self) -> Option<LayerReader<'_>> {
        self.layers
            .iter()
            .find(|index| index.is_default())
            .map(|index| LayerReader { font: self, index })
    }

    /// Returns the layer with the given name, if it was indexed.
    pub fn layer(&self, name: &str) -> Option<LayerReader<'_>> {
        self.layers
            .iter()
            .find(|index| index.name == name)
            .map(|index| LayerReader { font: self, index })
    }

    /// Returns a [`Font`] with the data selected by `request`.
    ///
    /// This is equivalent to [`Font::load_requested_data`], but reuses the
    /// layer index read when this reader was opened.
    ///
    /// Only the layers indexed when the reader was opened can be loaded: a
    /// layer that `request` admits but the reader's opening request did not is
    /// skipped, and requesting the default layer when it was not indexed fails
    /// with [`FontLoadError::MissingDefaultLayer`]. Likewise, a zip source only
    /// decompressed the files its opening request could read; anything else
    /// (for instance the `data` directory) reads as absent rather than
    /// failing.
    ///
    /// [`FontLoadError::MissingDefaultLayer`]: crate::error::FontLoadError::MissingDefaultLayer
    pub fn load(&self, request: &DataRequest) -> Result<Font, FontLoadError> {
        Font::load_indexed(&*self.source, &self.meta, &self.layers, request)
    }
}

impl fmt::Debug for FontReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontReader")
            .field("path", &self.path())
            .field("format_version", &self.meta.format_version)
            .field("layers", &self.layers().map(|l| l.index.name.clone()).collect::<Vec<_>>())
            .finish()
    }
}

impl LayerIndex {
    fn is_default(&self) -> bool {
        self.path == Path::new(DEFAULT_GLYPHS_DIRNAME)
    }
}

impl<'a> LayerReader<'a> {
    /// Returns the name of the layer.
    pub fn name(&self) -> &'a Name {
        &self.index.name
    }

    /// Returns the layer's directory, relative to the UFO root.
    pub fn path(&self) -> &'a Path {
        &self.index.path
    }

    /// Returns `true` if this is the default layer.
    pub fn is_default(&self) -> bool {
        self.index.is_default()
    }

    /// Returns the number of glyphs in the layer.
    pub fn len(&self) -> usize {
        self.index.contents.len()
    }

    /// Returns `true` if the layer contains no glyphs.
    pub fn is_empty(&self) -> bool {
        self.index.contents.is_empty()
    }

    /// Returns an iterator over the names of the glyphs in the layer, in
    /// sorted order.
    pub fn glyph_names(&self) -> impl Iterator<Item = &'a Name> {
        self.index.contents.keys()
    }

    /// Returns `true` if the layer contains a glyph with the given name.
    pub fn contains_glyph(&self, name: &str) -> bool {
        self.index.contents.contains_key(name)
    }

    /// Returns the path of the glyph's `.glif` file, relative to the layer's
    /// directory.
    pub fn glyph_path(&self, name: &str) -> Option<&'a Path> {
        self.index.contents.get(name).map(PathBuf::as_path)
    }

    /// Parses the glyph with the given name.
    ///
    /// Returns `None` if the layer's `contents.plist` does not list the glyph.
    /// As when loading a [`Font`], the glyph's name is the one listed in
    /// `contents.plist`, whatever its `.glif` file says.
    pub fn load_glyph(&self, name: &str) -> Option<Result<Glyph, GlifLoadError>> {
        let (name, glif_path) = self.index.contents.get_key_value(name)?;
        Some(Layer::load_one_glyph(&*self.font.source, &self.index.path, name, glif_path))
    }

    /// Loads the whole layer, parsing all of its glyphs.
    pub fn load(&self) -> Result<Layer, LayerLoadError> {
        Layer::load_from_contents(
            &*self.font.source,
            &self.index.path,
            self.index.name.clone(),
            self.index.contents.clone(),
        )
    }
}

impl fmt::Debug for LayerReader<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayerReader")
            .field("name", &self.index.name)
            .field("path", &self.index.path)
            .field("len", &self.len())
            .finish()
    }
}

/// Reads the metainfo, and indexes the layers admitted by `request`.
pub(crate) fn read_index(
    source: &dyn FontSource,
    request: &DataRequest,
) -> Result<(MetaInfo, Vec<LayerIndex>), FontLoadError> {
    let meta = font::load_meta(source)?;
    let layers = read_layer_index(source, &meta, &request.layers)?;
    Ok((meta, layers))
}

/// Reads the `contents.plist` of each layer admitted by `filter`, in
/// `layercontents.plist` order.
pub(crate) fn read_layer_index(
    source: &dyn FontSource,
    meta: &MetaInfo,
    filter: &LayerFilter,
) -> Result<Vec<LayerIndex>, FontLoadError> {
    let layers = layer::read_layer_contents(source, meta)?
        .into_iter()
        .filter(|(name, path)| filter.should_load(name, path))
        .map(|(name, path)| match Layer::read_contents(source, &path) {
            Ok(contents) => Ok(LayerIndex { name, path, contents }),
            Err(e) => {
                Err(FontLoadError::Layer { name: name.to_string(), path, source: Box::new(e) })
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if filter.includes_default_layer() && !layers.iter().any(LayerIndex::is_default) {
        return Err(FontLoadError::MissingDefaultLayer);
    }
    Ok(layers)
}
