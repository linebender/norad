use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::glyph::GlyphName;
use crate::{Error, Glyph};

static CONTENTS_FILE: &str = "contents.plist";
//static LAYER_INFO_FILE: &str = "layerinfo.plist";

/// A [layer], corresponding to a 'glyphs' directory. Conceptually, a layer
/// is just a collection of glyphs.
///
/// [layer]: http://unifiedfontobject.org/versions/ufo3/glyphs/
#[derive(Debug, Default)]
pub struct Layer {
    pub glyphs: BTreeMap<GlyphName, Rc<Glyph>>,
}

impl Layer {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Layer, Error> {
        let path = path.as_ref();
        let contents_path = path.join(CONTENTS_FILE);
        // these keys are never used; a future optimization would be to skip the
        // names and deserialize to a vec; that would not be a one-liner, though.
        let contents: BTreeMap<String, PathBuf> = plist::from_file(contents_path)?;
        let mut glyphs = BTreeMap::new();
        for (_, glyph_path) in contents.iter() {
            let glyph_path = path.join(glyph_path);
            let glyph = Glyph::load(glyph_path)?;
            // reuse the name in the glyph to avoid having two copies of each
            glyphs.insert(glyph.name.clone(), Rc::new(glyph));
        }
        Ok(Layer { glyphs })
    }

    /// Returns a reference the glyph with the given name, if it exists.
    pub fn get_glyph<K>(&self, glyph: &K) -> Option<&Rc<Glyph>>
    where
        GlyphName: Borrow<K>,
        K: Ord + ?Sized,
    {
        self.glyphs.get(glyph)
    }

    /// Returns a mutable reference to the glyph with the given name, if it exists.
    pub fn get_glyph_mut<K>(&mut self, glyph: &K) -> Option<&mut Rc<Glyph>>
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

    /// Set the given glyph. The name is taken from the glyph's `name` field.
    /// This replaces any existing glyph with this name.
    pub fn set_glyph<P: Into<PathBuf>>(&mut self, path: P, glyph: Glyph) {
        //FIXME: figure out what bookkeeping we have to do with this path
        let _path = path.into();
        let name = glyph.name.clone();
        self.glyphs.insert(name, Rc::new(glyph));
    }

    /// Remove the named glyph from this layer.
    pub fn delete_glyph(&mut self, name: &str) {
        self.glyphs.remove(name);
    }

    /// Iterate over the glyphs in this layer.
    pub fn iter_contents<'a>(&'a self) -> impl Iterator<Item = Rc<Glyph>> + 'a {
        self.glyphs.values().map(Rc::clone)
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
        assert_eq!(glyph.advance, Some(Advance::Width(1290.)));
        assert_eq!(glyph.codepoints.as_ref().map(Vec::len), Some(1));
        assert_eq!(glyph.codepoints.as_ref().unwrap()[0], 'A');
    }

    #[test]
    fn delete() {
        let layer_path = "testdata/mutatorSans/MutatorSansBoldWide.ufo/glyphs";
        let mut layer = Layer::load(layer_path).unwrap();
        layer.delete_glyph("A");
        if let Some(glyph) = layer.get_glyph("A") {
            panic!("{:?}", glyph);
        }
    }

    #[test]
    fn set_glyph() {
        let layer_path = "testdata/mutatorSans/MutatorSansBoldWide.ufo/glyphs";
        let mut layer = Layer::load(layer_path).unwrap();
        let mut glyph = Glyph::new_named("A");
        glyph.advance = Some(Advance::Height(69.));
        layer.set_glyph("A_.glif", glyph);
        let glyph = layer.get_glyph("A").expect("failed to load glyph 'A'");
        assert_eq!(glyph.advance, Some(Advance::Height(69.)));
    }
}
