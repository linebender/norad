use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(feature = "rayon")]
use rayon::prelude::*;

use crate::glyph::GlyphName;
use crate::names::NameList;
use crate::{Error, Glyph};

static CONTENTS_FILE: &str = "contents.plist";
//static LAYER_INFO_FILE: &str = "layerinfo.plist";

/// A [layer], corresponding to a 'glyphs' directory. Conceptually, a layer
/// is just a collection of glyphs.
///
/// [layer]: http://unifiedfontobject.org/versions/ufo3/glyphs/
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layer {
    pub(crate) glyphs: BTreeMap<GlyphName, Arc<Glyph>>,
    contents: BTreeMap<GlyphName, PathBuf>,
}

impl Layer {
    /// Load the layer at this path.
    ///
    /// Internal callers should use `load_impl` directly, so that glyph names
    /// can be reused between layers.
    pub fn load(path: impl AsRef<Path>) -> Result<Layer, Error> {
        let path = path.as_ref();
        let names = NameList::default();
        Layer::load_impl(path, &names)
    }

    /// the actual loading logic.
    ///
    /// `names` is a map of glyphnames; we pass it throughout parsing
    /// so that we reuse the same Arc<str> for identical names.
    pub(crate) fn load_impl(path: &Path, names: &NameList) -> Result<Layer, Error> {
        let contents_path = path.join(CONTENTS_FILE);
        // these keys are never used; a future optimization would be to skip the
        // names and deserialize to a vec; that would not be a one-liner, though.
        let contents: BTreeMap<GlyphName, PathBuf> = plist::from_file(contents_path)?;

        #[cfg(feature = "rayon")]
        let iter = contents.par_iter();
        #[cfg(not(feature = "rayon"))]
        let iter = contents.iter();

        let glyphs = iter
            .map(|(name, glyph_path)| {
                let name = names.get(name);
                let glyph_path = path.join(glyph_path);

                Glyph::load_with_names(&glyph_path, names).map(|mut glyph| {
                    glyph.name = name.clone();
                    (name, Arc::new(glyph))
                })
            })
            //FIXME: come up with a better way of reporting errors than just aborting at first failure
            .collect::<Result<_, _>>()?;
        Ok(Layer { contents, glyphs })
    }

    /// Attempt to write this layer to the given path.
    ///
    /// The path should not exist.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let path = path.as_ref();
        fs::create_dir(&path)?;
        plist::to_file_xml(path.join(CONTENTS_FILE), &self.contents)?;
        for (name, glyph_path) in self.contents.iter() {
            let glyph = self.glyphs.get(name).expect("all glyphs in contents must exist.");
            glyph.save(path.join(glyph_path))?;
        }

        Ok(())
    }

    /// Returns a reference the glyph with the given name, if it exists.
    pub fn get_glyph<K>(&self, glyph: &K) -> Option<&Arc<Glyph>>
    where
        GlyphName: Borrow<K>,
        K: Ord + ?Sized,
    {
        self.glyphs.get(glyph)
    }

    /// Returns a mutable reference to the glyph with the given name, if it exists.
    pub fn get_glyph_mut<K>(&mut self, glyph: &K) -> Option<&mut Arc<Glyph>>
    where
        GlyphName: Borrow<K>,
        K: Ord + ?Sized,
    {
        self.glyphs.get_mut(glyph)
    }

    /// Returns `true` if this layer contains a glyph with this name.
    pub fn contains_glyph(&self, name: &str) -> bool {
        self.glyphs.contains_key(name)
    }

    /// Adds or updates the given glyph.
    ///
    /// If the glyph does not previously exist, the filename is calculated from
    /// the glyph's name.
    pub fn insert_glyph(&mut self, glyph: impl Into<Arc<Glyph>>) {
        let glyph = glyph.into();
        if !self.contents.contains_key(&glyph.name) {
            let path = crate::glyph::default_file_name_for_glyph_name(&glyph.name);
            self.contents.insert(glyph.name.clone(), path.into());
        }
        self.glyphs.insert(glyph.name.clone(), glyph);
    }

    /// Remove the named glyph from this layer.
    #[doc(hidden)]
    #[deprecated(since = "0.3.0", note = "use remove_glyph instead")]
    pub fn delete_glyph(&mut self, name: &str) {
        self.remove_glyph(name);
    }

    /// Remove the named glyph from this layer and return it, if it exists.
    pub fn remove_glyph(&mut self, name: &str) -> Option<Arc<Glyph>> {
        self.contents.remove(name);
        self.glyphs.remove(name)
    }

    /// Iterate over the glyphs in this layer.
    pub fn iter_contents<'a>(&'a self) -> impl Iterator<Item = Arc<Glyph>> + 'a {
        self.glyphs.values().map(Arc::clone)
    }

    #[cfg(test)]
    pub fn get_path(&self, name: &str) -> Option<&Path> {
        self.contents.get(name).map(PathBuf::as_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph::Advance;
    use std::path::Path;

    #[test]
    fn load_layer() {
        let layer_path = "testdata/mutatorSans/MutatorSansBoldWide.ufo/glyphs";
        assert!(Path::new(layer_path).exists(), "missing test data. Did you `git submodule init`?");
        let layer = Layer::load(layer_path).unwrap();
        let glyph = layer.get_glyph("A").expect("failed to load glyph 'A'");
        assert_eq!(glyph.advance, Some(Advance { width: 1290., height: 0. }));
        assert_eq!(glyph.codepoints.as_ref().map(Vec::len), Some(1));
        assert_eq!(glyph.codepoints.as_ref().unwrap()[0], 'A');
    }

    #[test]
    fn delete() {
        let layer_path = "testdata/mutatorSans/MutatorSansBoldWide.ufo/glyphs";
        let mut layer = Layer::load(layer_path).unwrap();
        layer.remove_glyph("A");
        if let Some(glyph) = layer.get_glyph("A") {
            panic!("{:?}", glyph);
        }

        if let Some(path) = layer.get_path("A") {
            panic!("{:?}", path);
        }
    }

    #[test]
    fn set_glyph() {
        let layer_path = "testdata/mutatorSans/MutatorSansBoldWide.ufo/glyphs";
        let mut layer = Layer::load(layer_path).unwrap();
        let mut glyph = Glyph::new_named("A");
        glyph.advance = Some(Advance { height: 69., width: 0. });
        layer.insert_glyph(glyph);
        let glyph = layer.get_glyph("A").expect("failed to load glyph 'A'");
        assert_eq!(glyph.advance, Some(Advance { height: 69., width: 0. }));
    }
}
