
use std::collections::BTreeMap;
use std::rc::Rc;
use std::cell::RefCell;

use norad::{MetaInfo, GlyphName, GlifVersion, Guideline, Anchor, Component, Contour};

type SharedObj<T> = Rc<RefCell<T>>;

#[derive(Debug, Clone)]
struct SharedList<T> {
    items: Rc<RefCell<T>>,
}

pub struct PyFont {
    metainfo: SharedObj<MetaInfo>,

}

// acts like a dictionary of str: layer
// len()
// iterator over layers
// __contains__
// __get__
// __del__
// .defaultLayer
// newLayer(name, **kwargs) create and return a layer
// renameGlyph(name, newName, overwrite) rename across all glyphs. if 'overwrite' is false,
// raises an exception if the new name already exists
pub struct PyLayerSet {
    default: PyLayer,
    layers: SharedList<PyLayer>,
}

#[derive(Debug, Clone)]
pub struct PyLayer {
    glyphs: SharedObj<BTreeMap<GlyphName, SharedObj<PyGlyph>>>,
}

#[derive(Debug, Clone)]
pub struct PyGlyph {
    name: GlyphName,
    format: GlifVersion,
    height: f32,
    width: f32,
    codepoints: Vec<char>,
    note: SharedObj<String>,
    guidelines: SharedList<SharedObj<Guideline>>,
    anchors: SharedList<SharedObj<Anchor>>,
    components: SharedList<SharedObj<Component>>,
    contours: SharedList<SharedObj<Contour>>,
}
