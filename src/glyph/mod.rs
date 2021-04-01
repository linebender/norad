//! Data related to individual glyphs.

use std::path::Path;

#[cfg(feature = "druid")]
use druid::{Data, Lens};

use crate::error::{Error, ErrorKind, GlifError, GlifErrorInternal};
use crate::names::NameList;
use crate::shared_types::PUBLIC_OBJECT_LIBS_KEY;
use crate::{
    AffineTransform, Anchor, Color, Component, Contour, GlyphName, Guideline, Identifier, Image,
    Line, Plist,
};

pub mod affinetransform;
pub mod anchor;
pub mod builder;
pub mod component;
pub mod contour;
pub mod image;
mod parse;
pub mod point;
mod serialize;
#[cfg(test)]
mod tests;

/// A glyph, loaded from a [.glif file][glif].
///
/// [glif]: http://unifiedfontobject.org/versions/ufo3/glyphs/glif/
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "druid", derive(Lens))]
pub struct Glyph {
    pub name: GlyphName,
    pub format: GlifVersion,
    pub height: f32,
    pub width: f32,
    pub codepoints: Vec<char>,
    pub note: Option<String>,
    pub guidelines: Vec<Guideline>,
    pub anchors: Vec<Anchor>,
    pub components: Vec<Component>,
    pub contours: Vec<Contour>,
    pub image: Option<Image>,
    pub lib: Plist,
}

impl Glyph {
    /// Load the glyph at this path.
    ///
    /// When loading glyphs in bulk, `load_with_names` should be preferred,
    /// since it will allow glyph names (in glyphs and components) to be shared
    /// between instances.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let names = NameList::default();
        Glyph::load_with_names(path, &names)
    }

    pub fn load_with_names(path: &Path, names: &NameList) -> Result<Self, Error> {
        let data = std::fs::read(path)?;
        parse::GlifParser::from_xml(&data, Some(names)).map_err(|e| match e {
            GlifErrorInternal::Xml(e) => e.into(),
            GlifErrorInternal::Spec { kind, position } => {
                GlifError { kind, position, path: Some(path.to_owned()) }.into()
            }
        })
    }

    #[doc(hidden)]
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        if self.format != GlifVersion::V2 {
            return Err(Error::DowngradeUnsupported);
        }
        if self.lib.contains_key(PUBLIC_OBJECT_LIBS_KEY) {
            return Err(Error::PreexistingPublicObjectLibsKey);
        }
        let data = self.encode_xml()?;
        std::fs::write(path, &data)?;
        Ok(())
    }

    /// Create a new glyph with the given name.
    pub fn new_named<S: Into<GlyphName>>(name: S) -> Self {
        Glyph::new(name.into(), GlifVersion::V2)
    }

    pub(crate) fn new(name: GlyphName, format: GlifVersion) -> Self {
        Glyph {
            name,
            format,
            height: 0.0,
            width: 0.0,
            codepoints: Vec::new(),
            note: None,
            guidelines: Vec::new(),
            anchors: Vec::new(),
            components: Vec::new(),
            contours: Vec::new(),
            image: None,
            lib: Plist::new(),
        }
    }

    /// Move libs from the lib's `public.objectLibs` into the actual objects.
    /// The key will be removed from the glyph lib.
    fn load_object_libs(&mut self) -> Result<(), ErrorKind> {
        let mut object_libs = match self.lib.remove(PUBLIC_OBJECT_LIBS_KEY) {
            Some(lib) => lib.into_dictionary().ok_or(ErrorKind::BadLib)?,
            None => return Ok(()),
        };

        for anchor in &mut self.anchors {
            if let Some(lib) = anchor.identifier().and_then(|id| object_libs.remove(id.as_str())) {
                let lib = lib.into_dictionary().ok_or(ErrorKind::BadLib)?;
                anchor.replace_lib(lib);
            }
        }

        for guideline in &mut self.guidelines {
            if let Some(lib) = guideline.identifier().and_then(|id| object_libs.remove(id.as_str()))
            {
                let lib = lib.into_dictionary().ok_or(ErrorKind::BadLib)?;
                guideline.replace_lib(lib);
            }
        }

        for contour in &mut self.contours {
            if let Some(lib) = contour.identifier().and_then(|id| object_libs.remove(id.as_str())) {
                let lib = lib.into_dictionary().ok_or(ErrorKind::BadLib)?;
                contour.replace_lib(lib);
            }
            for point in &mut contour.points {
                if let Some(lib) = point.identifier().and_then(|id| object_libs.remove(id.as_str()))
                {
                    let lib = lib.into_dictionary().ok_or(ErrorKind::BadLib)?;
                    point.replace_lib(lib);
                }
            }
        }
        for component in &mut self.components {
            if let Some(lib) = component.identifier().and_then(|id| object_libs.remove(id.as_str()))
            {
                let lib = lib.into_dictionary().ok_or(ErrorKind::BadLib)?;
                component.replace_lib(lib);
            }
        }

        Ok(())
    }

    /// Dump guideline libs into a Plist.
    fn dump_object_libs(&self) -> Plist {
        let mut object_libs = Plist::default();

        let mut dump_lib = |id: Option<&Identifier>, lib: &Plist| {
            let id = id.map(|id| id.as_str().to_string());
            object_libs.insert(id.unwrap(), plist::Value::Dictionary(lib.clone()));
        };

        for anchor in &self.anchors {
            if let Some(lib) = anchor.lib() {
                dump_lib(anchor.identifier(), lib);
            }
        }

        for guideline in &self.guidelines {
            if let Some(lib) = guideline.lib() {
                dump_lib(guideline.identifier(), lib);
            }
        }

        for contour in &self.contours {
            if let Some(lib) = contour.lib() {
                dump_lib(contour.identifier(), lib);
            }
            for point in &contour.points {
                if let Some(lib) = point.lib() {
                    dump_lib(point.identifier(), lib);
                }
            }
        }
        for component in &self.components {
            if let Some(lib) = component.lib() {
                dump_lib(component.identifier(), lib);
            }
        }

        object_libs
    }
}

#[cfg(feature = "druid")]
impl Data for Glyph {
    fn same(&self, other: &Glyph) -> bool {
        self.name.same(&other.name)
            && self.format.same(&other.format)
            && self.height == other.height
            && self.width == other.width
            && self.codepoints == other.codepoints
            && self.note == other.note
            && self.guidelines == other.guidelines
            && self.anchors == other.anchors
            && self.components == other.components
            && self.contours == other.contours
            && self.image == other.image
            && self.lib == other.lib
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "druid", derive(Data))]
pub enum GlifVersion {
    V1 = 1,
    V2 = 2,
}

//NOTE: this is hacky, and intended mostly as a placeholder. It was adapted from
// https://github.com/unified-font-object/ufoLib/blob/master/Lib/ufoLib/filenames.py
/// given a glyph name, compute an appropriate file name.
pub(crate) fn default_file_name_for_glyph_name(name: impl AsRef<str>) -> String {
    fn fn_impl(name: &str) -> String {
        static SPECIAL_ILLEGAL: &[char] = &['\\', '*', '+', '/', ':', '<', '>', '?', '[', ']', '|'];
        static SUFFIX: &str = ".glif";
        const MAX_LEN: usize = 255;

        let mut result = String::with_capacity(name.len());

        for c in name.chars() {
            match c {
                '.' if result.is_empty() => result.push('_'),
                c if (c as u32) < 32 || (c as u32) == 0x7f || SPECIAL_ILLEGAL.contains(&c) => {
                    result.push('_')
                }
                c if c.is_ascii_uppercase() => {
                    result.push(c);
                    result.push('_');
                }
                c => result.push(c),
            }
        }

        //TODO: check for illegal names?
        if result.len() + SUFFIX.len() > MAX_LEN {
            let mut boundary = 255 - SUFFIX.len();
            while !result.is_char_boundary(boundary) {
                boundary -= 1;
            }
            result.truncate(boundary);
        }
        result.push_str(SUFFIX);
        result
    }

    let name = name.as_ref();
    fn_impl(name)
}
