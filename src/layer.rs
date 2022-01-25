use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "druid")]
use std::{ops::Deref, sync::Arc};

#[cfg(feature = "rayon")]
use rayon::prelude::*;

use crate::error::{FontLoadError, LayerLoadError, LayerWriteError, NamingError};
use crate::names::NameList;
use crate::shared_types::Color;
use crate::Name;
use crate::{util, Glyph, Plist, WriteOptions};

static CONTENTS_FILE: &str = "contents.plist";
static LAYER_INFO_FILE: &str = "layerinfo.plist";

pub(crate) static LAYER_CONTENTS_FILE: &str = "layercontents.plist";
pub(crate) static DEFAULT_LAYER_NAME: &str = "public.default";
pub(crate) static DEFAULT_GLYPHS_DIRNAME: &str = "glyphs";

/// A collection of [`Layer`] objects.
///
/// A layer set always includes a default layer, and may also include additional
/// layers.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerSet {
    /// A collection of [`Layer`]s.  The first [`Layer`] is the default.
    layers: Vec<Layer>,
    /// A set of lowercased layer paths (excluding the default layer, as it is
    /// always unique) for clash detection. This relies on Layer.path being
    /// immutable.
    path_set: HashSet<String>,
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
        let to_load: Vec<(Name, PathBuf)> = if layer_contents_path.exists() {
            plist::from_file(&layer_contents_path)
                .map_err(|source| FontLoadError::ParsePlist { name: LAYER_CONTENTS_FILE, source })?
        } else {
            vec![(Name::new_raw(DEFAULT_LAYER_NAME), PathBuf::from(DEFAULT_GLYPHS_DIRNAME))]
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

        Ok(LayerSet { layers, path_set: HashSet::new() })
    }

    /// Returns a new [`LayerSet`] from a `layers` collection.
    ///
    /// Will panic if `layers` is empty.
    pub fn new(mut layers: Vec<Layer>) -> Self {
        assert!(!layers.is_empty());
        layers.first_mut().unwrap().path = DEFAULT_GLYPHS_DIRNAME.into();
        LayerSet { layers, path_set: HashSet::new() }
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
    pub fn names(&self) -> impl Iterator<Item = &Name> {
        self.layers.iter().map(|l| &l.name)
    }

    /// Returns a new layer with the given name.
    pub fn new_layer(&mut self, name: &str) -> Result<&mut Layer, NamingError> {
        if name == DEFAULT_LAYER_NAME {
            Err(NamingError::ReservedName)
        } else if self.layers.iter().any(|l| l.name == name) {
            Err(NamingError::Duplicate(name.to_string()))
        } else {
            let name = Name::new(name).map_err(|_| NamingError::Invalid(name.into()))?;
            let path = crate::util::default_file_name_for_layer_name(&name, &self.path_set);
            let layer = Layer::new(name, path);
            self.path_set.insert(layer.path.to_string_lossy().to_lowercase());
            self.layers.push(layer);
            Ok(self.layers.last_mut().unwrap())
        }
    }

    /// Remove a layer.
    ///
    /// The default layer cannot be removed.
    pub fn remove(&mut self, name: &str) -> Option<Layer> {
        let removed_layer = self
            .layers
            .iter()
            .skip(1)
            .position(|l| l.name.as_ref() == name)
            .map(|idx| self.layers.remove(idx + 1));

        if let Some(layer) = &removed_layer {
            self.path_set.remove(&layer.path.to_string_lossy().to_lowercase());
        }

        removed_layer
    }

    /// Rename a layer.
    ///
    /// If `overwrite` is true, and a layer with the new name exists, it will
    /// be replaced.
    ///
    /// Returns an error if `overwrite` is false but a layer with the new
    /// name exists, if no layer with the old name exists, if the new name
    /// is not a valid [`Name`] or when anything but the default layer should
    /// be renamed to "public.default".
    pub fn rename_layer(
        &mut self,
        old: &str,
        new: &str,
        overwrite: bool,
    ) -> Result<(), NamingError> {
        if !overwrite && self.get(new).is_some() {
            Err(NamingError::Duplicate(new.to_string()))
        } else if self.get(old).is_none() {
            Err(NamingError::Missing(old.into()))
        } else if new == DEFAULT_LAYER_NAME && self.layers[0].name != old {
            Err(NamingError::ReservedName)
        } else {
            let name = Name::new(new)?;
            if overwrite {
                self.remove(&name);
            }

            // Dance around the borrow checker by using indices instead of references.
            let layer_pos = self.layers.iter().position(|l| l.name.as_ref() == old).unwrap();

            if layer_pos == 0 {
                // Default layer: just change the name.
                let layer = &mut self.layers[layer_pos];
                layer.name = name;
            } else {
                // Non-default layer.
                let old_path = self.layers[layer_pos].path.to_string_lossy().to_lowercase();
                self.path_set.remove(&old_path);
                let new_path = crate::util::default_file_name_for_layer_name(&name, &self.path_set);
                self.path_set.insert(new_path.to_string_lossy().to_lowercase());

                let layer = &mut self.layers[layer_pos];
                layer.name = name;
                layer.path = new_path;
            }

            Ok(())
        }
    }
}

impl Default for LayerSet {
    fn default() -> Self {
        let layers = vec![Layer::default()];
        LayerSet { layers, path_set: HashSet::new() }
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
    pub(crate) glyphs: BTreeMap<Name, Arc<Glyph>>,
    #[cfg(not(feature = "druid"))]
    pub(crate) glyphs: BTreeMap<Name, Glyph>,
    pub(crate) name: Name,
    pub(crate) path: PathBuf,
    contents: BTreeMap<Name, PathBuf>,
    /// A set of lowercased glif file names (excluding the default layer, as it
    /// is always unique) for clash detection.
    path_set: HashSet<String>,
    /// An optional color, specified in the layer's [`layerinfo.plist`][info].
    ///
    /// [info]: https://unifiedfontobject.org/versions/ufo3/glyphs/layerinfo.plist/
    pub color: Option<Color>,
    /// Optional lib data for this layer.
    ///
    /// An empty lib is not serialized.
    pub lib: Plist,
}

impl Layer {
    /// Returns a new [`Layer`] with the provided `name` and `path`.
    ///
    /// The `path` argument will be the directory within the UFO that the layer
    /// is saved.
    pub(crate) fn new(name: Name, path: PathBuf) -> Self {
        Layer {
            glyphs: BTreeMap::new(),
            name,
            path,
            contents: BTreeMap::new(),
            path_set: HashSet::new(),
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
    pub(crate) fn load(path: impl AsRef<Path>, name: &str) -> Result<Layer, LayerLoadError> {
        let path = path.as_ref();
        let names = NameList::default();
        let name = Name::new_raw(name);
        Layer::load_impl(path, name, &names)
    }

    /// The actual loading logic.
    ///
    /// `names` is a map of glyphnames; we pass it throughout parsing
    /// so that we reuse the same Arc<str> for identical names.
    pub(crate) fn load_impl(
        path: &Path,
        name: Name,
        names: &NameList,
    ) -> Result<Layer, LayerLoadError> {
        let contents_path = path.join(CONTENTS_FILE);
        if !contents_path.exists() {
            return Err(LayerLoadError::MissingContentsFile);
        }
        // these keys are never used; a future optimization would be to skip the
        // names and deserialize to a vec; that would not be a one-liner, though.
        let contents: BTreeMap<Name, PathBuf> = plist::from_file(&contents_path)
            .map_err(|source| LayerLoadError::ParsePlist { name: CONTENTS_FILE, source })?;
        let path_set = contents.values().map(|p| p.to_string_lossy().to_lowercase()).collect();

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

        Ok(Layer { glyphs, name, path, contents, path_set, color, lib })
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
    pub fn name(&self) -> &Name {
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
    pub fn get_glyph(&self, glyph: &str) -> Option<&Glyph> {
        #[cfg(feature = "druid")]
        return self.glyphs.get(glyph).map(|g| g.deref());
        #[cfg(not(feature = "druid"))]
        self.glyphs.get(glyph)
    }

    /// Returns a reference to the given glyph, behind an `Arc`, if it exists.
    #[cfg(feature = "druid")]
    pub fn get_glyph_raw(&self, glyph: &str) -> Option<&Arc<Glyph>> {
        self.glyphs.get(glyph)
    }

    /// Returns a mutable reference to the glyph with the given name, if it exists.
    pub fn get_glyph_mut(&mut self, glyph: &str) -> Option<&mut Glyph> {
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
            let path = crate::util::default_file_name_for_glyph_name(&glyph.name, &self.path_set);
            self.path_set.insert(path.to_string_lossy().to_lowercase());
            self.contents.insert(glyph.name.clone(), path);
        }
        self.glyphs.insert(glyph.name.clone(), glyph);
    }

    /// Remove all glyphs in the layer. Leave color and the lib untouched.
    pub fn clear(&mut self) {
        self.contents.clear();
        self.path_set.clear();
        self.glyphs.clear()
    }

    /// Remove the named glyph from this layer and return it, if it exists.
    ///
    /// **Note**: If the `druid` feature is enabled, this will not return the
    /// removed `Glyph` if there are any other outstanding references to it,
    /// although it will still be removed. In this case, consider using the
    /// `remove_glyph_raw` method instead.
    pub fn remove_glyph(&mut self, name: &str) -> Option<Glyph> {
        if let Some(path) = self.contents.remove(name) {
            self.path_set.remove(&path.to_string_lossy().to_lowercase());
        }
        #[cfg(feature = "druid")]
        return self.glyphs.remove(name).and_then(|g| Arc::try_unwrap(g).ok());
        #[cfg(not(feature = "druid"))]
        self.glyphs.remove(name)
    }

    /// Remove the named glyph and return it, including the containing `Arc`.
    #[cfg(feature = "druid")]
    pub fn remove_glyph_raw(&mut self, name: &str) -> Option<Arc<Glyph>> {
        if let Some(path) = self.contents.remove(name) {
            self.path_set.remove(&path.to_string_lossy().to_lowercase());
        }
        self.glyphs.remove(name)
    }

    /// Rename a glyph.
    ///
    /// If `overwrite` is true, and a glyph with the new name exists, it will
    /// be replaced.
    ///
    /// Returns an error if `overwrite` is false but a glyph with the new
    /// name exists, or if no glyph with the old name exists, or if the new
    /// name is not a valid [`Name`].
    pub fn rename_glyph(
        &mut self,
        old: &str,
        new: &str,
        overwrite: bool,
    ) -> Result<(), NamingError> {
        if !overwrite && self.glyphs.contains_key(new) {
            Err(NamingError::Duplicate(new.to_string()))
        } else if !self.glyphs.contains_key(old) {
            Err(NamingError::Missing(old.into()))
        } else {
            let name = Name::new(new).map_err(|_| NamingError::Invalid(new.into()))?;
            #[cfg(feature = "druid")]
            {
                let mut g = self.remove_glyph_raw(old).unwrap();
                Arc::make_mut(&mut g).name = name;
                self.insert_glyph(g);
            }
            #[cfg(not(feature = "druid"))]
            {
                let mut g = self.remove_glyph(old).unwrap();
                g.name = name;
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
        Layer::new(Name::new_raw(DEFAULT_LAYER_NAME), DEFAULT_GLYPHS_DIRNAME.into())
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
        let layer = Layer::load(layer_path, DEFAULT_LAYER_NAME).unwrap();
        assert_eq!(layer.color.as_ref().unwrap(), &Color::new(1.0, 0.75, 0.0, 0.7).unwrap());
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
        let mut layer = Layer::load(layer_path, DEFAULT_LAYER_NAME).unwrap();

        layer.color.replace(Color::new(0.5, 0.5, 0.5, 0.5).unwrap());
        layer.lib.insert(
            "com.typemytype.robofont.segmentType".into(),
            plist::Value::String("test".into()),
        );

        let temp_dir = tempdir::TempDir::new("test.ufo").unwrap();
        let dir = temp_dir.path().join("glyphs");
        layer.save(&dir).unwrap();
        let layer2 = Layer::load(&dir, DEFAULT_LAYER_NAME).unwrap();

        assert_eq!(layer2.color.as_ref().unwrap(), &Color::new(0.5, 0.5, 0.5, 0.5).unwrap());
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
        let mut layer = Layer::load(layer_path, DEFAULT_LAYER_NAME).unwrap();
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
        let mut layer = Layer::load(layer_path, DEFAULT_LAYER_NAME).unwrap();
        let mut glyph = Glyph::new_named("A");
        glyph.width = 69.;
        layer.insert_glyph(glyph);
        let glyph = layer.get_glyph("A").expect("failed to load glyph 'A'");
        assert_eq!(glyph.width, 69.);
    }

    #[test]
    fn layer_creation() {
        let mut ufo = crate::Font::load("testdata/MutatorSansLightWide.ufo").unwrap();

        let default_layer = ufo.layers.get_mut("foreground").unwrap();
        assert!(!default_layer.is_empty());
        default_layer.clear();

        let background_layer = ufo.layers.get_mut("background").unwrap();
        assert!(!background_layer.is_empty());
        background_layer.clear();

        let misc_layer = ufo.layers.new_layer("misc").unwrap();
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

    #[test]
    fn rename_layer() {
        let mut layer_set = LayerSet::default();

        // Non-default layers can be renamed and get a new path.
        layer_set.new_layer("aaa").unwrap();
        assert_eq!(layer_set.get("aaa").unwrap().path().as_os_str(), "glyphs.aaa");

        layer_set.rename_layer("aaa", "bbb", false).unwrap();
        assert!(layer_set.get("aaa").is_none());
        assert_eq!(layer_set.get("bbb").unwrap().path().as_os_str(), "glyphs.bbb");

        layer_set.rename_layer("bbb", "aaa", false).unwrap();
        assert_eq!(layer_set.get("aaa").unwrap().path().as_os_str(), "glyphs.aaa");
        assert!(layer_set.get("bbb").is_none());
    }

    #[test]
    fn rename_layer_overwrite() {
        let mut layer_set = LayerSet::default();

        // Non-default layers can be renamed and get a new path.
        layer_set.new_layer("aaa").unwrap();
        layer_set.new_layer("bbb").unwrap();
        assert_eq!(layer_set.get("aaa").unwrap().path().as_os_str(), "glyphs.aaa");
        assert_eq!(layer_set.get("bbb").unwrap().path().as_os_str(), "glyphs.bbb");

        layer_set.rename_layer("aaa", "bbb", true).unwrap();
        assert!(layer_set.get("aaa").is_none());
        assert_eq!(layer_set.get("bbb").unwrap().path().as_os_str(), "glyphs.bbb");

        layer_set.rename_layer("bbb", "aaa", false).unwrap();
        assert_eq!(layer_set.get("aaa").unwrap().path().as_os_str(), "glyphs.aaa");
        assert!(layer_set.get("bbb").is_none());
    }

    #[test]
    #[should_panic(expected = "Reserved")]
    fn rename_layer_nondefault_default() {
        let mut layer_set = LayerSet::default();

        layer_set.rename_layer("public.default", "foreground", false).unwrap();

        // "public.default" is the reserved name for the actual default layer.
        layer_set.new_layer("aaa").unwrap();
        layer_set.rename_layer("aaa", "public.default", true).unwrap();
    }

    #[test]
    fn rename_default_layer() {
        let mut layer_set = LayerSet::default();

        // The default layer can be renamed but the path stays the same.
        layer_set.rename_layer("public.default", "aaa", false).unwrap();

        assert_eq!(*layer_set.default_layer().name(), "aaa");
        assert_eq!(layer_set.default_layer().path().as_os_str(), "glyphs");

        // Renaming back must work.
        layer_set.rename_layer("aaa", "public.default", false).unwrap();

        assert_eq!(*layer_set.default_layer().name(), "public.default");
        assert_eq!(layer_set.default_layer().path().as_os_str(), "glyphs");

        layer_set.rename_layer("public.default", "aaa", false).unwrap();

        assert_eq!(*layer_set.default_layer().name(), "aaa");
        assert_eq!(layer_set.default_layer().path().as_os_str(), "glyphs");
    }

    #[test]
    fn rename_default_layer_overwrite() {
        let mut layer_set = LayerSet::default();

        // The default layer can be renamed but the path stays the same.
        layer_set.new_layer("aaa").unwrap();
        layer_set.rename_layer("public.default", "aaa", true).unwrap();

        assert_eq!(*layer_set.default_layer().name(), "aaa");
        assert_eq!(layer_set.default_layer().path().as_os_str(), "glyphs");
        assert!(layer_set.get("public.default").is_none());

        // Renaming back must work.
        layer_set.rename_layer("aaa", "public.default", true).unwrap();

        assert_eq!(*layer_set.default_layer().name(), "public.default");
        assert_eq!(layer_set.default_layer().path().as_os_str(), "glyphs");
        assert!(layer_set.get("aaa").is_none());

        layer_set.new_layer("aaa").unwrap();
        layer_set.rename_layer("public.default", "aaa", true).unwrap();

        assert_eq!(*layer_set.default_layer().name(), "aaa");
        assert_eq!(layer_set.default_layer().path().as_os_str(), "glyphs");
        assert!(layer_set.get("public.default").is_none());
    }

    #[test]
    fn layerset_duplicate_paths() {
        let mut layer_set = LayerSet::default();

        layer_set.new_layer("Ab").unwrap();
        assert_eq!(layer_set.get("Ab").unwrap().path().as_os_str(), "glyphs.A_b");

        layer_set.new_layer("a_b").unwrap();
        assert_eq!(layer_set.get("a_b").unwrap().path().as_os_str(), "glyphs.a_b000000000000001");

        layer_set.remove("Ab");
        layer_set.new_layer("Ab").unwrap();
        assert_eq!(layer_set.get("Ab").unwrap().path().as_os_str(), "glyphs.A_b");
    }

    #[test]
    fn layer_duplicate_paths() {
        let mut layer = Layer::default();

        layer.insert_glyph(Glyph::new_named("Ab"));
        assert_eq!(layer.contents.get("Ab").unwrap().as_os_str(), "A_b.glif");

        layer.insert_glyph(Glyph::new_named("a_b"));
        assert_eq!(layer.contents.get("a_b").unwrap().as_os_str(), "a_b000000000000001.glif");

        layer.remove_glyph("Ab");
        layer.insert_glyph(Glyph::new_named("Ab"));
        assert_eq!(layer.contents.get("Ab").unwrap().as_os_str(), "A_b.glif");
    }
}
