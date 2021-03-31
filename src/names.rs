use std::collections::HashSet;
use std::sync::Arc;

#[cfg(not(feature = "rayon"))]
use std::cell::RefCell;
#[cfg(feature = "rayon")]
use std::sync::RwLock;

/// The name of a glyph.
pub type GlyphName = Arc<str>;

/// Manages interned names
///
/// We store names as `Arc<str>`, and we want to reuse the same pointer
/// for all instances of the same name.
#[derive(Debug, Default)]
pub struct NameList {
    #[cfg(feature = "rayon")]
    inner: ParNameList,
    #[cfg(not(feature = "rayon"))]
    inner: SeqNameList,
}

#[derive(Debug)]
#[cfg(feature = "rayon")]
struct ParNameList(RwLock<HashSet<GlyphName>>);

#[derive(Debug, Default)]
#[cfg(not(feature = "rayon"))]
struct SeqNameList(RefCell<HashSet<GlyphName>>);

impl NameList {
    pub(crate) fn get(&self, name: &GlyphName) -> GlyphName {
        self.inner.get(name)
    }

    pub(crate) fn contains(&self, key: impl AsRef<str>) -> bool {
        self.inner.contains(key)
    }
}

#[cfg(feature = "rayon")]
impl ParNameList {
    pub(crate) fn get(&self, name: &GlyphName) -> GlyphName {
        let existing = self.0.read().unwrap().get(name).cloned();
        match existing {
            Some(name) => name,
            None => {
                self.0.write().unwrap().insert(name.clone());
                name.clone()
            }
        }
    }

    pub(crate) fn contains(&self, key: impl AsRef<str>) -> bool {
        self.0.read().unwrap().contains(key.as_ref())
    }
}

#[cfg(not(feature = "rayon"))]
impl SeqNameList {
    pub(crate) fn get(&self, name: &GlyphName) -> GlyphName {
        let existing = self.0.borrow().get(name).cloned();
        match existing {
            Some(name) => name,
            None => {
                self.0.borrow_mut().insert(name.clone());
                name.clone()
            }
        }
    }

    pub(crate) fn contains(&self, key: impl AsRef<str>) -> bool {
        self.0.borrow().contains(key.as_ref())
    }
}

#[cfg(feature = "rayon")]
impl Default for ParNameList {
    fn default() -> Self {
        ParNameList(RwLock::new(HashSet::new()))
    }
}

impl<T: Into<GlyphName>> std::iter::FromIterator<T> for NameList {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let names = NameList::default();

        for i in iter {
            names.get(&i.into());
        }
        names
    }
}
