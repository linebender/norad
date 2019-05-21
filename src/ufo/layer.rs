use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::parse::parse_glyph;
use crate::ufo::Glyph;
use crate::Error;

static CONTENTS_FILE: &str = "contents.plist";
//static LAYER_INFO_FILE: &str = "layerinfo.plist";

/// A [layer], corresponding to a 'glyphs' directory. Conceptually, a layer
/// is just a collection of glyphs.
///
/// [layer]: http://unifiedfontobject.org/versions/ufo3/glyphs/
pub struct Layer {
    path: PathBuf,
    contents: BTreeMap<String, PathBuf>,
    loaded: BTreeMap<String, Entry>,
}

enum Entry {
    Loaded(Glyph),
    // Boxed so we can clone
    Errored(Rc<Error>),
}

impl Layer {
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Layer, Error> {
        let path = path.into();
        let contents_path = path.join(CONTENTS_FILE);
        let contents = plist::from_file(contents_path)?;
        Ok(Layer { path, contents, loaded: BTreeMap::new() })
    }

    /// Attempt to load and return the glyph with this name.
    ///
    /// Glyphs are lazily loaded from files on disk, so this function may
    /// fail if a glyph file cannot be read.
    pub fn get_glyph(&mut self, glyph: &str) -> Result<&Glyph, Error> {
        if !self.loaded.contains_key(glyph) {
            self.load_glyph(glyph);
        }

        match self.loaded.get(glyph).expect("glyph always loaded before get") {
            Entry::Loaded(ref g) => return Ok(g),
            Entry::Errored(e) => return Err(Error::SavedError(e.clone())),
            _ => unreachable!(),
        }
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
        self.loaded.insert(name.clone(), Entry::Loaded(glyph));
    }

    /// Remove the named glyph from this layer.
    pub fn delete_glyph(&mut self, name: &str) {
        self.loaded.remove(name);
        self.contents.remove(name);
    }

    fn load_glyph(&mut self, glyph: &str) {
        let glif = match self.load_glyph_impl(&glyph) {
            Ok(g) => Entry::Loaded(g),
            Err(e) => Entry::Errored(Rc::new(e)),
        };
        self.loaded.insert(glyph.to_owned(), glif);
    }

    fn load_glyph_impl(&mut self, glyph: &str) -> Result<Glyph, Error> {
        let path = self.contents.get(glyph).ok_or(Error::MissingGlyph)?;
        let path = self.path.join(path);
        let data = std::fs::read(&path)?;
        parse_glyph(&data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ufo::Advance;
    use std::path::Path;

    #[test]
    fn load_layer() {
        let layer_path = "testdata/mutatorSans/MutatorSansBoldWide.ufo/glyphs";
        assert!(Path::new(layer_path).exists(), "missing test data. Did you `git submodule init`?");
        let mut layer = Layer::load(layer_path).unwrap();
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
        if let Ok(glyph) = layer.get_glyph("A") {
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
