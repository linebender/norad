use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::glyph::GlyphName;
use crate::{Error, Glyph};

static CONTENTS_FILE: &str = "contents.plist";
//static LAYER_INFO_FILE: &str = "layerinfo.plist";

/// A [layer], corresponding to a 'glyphs' directory. Conceptually, a layer
/// is just a collection of glyphs.
///
/// [layer]: http://unifiedfontobject.org/versions/ufo3/glyphs/
#[allow(dead_code)] // path is unused, but we'll need it when we save
pub struct Layer {
    path: PathBuf,
    contents: BTreeMap<GlyphName, PathBuf>,
    loaded: BTreeMap<GlyphName, Rc<Glyph>>,
}

impl Layer {
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Layer, Error> {
        let path = path.into();
        let contents_path = path.join(CONTENTS_FILE);
        let contents: BTreeMap<GlyphName, PathBuf> = plist::from_file(contents_path)?;
        let mut loaded = BTreeMap::new();
        for (name, glyph_path) in contents.iter() {
            let glyph_path = path.join(glyph_path);
            let glyph = Glyph::load(glyph_path)?;
            loaded.insert(name.clone(), Rc::new(glyph));
        }
        Ok(Layer { path, contents, loaded })
    }

    /// Returns the glyph with the given name, if it exists.
    pub fn get_glyph(&self, glyph: &str) -> Option<Rc<Glyph>> {
        self.loaded.get(glyph).map(Rc::clone)
    }

    /// Returns `true` if this layer contains a glyph with this name.
    pub fn contains_glyph(&self, name: &str) -> bool {
        self.loaded.contains_key(name) | self.contents.contains_key(name)
    }

    /// Set the given glyph. The name is taken from the glyph's `name` field.
    /// This replaces any existing glyph with this name.
    pub fn set_glyph<P: Into<PathBuf>>(&mut self, path: P, glyph: Glyph) {
        //FIXME: figure out what bookkeeping we have to do with this path
        let _path = path.into();
        let name = glyph.name.clone();
        self.loaded.insert(name, Rc::new(glyph));
    }

    /// Remove the named glyph from this layer.
    pub fn delete_glyph(&mut self, name: &str) {
        self.loaded.remove(name);
        self.contents.remove(name);
    }

    /// Iterate over the glyphs in this layer.
    pub fn iter_contents<'a>(&'a self) -> impl Iterator<Item = Rc<Glyph>> + 'a {
        self.loaded.values().map(Rc::clone)
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
