use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::parse::parse_glyph;
use crate::ufo::Glyph;
use crate::Error;

static LAYER_CONTENTS_FILE: &str = "contents.plist";
static LAYER_INFO_FILE: &str = "layerinfo.plist";

enum Glif {
    Loaded(Glyph),
    // Boxed so we can clone
    Errored(Rc<Error>),
}

pub struct Layer {
    path: PathBuf,
    contents: BTreeMap<String, PathBuf>,
    loaded: BTreeMap<String, Glif>,
}

impl Layer {
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<Layer, Error> {
        let path = path.into();
        let contents_path = path.join(LAYER_CONTENTS_FILE);
        let contents = plist::from_file(contents_path)?;
        Ok(Layer { path, contents, loaded: BTreeMap::new() })
    }

    pub fn get_glyph(&mut self, glyph: &str) -> Result<&Glyph, Error> {
        if !self.loaded.contains_key(glyph) {
            self.load_glyph(glyph);
        }

        match self.loaded.get(glyph).expect("glyph always loaded before get") {
            Glif::Loaded(ref g) => return Ok(g),
            Glif::Errored(e) => return Err(Error::SavedError(e.clone())),
            _ => unreachable!(),
        }
    }

    fn load_glyph(&mut self, glyph: &str) {
        let glif = match self.load_glyph_impl(&glyph) {
            Ok(g) => Glif::Loaded(g),
            Err(e) => Glif::Errored(Rc::new(e)),
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
    use std::path::Path;

    #[test]
    fn load_layer() {
        let layer_path = "testdata/mutatorSans/MutatorSansBoldWide.ufo/glyphs";
        assert!(Path::new(layer_path).exists(), "missing test data. Did you `git submodule init`?");
        let mut layer = Layer::load(layer_path).unwrap();
        let glyph = layer.get_glyph("A").expect("failed to load glyph 'A'");
        assert_eq!(glyph.width, Some(1290.));
        assert_eq!(glyph.codepoints.as_ref().map(Vec::len), Some(1));
        assert_eq!(glyph.codepoints.as_ref().unwrap()[0], 'A');
    }
}
