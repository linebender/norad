//! Load only requested font data.

/// A type that describes which components of a UFO should be loaded.
///
/// By default, we load all components of the UFO file; however if you only
/// need some subset of these, you can pass this struct to [`Ufo::with_fields`]
/// in order to only load the fields specified in this object. This can help a
/// lot with performance with large UFO files if you don't need the glyph data.
///
/// [`Ufo::with_fields`]: struct.Ufo.html#method.with_fields
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DataRequest {
    pub layers: bool,
    pub lib: bool,
    pub groups: bool,
    pub kerning: bool,
    pub features: bool,
    pub data: bool,
    pub images: bool,
}

impl DataRequest {
    fn from_bool(b: bool) -> Self {
        DataRequest { layers: b, lib: b, groups: b, kerning: b, features: b, data: b, images: b }
    }

    /// Returns a `DataRequest` requesting all UFO data.
    pub fn all() -> Self {
        DataRequest::from_bool(true)
    }

    /// Returns a `DataRequest` requesting no UFO data.
    pub fn none() -> Self {
        DataRequest::from_bool(false)
    }

    /// Request that returned UFO data include the glyph layers and points.
    pub fn layers(&mut self, b: bool) -> &mut Self {
        self.layers = b;
        self
    }

    /// Request that returned UFO data include <lib> sections.
    pub fn lib(&mut self, b: bool) -> &mut Self {
        self.lib = b;
        self
    }

    /// Request that returned UFO data include parsed `groups.plist`.
    pub fn groups(&mut self, b: bool) -> &mut Self {
        self.groups = b;
        self
    }

    /// Request that returned UFO data include parsed `kerning.plist`.
    pub fn kerning(&mut self, b: bool) -> &mut Self {
        self.kerning = b;
        self
    }

    /// Request that returned UFO data include OpenType Layout features in Adobe
    /// .fea format.
    pub fn features(&mut self, b: bool) -> &mut Self {
        self.features = b;
        self
    }

    /// Request that returned UFO data include data.
    pub fn data(&mut self, b: bool) -> &mut Self {
        self.data = b;
        self
    }

    /// Request that returned UFO data include images.
    pub fn images(&mut self, b: bool) -> &mut Self {
        self.images = b;
        self
    }
}

impl Default for DataRequest {
    fn default() -> Self {
        DataRequest::from_bool(true)
    }
}
