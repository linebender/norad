//! Load only requested font data.

use std::path::Path;

/// A type that describes which components of a UFO should be loaded.
///
/// By default, all components of the UFO file are loaded; however, if you only
/// need a subset of them, you can pass this struct to [`Ufo::with_fields`] in
/// order to only load the fields you specify. This can improve performance in
/// large projects.
///
/// # Examples
///
/// A [DataRequest] that excludes all layer, glyph and kerning data:
///
/// ```
/// use norad::DataRequest;
///
/// let datareq = DataRequest::default().layers(false).kerning(false);
/// ```
///
/// A [DataRequest] that excludes all UFO data and images:
///
/// ```
/// use norad::DataRequest;
///
/// let datareq = DataRequest::default().data(false).images(false);
/// ```
///
/// A [DataRequest] that only includes parsed lib.plist data:
///
/// ```
/// use norad::DataRequest;
///
/// let datareq = DataRequest::none().lib(true);
/// ```
///
/// [`Ufo::with_fields`]: struct.Ufo.html#method.with_fields
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DataRequest {
    // the layers to load.
    pub(crate) layers: LayerFilter,
    /// Load parsed lib.plist data
    pub lib: bool,
    /// Load parsed groups.plist data
    pub groups: bool,
    /// Load parsed kerning.plist data
    pub kerning: bool,
    /// Load Adobe .fea format feature file data
    pub features: bool,
    /// Load data
    pub data: bool,
    /// Load images
    pub images: bool,
}

/// A type describing which layers to load.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LayerFilter {
    all: bool,
    load_default: bool,
    named_layers: Vec<String>,
}

impl LayerFilter {
    fn from_bool(b: bool) -> Self {
        LayerFilter { all: b, ..Default::default() }
    }

    pub(crate) fn should_load(&self, name: &str, path: &Path) -> bool {
        self.all
            || (self.load_default && path == Path::new("glyphs"))
            || self.named_layers.iter().any(|req_name| name == req_name)
    }

    /// `true` if this filter includes the default layer
    pub(crate) fn includes_default_layer(&self) -> bool {
        self.all || self.load_default
    }
}

impl DataRequest {
    fn from_bool(b: bool) -> Self {
        DataRequest {
            layers: LayerFilter::from_bool(b),
            lib: b,
            groups: b,
            kerning: b,
            features: b,
            data: b,
            images: b,
        }
    }

    /// Returns a [`DataRequest`] requesting all UFO data.
    pub fn all() -> Self {
        DataRequest::from_bool(true)
    }

    /// Returns a [`DataRequest`] requesting no UFO data.
    pub fn none() -> Self {
        DataRequest::from_bool(false)
    }

    /// Request that returned UFO data include layers and their glyph data.
    pub fn layers(mut self, b: bool) -> Self {
        self.layers.all = b;
        self
    }

    /// Request to only load the default layer.
    ///
    /// If set, we will ignore the `layers` option.
    pub fn default_layer(mut self, b: bool) -> Self {
        self.layers.load_default = b;
        self.layers.all = false;
        self
    }

    /// Request to load the named layers.
    ///
    /// If set, we will ignore the `layers` option.
    pub fn named_layers(mut self, layers: &[&str]) -> Self {
        self.layers.named_layers = layers.iter().map(|s| s.to_string()).collect();
        self.layers.all = false;
        self
    }

    /// Request that returned UFO data include <lib> sections.
    pub fn lib(mut self, b: bool) -> Self {
        self.lib = b;
        self
    }

    /// Request that returned UFO data include parsed `groups.plist`.
    pub fn groups(mut self, b: bool) -> Self {
        self.groups = b;
        self
    }

    /// Request that returned UFO data include parsed `kerning.plist`.
    pub fn kerning(mut self, b: bool) -> Self {
        self.kerning = b;
        self
    }

    /// Request that returned UFO data include [OpenType Layout features in Adobe
    /// .fea format](https://unifiedfontobject.org/versions/ufo3/features.fea/).
    pub fn features(mut self, b: bool) -> Self {
        self.features = b;
        self
    }

    /// Request that returned UFO data include data.
    pub fn data(mut self, b: bool) -> Self {
        self.data = b;
        self
    }

    /// Request that returned UFO data include images.
    pub fn images(mut self, b: bool) -> Self {
        self.images = b;
        self
    }
}

impl Default for DataRequest {
    fn default() -> Self {
        DataRequest::from_bool(true)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn all_fields_are_true(dr: &DataRequest) -> bool {
        dr.layers.all && dr.lib && dr.groups && dr.kerning && dr.features && dr.data && dr.images
    }

    fn all_fields_are_false(dr: &DataRequest) -> bool {
        !dr.layers.all
            && !dr.lib
            && !dr.groups
            && !dr.kerning
            && !dr.features
            && !dr.data
            && !dr.images
    }

    #[test]
    fn test_datarequest_default() {
        assert!(all_fields_are_true(&DataRequest::default()));
    }

    #[test]
    fn test_datarequest_all() {
        assert!(all_fields_are_true(&DataRequest::all()));
    }

    #[test]
    fn test_datarequest_none() {
        assert!(all_fields_are_false(&DataRequest::none()));
    }

    #[test]
    fn test_datarequest_builder() {
        let dr = DataRequest::default()
            .layers(false)
            .lib(false)
            .groups(false)
            .kerning(false)
            .features(false)
            .data(false)
            .images(false);

        assert!(all_fields_are_false(&dr));
    }
}
