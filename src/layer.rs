use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(feature = "druid")]
use std::ops::Deref;

#[cfg(feature = "rayon")]
use rayon::prelude::*;

use crate::error::{FontLoadError, LayerLoadError, LayerWriteError, NamingError};
use crate::glyph::GlyphName;
use crate::names::NameList;
use crate::shared_types::Color;
use crate::{util, Glyph, Plist, WriteOptions};

static CONTENTS_FILE: &str = "contents.plist";
static LAYER_INFO_FILE: &str = "layerinfo.plist";

pub(crate) static LAYER_CONTENTS_FILE: &str = "layercontents.plist";
pub(crate) static DEFAULT_LAYER_NAME: &str = "public.default";
pub(crate) static DEFAULT_GLYPHS_DIRNAME: &str = "glyphs";

pub type LayerName = Arc<str>;

/// A collection of [`Layer`] objects.
///
/// A layer set always includes a default layer, and may also include additional
/// layers.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerSet {
    /// A collection of [`Layer`]s.  The first [`Layer`] is the default.
    layers: Vec<Layer>,
}

#[allow(clippy::len_without_is_empty)] // never empty
impl LayerSet {
    /// Returns a [`LayerSet`] from the provided `path`.
    ///
    /// If a `layercontents.plist` file exists, it will be used, otherwise
    /// we will assume the pre-UFOv3 behaviour, and expect a single glyphs dir.
    ///
    /// The `glyph_names` argument allows norad to reuse glyph name strings,
    /// reducing memory use.
    pub(crate) fn load(base_dir: &Path, glyph_names: &NameList) -> Result<LayerSet, FontLoadError> {
        let layer_contents_path = base_dir.join(LAYER_CONTENTS_FILE);
        let to_load: Vec<(LayerName, PathBuf)> = if layer_contents_path.exists() {
            plist::from_file(&layer_contents_path)
                .map_err(|source| FontLoadError::ParsePlist { name: LAYER_CONTENTS_FILE, source })?
        } else {
            vec![(Arc::from(DEFAULT_LAYER_NAME), PathBuf::from(DEFAULT_GLYPHS_DIRNAME))]
        };

        let mut layers: Vec<_> = to_load
            .into_iter()
            .map(|(name, path)| {
                let layer_path = base_dir.join(&path);
                Layer::load_impl(&layer_path, name.clone(), glyph_names).map_err(|source| {
                    FontLoadError::Layer { name: name.to_string(), path: layer_path, source }
                })
            })
            .collect::<Result<_, _>>()?;

        // move the default layer to the front
        let default_idx = layers
            .iter()
            .position(|l| l.path.to_str() == Some(DEFAULT_GLYPHS_DIRNAME))
            .ok_or(FontLoadError::MissingDefaultLayer)?;
        layers.rotate_left(default_idx);

        Ok(LayerSet { layers })
    }

    /// Returns a new [`LayerSet`] from a `layers` collection.
    ///
    /// Will panic if `layers` is empty.
    pub fn new(mut layers: Vec<Layer>) -> Self {
        assert!(!layers.is_empty());
        layers.first_mut().unwrap().path = DEFAULT_GLYPHS_DIRNAME.into();
        LayerSet { layers }
    }

    /// Returns the number of layers in the set.
    ///
    /// This is always non-zero.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Returns a reference to a layer, by name.
    pub fn get(&self, name: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| &*l.name == name)
    }

    /// Returns a mutable reference to a layer, by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| &*l.name == name)
    }

    /// Returns a mutable reference to a layer, by name, or create it if it doesn't exist.
    pub fn get_or_create(&mut self, name: &str) -> &mut Layer {
        if let Some(index) = self.layers.iter().position(|l| &*l.name == name) {
            self.layers.get_mut(index).unwrap()
        } else {
            let layer = Layer::new(name.into(), None);
            self.layers.push(layer);
            self.layers.last_mut().unwrap()
        }
    }

    /// Returns a reference to the default layer.
    pub fn default_layer(&self) -> &Layer {
        debug_assert!(self.layers[0].path() == Path::new(DEFAULT_GLYPHS_DIRNAME));
        &self.layers[0]
    }

    /// Returns a mutable reference to the default layer.
    pub fn default_layer_mut(&mut self) -> &mut Layer {
        debug_assert!(self.layers[0].path() == Path::new(DEFAULT_GLYPHS_DIRNAME));
        &mut self.layers[0]
    }

    /// Returns an iterator over all layers.
    pub fn iter(&self) -> impl Iterator<Item = &Layer> {
        self.layers.iter()
    }

    /// Returns an iterator over the names of all layers.
    pub fn names(&self) -> impl Iterator<Item = &LayerName> {
        self.layers.iter().map(|l| &l.name)
    }

    /// Returns a new layer with the given name.
    pub fn new_layer(&mut self, name: &str) -> Result<(), NamingError> {
        if self.layers.iter().any(|l| &*l.name == name) {
            Err(NamingError::Duplicate(name.into()))
        } else {
            let layer = Layer::new(name.into(), None);
            self.layers.push(layer);
            Ok(())
        }
    }

    /// Remove a layer.
    ///
    /// The default layer cannot be removed.
    pub fn remove(&mut self, name: &str) -> Option<Layer> {
        self.layers
            .iter()
            .skip(1)
            .position(|l| l.name.as_ref() == name)
            .map(|idx| self.layers.remove(idx + 1))
    }

    /// Rename a layer.
    ///
    /// If `overwrite` is true, and a layer with the new name exists, it will
    /// be replaced.
    ///
    /// Returns an error if `overwrite` is false but a layer with the new
    /// name exists, or if no layer with the old name exists.
    pub fn rename_layer(
        &mut self,
        old: &str,
        new: &str,
        overwrite: bool,
    ) -> Result<(), NamingError> {
        if !overwrite && self.get(new).is_some() {
            Err(NamingError::Duplicate(new.into()))
        } else if self.get(old).is_none() {
            Err(NamingError::Missing(old.into()))
        } else {
            if overwrite {
                self.layers.retain(|l| &*l.name != new)
            }
            self.get_mut(old).unwrap().name = new.into();
            Ok(())
        }
    }
}

impl Default for LayerSet {
    fn default() -> Self {
        let layer = Layer::new(DEFAULT_LAYER_NAME.into(), None);
        let layers = vec![layer];
        LayerSet { layers }
    }
}

/// A [UFO layer], corresponding to a 'glyphs' sub-directory.
///
/// Conceptually, a layer is just a collection of glyphs.
///
/// [UFO layer]: http://unifiedfontobject.org/versions/ufo3/glyphs/
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    #[cfg(feature = "druid")]
    pub(crate) glyphs: BTreeMap<GlyphName, Arc<Glyph>>,
    #[cfg(not(feature = "druid"))]
    pub(crate) glyphs: BTreeMap<GlyphName, Glyph>,
    pub(crate) name: LayerName,
    pub(crate) path: PathBuf,
    contents: BTreeMap<GlyphName, PathBuf>,
    /// Color field.
    pub color: Option<Color>,
    /// lib field.
    pub lib: Plist,
}

impl Layer {
    /// Returns a new [`Layer`] with the provided `name` and `path`.
    ///
    /// The `path` argument, if provided, will be the directory within the UFO
    /// that the layer is saved. If it is not provided, it will be derived from
    /// the layer name.
    pub fn new(name: LayerName, path: Option<PathBuf>) -> Self {
        let path = match path {
            Some(path) => path,
            None if &*name == DEFAULT_LAYER_NAME => DEFAULT_GLYPHS_DIRNAME.into(),
            _ => crate::util::default_file_name_for_layer_name(&name).into(),
        };
        Layer {
            glyphs: BTreeMap::new(),
            name,
            path,
            contents: BTreeMap::new(),
            color: None,
            lib: Default::default(),
        }
    }

    /// Returns a new [`Layer`] that is loaded from `path` with the provided `name`.
    ///
    /// Internal callers should use `load_impl` directly, so that glyph names
    /// can be reused between layers.
    ///
    /// You generally shouldn't need this; instead prefer to load all layers
    /// with [`LayerSet::load`] and then get the layer you need from there.
    #[cfg(test)]
    pub(crate) fn load(path: impl AsRef<Path>, name: LayerName) -> Result<Layer, LayerLoadError> {
        let path = path.as_ref();
        let names = NameList::default();
        Layer::load_impl(path, name, &names)
    }

    /// The actual loading logic.
    ///
    /// `names` is a map of glyphnames; we pass it throughout parsing
    /// so that we reuse the same Arc<str> for identical names.
    pub(crate) fn load_impl(
        path: &Path,
        name: LayerName,
        names: &NameList,
    ) -> Result<Layer, LayerLoadError> {
        let contents_path = path.join(CONTENTS_FILE);
        if !contents_path.exists() {
            return Err(LayerLoadError::MissingContentsFile);
        }
        // these keys are never used; a future optimization would be to skip the
        // names and deserialize to a vec; that would not be a one-liner, though.
        let contents: BTreeMap<GlyphName, PathBuf> = plist::from_file(&contents_path)
            .map_err(|source| LayerLoadError::ParsePlist { name: CONTENTS_FILE, source })?;

        #[cfg(feature = "rayon")]
        let iter = contents.par_iter();
        #[cfg(not(feature = "rayon"))]
        let iter = contents.iter();

        let glyphs = iter
            .map(|(name, glyph_path)| {
                let name = names.get(name);
                let glyph_path = path.join(glyph_path);

                Glyph::load_with_names(&glyph_path, names)
                    .map_err(|source| LayerLoadError::Glyph {
                        name: name.to_string(),
                        path: glyph_path,
                        source,
                    })
                    .map(|mut glyph| {
                        glyph.name = name.clone();
                        #[cfg(feature = "druid")]
                        return (name, Arc::new(glyph));
                        #[cfg(not(feature = "druid"))]
                        (name, glyph)
                    })
            })
            .collect::<Result<_, _>>()?;

        let layerinfo_path = path.join(LAYER_INFO_FILE);
        let (color, lib) = if layerinfo_path.exists() {
            Self::parse_layer_info(&layerinfo_path)?
        } else {
            (None, Plist::new())
        };

        // for us to get this far, the path must have a file name
        let path = path.file_name().unwrap().into();

        Ok(Layer { glyphs, name, path, contents, color, lib })
    }

    fn parse_layer_info(path: &Path) -> Result<(Option<Color>, Plist), LayerLoadError> {
        // Pluck apart the data found in the file, as we want to insert it into `Layer`.
        #[derive(Deserialize)]
        struct LayerInfoHelper {
            color: Option<Color>,
            #[serde(default)]
            lib: Plist,
        }
        let layerinfo: LayerInfoHelper = plist::from_file(path)
            .map_err(|source| LayerLoadError::ParsePlist { name: LAYER_INFO_FILE, source })?;
        Ok((layerinfo.color, layerinfo.lib))
    }

    fn layerinfo_to_file_if_needed(
        &self,
        path: &Path,
        options: &WriteOptions,
    ) -> Result<(), LayerWriteError> {
        if self.color.is_none() && self.lib.is_empty() {
            return Ok(());
        }

        let mut dict = plist::dictionary::Dictionary::new();

        if let Some(c) = &self.color {
            dict.insert("color".into(), c.to_rgba_string().into());
        }
        if !self.lib.is_empty() {
            dict.insert("lib".into(), self.lib.clone().into());
        }

        util::recursive_sort_plist_keys(&mut dict);

        crate::write::write_xml_to_file(&path.join(LAYER_INFO_FILE), &dict, options)
            .map_err(LayerWriteError::LayerInfo)
    }

    /// Serialize this layer to the given path with the default
    /// [`WriteOptions`] serialization format configuration.
    ///
    /// The path should not exist.
    #[cfg(test)]
    pub(crate) fn save(&self, path: impl AsRef<Path>) -> Result<(), LayerWriteError> {
        let options = WriteOptions::default();
        self.save_with_options(path.as_ref(), &options)
    }

    /// Serialize this layer to the given `path` with a custom
    /// [`WriteOptions`] serialization format configuration.
    ///
    /// The path should not exist.
    pub(crate) fn save_with_options(
        &self,
        path: &Path,
        opts: &WriteOptions,
    ) -> Result<(), LayerWriteError> {
        fs::create_dir(&path).map_err(LayerWriteError::CreateDir)?;
        crate::write::write_xml_to_file(&path.join(CONTENTS_FILE), &self.contents, opts)
            .map_err(LayerWriteError::Contents)?;

        self.layerinfo_to_file_if_needed(path, opts)?;

        #[cfg(feature = "rayon")]
        let iter = self.contents.par_iter();
        #[cfg(not(feature = "rayon"))]
        let mut iter = self.contents.iter();

        iter.try_for_each(|(name, glyph_path)| {
            let glyph = self.glyphs.get(name).expect("all glyphs in contents must exist.");
            let glyph_path = path.join(glyph_path);
            glyph.save_with_options(&glyph_path, opts).map_err(|source| LayerWriteError::Glyph {
                name: glyph.name.to_string(),
                path: glyph_path,
                source,
            })
        })
    }

    /// Returns the number of [`Glyph`]s in the layer.
    pub fn len(&self) -> usize {
        self.glyphs.len()
    }

    /// Returns `true` if this layer contains no glyphs.
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    /// Returns the name of the layer.
    ///
    /// This can only be mutated through the [`LayerSet`].
    pub fn name(&self) -> &LayerName {
        &self.name
    }

    /// Returns the directory path of this layer.
    ///
    /// This cannot be mutated; it is either provided when the layer
    /// is loaded, or we will create it for you. Maybe this is bad? We can talk
    /// about it, if you like.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns a reference to the glyph with the given name, if it exists.
    pub fn get_glyph<K>(&self, glyph: &K) -> Option<&Glyph>
    where
        GlyphName: Borrow<K>,
        K: Ord + ?Sized,
    {
        #[cfg(feature = "druid")]
        return self.glyphs.get(glyph).map(|g| g.deref());
        #[cfg(not(feature = "druid"))]
        self.glyphs.get(glyph)
    }

    /// Returns a reference to the given glyph, behind an `Arc`, if it exists.
    #[cfg(feature = "druid")]
    pub fn get_glyph_raw<K>(&self, glyph: &K) -> Option<&Arc<Glyph>>
    where
        GlyphName: Borrow<K>,
        K: Ord + ?Sized,
    {
        self.glyphs.get(glyph)
    }

    /// Returns a mutable reference to the glyph with the given name, if it exists.
    pub fn get_glyph_mut<K>(&mut self, glyph: &K) -> Option<&mut Glyph>
    where
        GlyphName: Borrow<K>,
        K: Ord + ?Sized,
    {
        #[cfg(feature = "druid")]
        return self.glyphs.get_mut(glyph).map(Arc::make_mut);
        #[cfg(not(feature = "druid"))]
        self.glyphs.get_mut(glyph)
    }

    /// Returns `true` if this layer contains a glyph with this `name`.
    pub fn contains_glyph(&self, name: &str) -> bool {
        self.glyphs.contains_key(name)
    }

    /// Adds or updates the given glyph.
    ///
    /// If the glyph does not previously exist, the filename is calculated from
    /// the glyph's name.
    pub fn insert_glyph(
        &mut self,
        #[cfg(feature = "druid")] glyph: impl Into<Arc<Glyph>>,
        #[cfg(not(feature = "druid"))] glyph: impl Into<Glyph>,
    ) {
        let glyph = glyph.into();
        if !self.contents.contains_key(&glyph.name) {
            let path = crate::util::default_file_name_for_glyph_name(&glyph.name);
            self.contents.insert(glyph.name.clone(), path.into());
        }
        self.glyphs.insert(glyph.name.clone(), glyph);
    }

    /// Remove all glyphs in the layer. Leave color and the lib untouched.
    pub fn clear(&mut self) {
        self.contents.clear();
        self.glyphs.clear()
    }

    /// Remove the named glyph from this layer and return it, if it exists.
    ///
    /// **Note**: If the `druid` feature is enabled, this will not return the
    /// removed `Glyph` if there are any other outstanding references to it,
    /// although it will still be removed. In this case, consider using the
    /// `remove_glyph_raw` method instead.
    pub fn remove_glyph(&mut self, name: &str) -> Option<Glyph> {
        self.contents.remove(name);
        #[cfg(feature = "druid")]
        return self.glyphs.remove(name).and_then(|g| Arc::try_unwrap(g).ok());
        #[cfg(not(feature = "druid"))]
        self.glyphs.remove(name)
    }

    /// Remove the named glyph and return it, including the containing `Arc`.
    #[cfg(feature = "druid")]
    pub fn remove_glyph_raw(&mut self, name: &str) -> Option<Arc<Glyph>> {
        self.glyphs.remove(name)
    }

    /// Rename a glyph.
    ///
    /// If `overwrite` is true, and a glyph with the new name exists, it will
    /// be replaced.
    ///
    /// Returns an error if `overwrite` is false but a glyph with the new
    /// name exists, or if no glyph with the old name exists
    pub fn rename_glyph(
        &mut self,
        old: &str,
        new: &str,
        overwrite: bool,
    ) -> Result<(), NamingError> {
        if !overwrite && self.glyphs.contains_key(new) {
            Err(NamingError::Duplicate(new.into()))
        } else if !self.glyphs.contains_key(old) {
            Err(NamingError::Missing(old.into()))
        } else {
            #[cfg(feature = "druid")]
            {
                let mut g = self.remove_glyph_raw(old).unwrap();
                Arc::make_mut(&mut g).name = new.into();
                self.insert_glyph(g);
            }
            #[cfg(not(feature = "druid"))]
            {
                let mut g = self.remove_glyph(old).unwrap();
                g.name = new.into();
                self.insert_glyph(g);
            }
            Ok(())
        }
    }

    /// Returns an iterator over the glyphs in this layer.
    pub fn iter(&self) -> impl Iterator<Item = &Glyph> + '_ {
        #[cfg(feature = "druid")]
        return self.glyphs.values().map(|g| g.deref());
        #[cfg(not(feature = "druid"))]
        self.glyphs.values()
    }

    /// Returns an iterator over the glyphs in this layer.
    #[cfg(feature = "druid")]
    pub fn iter_raw(&self) -> impl Iterator<Item = &Arc<Glyph>> + '_ {
        self.glyphs.values()
    }

    /// Returns an iterator over the glyphs in this layer, mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Glyph> {
        #[cfg(feature = "druid")]
        return self.glyphs.values_mut().map(Arc::make_mut);
        #[cfg(not(feature = "druid"))]
        self.glyphs.values_mut()
    }

    /// Returns the path to the .glif file of a given glyph `name`.
    ///
    /// The returned path is relative to the path of the current layer.
    pub fn get_path(&self, name: &str) -> Option<&Path> {
        self.contents.get(name).map(PathBuf::as_path)
    }
}

impl Default for Layer {
    fn default() -> Self {
        Layer::new(DEFAULT_LAYER_NAME.into(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    #[allow(clippy::float_cmp)]
    fn load_layer() {
        let layer_path = "testdata/MutatorSansLightWide.ufo/glyphs";
        assert!(Path::new(layer_path).exists(), "missing test data. Did you `git submodule init`?");
        let layer = Layer::load(layer_path, DEFAULT_LAYER_NAME.into()).unwrap();
        assert_eq!(
            layer.color.as_ref().unwrap(),
            &Color { red: 1.0, green: 0.75, blue: 0.0, alpha: 0.7 }
        );
        assert_eq!(
            layer.lib.get("com.typemytype.robofont.segmentType").unwrap().as_string().unwrap(),
            "curve"
        );
        let glyph = layer.get_glyph("A").expect("failed to load glyph 'A'");
        assert_eq!(glyph.height, 0.);
        assert_eq!(glyph.width, 1190.);
        assert_eq!(glyph.codepoints, vec!['A']);
    }

    #[test]
    fn load_write_layerinfo() {
        let layer_path = "testdata/MutatorSansLightWide.ufo/glyphs";
        assert!(Path::new(layer_path).exists(), "missing test data. Did you `git submodule init`?");
        let mut layer = Layer::load(layer_path, DEFAULT_LAYER_NAME.into()).unwrap();

        layer.color.replace(Color { red: 0.5, green: 0.5, blue: 0.5, alpha: 0.5 });
        layer.lib.insert(
            "com.typemytype.robofont.segmentType".into(),
            plist::Value::String("test".into()),
        );

        let temp_dir = tempdir::TempDir::new("test.ufo").unwrap();
        let dir = temp_dir.path().join("glyphs");
        layer.save(&dir).unwrap();
        let layer2 = Layer::load(&dir, DEFAULT_LAYER_NAME.into()).unwrap();

        assert_eq!(
            layer2.color.as_ref().unwrap(),
            &Color { red: 0.5, green: 0.5, blue: 0.5, alpha: 0.5 }
        );
        assert_eq!(
            layer2.lib.get("com.typemytype.robofont.segmentType").unwrap().as_string().unwrap(),
            "test"
        );
    }

    #[test]
    fn skip_writing_empty_layerinfo() {
        let mut layer = Layer::default();
        let temp_dir = tempdir::TempDir::new("test.ufo").unwrap();
        let dir = temp_dir.path().join("glyphs");

        layer.save(&dir).unwrap();
        assert!(!dir.join("layerinfo.plist").exists());

        fs::remove_dir_all(&dir).unwrap();
        layer.lib = Plist::new();
        layer.save(&dir).unwrap();
        assert!(!dir.join("layerinfo.plist").exists());
    }

    #[test]
    fn delete() {
        let layer_path = "testdata/MutatorSansLightWide.ufo/glyphs";
        let mut layer = Layer::load(layer_path, DEFAULT_LAYER_NAME.into()).unwrap();
        layer.remove_glyph("A");
        if let Some(glyph) = layer.get_glyph("A") {
            panic!("{:?}", glyph);
        }

        if let Some(path) = layer.get_path("A") {
            panic!("{:?}", path);
        }
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn set_glyph() {
        let layer_path = "testdata/MutatorSansLightWide.ufo/glyphs";
        let mut layer = Layer::load(layer_path, DEFAULT_LAYER_NAME.into()).unwrap();
        let mut glyph = Glyph::new_named("A");
        glyph.width = 69.;
        layer.insert_glyph(glyph);
        let glyph = layer.get_glyph("A").expect("failed to load glyph 'A'");
        assert_eq!(glyph.width, 69.);
    }

    #[test]
    fn layer_creation() {
        let mut ufo = crate::Font::load("testdata/MutatorSansLightWide.ufo").unwrap();

        let default_layer = ufo.layers.get_or_create("foreground");
        assert!(!default_layer.is_empty());
        default_layer.clear();

        let background_layer = ufo.layers.get_or_create("background");
        assert!(!background_layer.is_empty());
        background_layer.clear();

        let misc_layer = ufo.layers.get_or_create("misc");
        assert!(misc_layer.is_empty());
        misc_layer.insert_glyph(Glyph::new_named("A"));

        assert!(ufo.default_layer().is_empty());
        assert!(ufo.layers.get("background").unwrap().is_empty());
        assert_eq!(
            ufo.layers
                .get("misc")
                .unwrap()
                .iter()
                .map(|g| g.name.to_string())
                .collect::<Vec<String>>(),
            vec!["A".to_string()]
        );
    }
}
