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
#[derive(Debug)]
#[non_exhaustive]
pub struct DataRequest<'a> {
    // the layers to load.
    pub(crate) layers: LayerFilter<'a>,
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

type FilterFn<'a> = dyn Fn(&str, &Path) -> bool + 'a;

/// A type describing which layers to load.
pub(crate) struct LayerFilter<'a> {
    all: bool,
    load_default: bool,
    custom: Option<Box<FilterFn<'a>>>,
}

impl<'a> LayerFilter<'a> {
    fn from_bool(b: bool) -> Self {
        LayerFilter { all: b, ..Default::default() }
    }

    pub(crate) fn should_load(&self, name: &str, path: &Path) -> bool {
        self.all
            || (self.load_default && path == Path::new("glyphs"))
            || self.custom.as_ref().map(|f| f(name, path)).unwrap_or(false)
    }

    /// `true` if this filter includes the default layer
    pub(crate) fn includes_default_layer(&self) -> bool {
        self.all || self.load_default
    }
}

impl<'a> DataRequest<'a> {
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
    ///
    /// See also the [`filter_layers`] and [`default_layer`] options.
    ///
    /// [`filter_layers`]: Self::filter_layers
    /// [`default_layer`]: Self::default_layer
    pub fn layers(mut self, b: bool) -> Self {
        self.layers.all = b;
        self
    }

    /// Request to only load the default layer.
    ///
    /// If set, we will ignore the [`layers`] option. For finer-grained control,
    /// see the [`filter_layers`] option.
    ///
    /// [`filter_layers`]: Self::filter_layers
    /// [`layers`]: Self::layers
    pub fn default_layer(mut self, b: bool) -> Self {
        self.layers.load_default = b;
        self.layers.all = false;
        self
    }

    /// Request to load a subset of layers using a closure
    ///
    /// Given the name and directory of a layer, the closure must return `true`
    /// or `false`. Only layers for which the closure returns `true` will be loaded.
    ///
    /// If this is set, it will override the [`layers`] option.
    ///
    /// To only load the default layer, use the [`default_layer`] option.
    ///
    /// # Examples
    ///
    /// To only load the background layer:
    ///
    /// ```no_run
    /// # use norad::{DataRequest, Font};
    /// let to_load = DataRequest::none().filter_layers(|name, _path| name.contains("background"));
    /// let font = Font::load_requested_data("path/to/font.ufo", to_load).unwrap();
    /// ```
    ///
    /// [`default_layer`]: Self::default_layer
    /// [`layers`]: Self::layers
    pub fn filter_layers(mut self, filter: impl Fn(&str, &Path) -> bool + 'a) -> Self {
        self.layers.custom = Some(Box::new(filter));
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

impl Default for DataRequest<'_> {
    fn default() -> Self {
        DataRequest::from_bool(true)
    }
}

impl Default for LayerFilter<'_> {
    fn default() -> Self {
        Self { all: true, load_default: false, custom: None }
    }
}

impl std::fmt::Debug for LayerFilter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("LayerFilter")
            .field("all", &self.all)
            .field("load_default", &self.load_default)
            .field("custom", &self.custom.is_some())
            .finish()
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
