use crate::{Color, Identifier, Plist};

#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    pub x: f32,
    pub y: f32,
    /// An arbitrary name for the anchor.
    pub name: Option<String>,
    pub color: Option<Color>,
    /// Unique identifier for the anchor within the glyph. This attribute is only required
    /// when a lib is present and should otherwise only be added as needed.
    identifier: Option<Identifier>,
    /// The anchor's lib for arbitary data.
    lib: Option<Plist>,
}

impl Anchor {
    pub fn new(
        x: f32,
        y: f32,
        name: Option<String>,
        color: Option<Color>,
        identifier: Option<Identifier>,
        lib: Option<Plist>,
    ) -> Self {
        let mut this = Self { x, y, name, color, identifier: None, lib: None };
        if let Some(id) = identifier {
            this.replace_identifier(id);
        }
        if let Some(lib) = lib {
            this.replace_lib(lib);
        }
        this
    }

    /// Returns an immutable reference to the anchor's lib.
    pub fn lib(&self) -> Option<&Plist> {
        self.lib.as_ref()
    }

    /// Returns a mutable reference to the anchor's lib.
    pub fn lib_mut(&mut self) -> Option<&mut Plist> {
        self.lib.as_mut()
    }

    /// Replaces the actual lib by the lib given in parameter, returning the old
    /// lib if present. Sets a new UUID v4 identifier if none is set already.
    pub fn replace_lib(&mut self, lib: Plist) -> Option<Plist> {
        if self.identifier.is_none() {
            self.identifier.replace(Identifier::from_uuidv4());
        }
        self.lib.replace(lib)
    }

    /// Takes the lib out of the anchor, leaving a None in its place.
    pub fn take_lib(&mut self) -> Option<Plist> {
        self.lib.take()
    }

    /// Returns an immutable reference to the anchor's identifier.
    pub fn identifier(&self) -> Option<&Identifier> {
        self.identifier.as_ref()
    }

    /// Replaces the actual identifier by the identifier given in parameter,
    /// returning the old identifier if present.
    pub fn replace_identifier(&mut self, id: Identifier) -> Option<Identifier> {
        self.identifier.replace(id)
    }
}
