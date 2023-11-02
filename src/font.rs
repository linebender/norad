//! Reading and writing Unified Font Object files.

#![deny(rustdoc::broken_intra_doc_links)]

use std::fs;
use std::path::{Path, PathBuf};

use crate::data_request::LayerFilter;
use crate::datastore::{DataStore, ImageStore};
use crate::error::{FontLoadError, FontWriteError};
use crate::fontinfo::FontInfo;
use crate::glyph::Glyph;
use crate::groups::{validate_groups, Groups};
use crate::guideline::Guideline;
use crate::kerning::Kerning;
use crate::layer::{Layer, LayerContents, LAYER_CONTENTS_FILE};
use crate::name::Name;
use crate::names::NameList;
use crate::shared_types::{Plist, PUBLIC_OBJECT_LIBS_KEY};
use crate::upconversion;
use crate::write::{self, WriteOptions};
use crate::DataRequest;

static METAINFO_FILE: &str = "metainfo.plist";
static FONTINFO_FILE: &str = "fontinfo.plist";
pub(crate) static LIB_FILE: &str = "lib.plist";
static GROUPS_FILE: &str = "groups.plist";
static KERNING_FILE: &str = "kerning.plist";
static FEATURES_FILE: &str = "features.fea";
static DEFAULT_METAINFO_CREATOR: &str = "org.linebender.norad";
pub(crate) static DATA_DIR: &str = "data";
pub(crate) static IMAGES_DIR: &str = "images";

/// A font object, corresponding to a [UFO directory].
/// A Unified Font Object.
///
/// See the [UFO specification] for a description of the underlying data.
///
/// [UFO specification]: https://unifiedfontobject.org/versions/ufo3/
/// [UFO directory]: https://unifiedfontobject.org/versions/ufo3/index.html#directory-structure
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Font {
    /// The font's metainfo, corresponding to the [`metainfo.plist`][mi] file.
    ///
    /// [mi]: https://unifiedfontobject.org/versions/ufo3/metainfo.plist/
    pub meta: MetaInfo,
    /// The font info, corresponding to the [`fontinfo.plist`][fi] file.
    ///
    /// [fi]: https://unifiedfontobject.org/versions/ufo3/fontinfo.plist/
    pub font_info: FontInfo,
    ///  The font's layers.
    ///
    ///  Each layer contains some number of [`Glyph`]s, and corresponds to a
    ///  [glyph directory][] on disk.
    ///
    ///  [glyph directory]: https://unifiedfontobject.org/versions/ufo3/glyphs/
    pub layers: LayerContents,
    /// Arbitrary user-supplied data.
    ///
    /// This corresponds to the [`lib.plist`][l] file on disk. This file is
    /// optional; an empty lib will not be serialized.
    ///
    /// [l]: https://unifiedfontobject.org/versions/ufo3/lib.plist/
    pub lib: Plist,
    /// Glyph groups, corresponding to the [`groups.plist`][g] file.
    ///
    /// This file is optional; if no groups are specified it will not be serialized.
    ///
    /// [g]: https://unifiedfontobject.org/versions/ufo3/groups.plist/
    pub groups: Groups,
    /// Horizontal kerning pairs, corresponding to the [`kerning.plist`][k] file.
    ///
    /// This file is optional, and will not be serialized if no pairs are specified.
    ///
    /// [k]: https://unifiedfontobject.org/versions/ufo3/kerning.plist/
    pub kerning: Kerning,
    /// The contents of the [`features.fea`][fea] file, if one exists.
    ///
    /// [fea]: https://unifiedfontobject.org/versions/ufo3/features.fea/
    pub features: String,
    /// The contents of the font's [`data` directory][dir].
    ///
    /// [dir]: https://unifiedfontobject.org/versions/ufo3/data/
    pub data: DataStore,
    /// The contents of the font's [`images` directory][dir].
    ///
    /// [dir]: https://unifiedfontobject.org/versions/ufo3/images/
    pub images: ImageStore,
}

/// A version of the [UFO spec].
///
/// [UFO spec]: http://unifiedfontobject.org
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum FormatVersion {
    /// UFO specification major version 1. Only reading (and upconversion) is supported.
    V1 = 1,
    /// UFO specfication major version 2. Only reading (and upconversion) is supported.
    V2 = 2,
    /// UFO specification major version 3
    V3 = 3,
}

/// The contents of the [`metainfo.plist`] file.
///
/// [`metainfo.plist`]: http://unifiedfontobject.org/versions/ufo3/metainfo.plist/
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MetaInfo {
    /// Creator field
    pub creator: Option<String>,
    /// UFO specification major version field
    pub format_version: FormatVersion,
    /// UFO specification minor version field
    #[serde(default, skip_serializing_if = "is_zero")]
    pub format_version_minor: u32,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

impl Default for MetaInfo {
    fn default() -> Self {
        MetaInfo {
            creator: Some(DEFAULT_METAINFO_CREATOR.to_string()),
            format_version: FormatVersion::V3,
            format_version_minor: 0,
        }
    }
}

impl Font {
    /// Returns a new, empty [`Font`] object.
    pub fn new() -> Self {
        Font::default()
    }

    /// Returns a [`Font`] object with data from a UFO directory `path`.
    ///
    /// `path` must point to a directory with the structure described in
    /// [v3 of the Unified Font Object][v3] spec.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use norad::Font;
    ///
    /// let ufo = Font::load("path/to/font.ufo").expect("failed to load");
    /// ```
    ///
    /// Note: This will consume the `public.objectLibs` key in the global lib
    /// and in glyph libs and assign object libs found therein to global
    /// guidelines and glyph objects with the matching identifier, respectively.
    ///
    /// See [Font::load_requested_data] for a load method that supports customization
    /// of the data inclusion / exclusion criteria.
    ///
    /// [v3]: http://unifiedfontobject.org/versions/ufo3/
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Font, FontLoadError> {
        Self::load_requested_data(path, DataRequest::all())
    }

    /// Returns a [`Font`] object with custom data inclusion/exclusion
    /// criteria from a UFO directory `path`.
    ///
    /// UFO data inclusion and exclusion criteria are defined with a [`DataRequest`] parameter.
    ///
    /// # Examples
    ///
    /// A font object that excludes all layer, glyph and kerning data:
    ///
    /// ```no_run
    /// use norad::DataRequest;
    /// use norad::Font;
    ///
    /// let datareq = DataRequest::default().layers(false).kerning(false);
    ///
    /// let ufo = Font::load_requested_data("path/to/font.ufo", datareq).expect("failed to load");
    /// ```
    ///
    /// A font object that excludes all data and images:
    ///
    /// ```no_run
    /// use norad::DataRequest;
    /// use norad::Font;
    ///
    /// let datareq = DataRequest::default().data(false).images(false);
    ///
    /// let ufo = Font::load_requested_data("path/to/font.ufo", datareq).expect("failed to load");
    /// ```
    ///
    /// A font object that includes only parsed lib.plist data:
    ///
    /// ```no_run
    /// use norad::DataRequest;
    /// use norad::Font;
    ///
    /// let datareq = DataRequest::none().lib(true);
    ///
    /// let ufo = Font::load_requested_data("path/to/font.ufo", datareq).expect("failed to load");
    /// ```
    pub fn load_requested_data(
        path: impl AsRef<Path>,
        request: DataRequest,
    ) -> Result<Font, FontLoadError> {
        Self::load_impl(path.as_ref(), request)
    }

    fn load_impl(path: &Path, request: DataRequest) -> Result<Font, FontLoadError> {
        let metadata = path.metadata().map_err(FontLoadError::AccessUfoDir)?;
        if !metadata.is_dir() {
            return Err(FontLoadError::UfoNotADir);
        }

        let meta_path = path.join(METAINFO_FILE);
        if !meta_path.exists() {
            return Err(FontLoadError::MissingMetaInfoFile);
        }
        let mut meta: MetaInfo = plist::from_file(&meta_path)
            .map_err(|source| FontLoadError::ParsePlist { name: METAINFO_FILE, source })?;

        let lib_path = path.join(LIB_FILE);
        let mut lib =
            if request.lib && lib_path.exists() { load_lib(&lib_path)? } else { Plist::new() };

        let fontinfo_path = path.join(FONTINFO_FILE);
        let mut font_info = if fontinfo_path.exists() {
            load_fontinfo(&fontinfo_path, &meta, &mut lib)?
        } else {
            Default::default()
        };

        let groups_path = path.join(GROUPS_FILE);
        let groups = if request.groups && groups_path.exists() {
            Some(load_groups(&groups_path)?)
        } else {
            None
        };

        let kerning_path = path.join(KERNING_FILE);
        let kerning = if request.kerning && kerning_path.exists() {
            Some(load_kerning(&kerning_path)?)
        } else {
            None
        };

        let features_path = path.join(FEATURES_FILE);
        let mut features = if request.features && features_path.exists() {
            load_features(&features_path)?
        } else {
            Default::default()
        };

        let glyph_names = NameList::default();
        let layers = load_layer_set(path, &meta, &glyph_names, &request.layers)?;

        let data = if request.data && path.join(DATA_DIR).exists() {
            DataStore::new(path).map_err(FontLoadError::DataStore)?
        } else {
            Default::default()
        };

        let images = if request.images && path.join(IMAGES_DIR).exists() {
            ImageStore::new(path).map_err(FontLoadError::ImagesStore)?
        } else {
            Default::default()
        };

        // Upconvert UFO v1 or v2 kerning data if necessary. To upconvert, we need at least
        // a groups.plist file, while a kerning.plist is optional.
        let (groups, kerning) = match (meta.format_version, groups, kerning) {
            (FormatVersion::V3, g, k) => (g, k), // For v3, we do nothing.
            (_, None, k) => (None, k), // Without a groups.plist, there's nothing to upgrade.
            (_, Some(g), k) => {
                let (groups, kerning) =
                    upconversion::upconvert_kerning(&g, &k.unwrap_or_default(), &glyph_names);
                validate_groups(&groups).map_err(FontLoadError::GroupsUpconversionFailure)?;
                (Some(groups), Some(kerning))
            }
        };

        // The v1 format stores some Postscript hinting related data in the lib,
        // which we only import into fontinfo if we're reading a v1 UFO.
        if meta.format_version == FormatVersion::V1 && lib_path.exists() {
            if let Some(features_upgraded) =
                upconversion::upconvert_ufov1_robofab_data(&lib_path, &mut lib, &mut font_info)?
            {
                if !features_upgraded.is_empty() {
                    features = features_upgraded;
                }
            }
        }

        meta.format_version = FormatVersion::V3;

        Ok(Font {
            layers,
            meta,
            font_info,
            lib,
            groups: groups.unwrap_or_default(),
            kerning: kerning.unwrap_or_default(),
            features,
            data,
            images,
        })
    }

    /// Serialize a [`Font`] to the given `path`, overwriting any existing contents.
    ///
    /// # Examples
    ///
    /// With a [`Font`] object such as:
    ///
    /// ```no_run
    /// use norad::Font;
    ///
    /// let ufo = Font::load("path/to/in-font.ufo").expect("failed to load");
    /// # ufo.save("path/to/out-font.ufo").expect("failed to save");
    /// ```
    ///
    /// do things with the [`Font`], then serialize to disk with:
    ///
    /// ```no_run
    /// # use norad::Font;
    /// # let ufo = Font::load("path/to/in-font.ufo").expect("failed to load");
    /// ufo.save("path/to/out-font.ufo").expect("failed to save");
    /// ```
    ///
    /// Note: This may fail; instead of saving directly to the target path, it is a good
    /// idea to save to a temporary location and then move that to the target path
    /// if the save is successful.
    ///
    /// This _will_ fail if either the global or any glyph lib contains the
    /// `public.objectLibs` key, as object lib management must currently be done
    /// by norad.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), FontWriteError> {
        let path = path.as_ref();
        self.save_impl(path, &Default::default())
    }

    /// Serialize a [`Font`] to the given `path`, overwriting any existing contents,
    /// with custom [`WriteOptions`] serialization format settings.
    ///
    /// # Examples
    ///
    /// With a [`Font`] object:
    ///
    /// ```no_run
    /// use norad::{Font, QuoteChar, WriteOptions};
    ///
    /// let ufo = Font::load("path/to/in-font.ufo").expect("failed to load");
    /// ```
    ///
    /// define the serialization format with a [`WriteOptions`] type:
    ///
    /// ```no_run
    /// # use norad::{Font, QuoteChar, WriteOptions};
    /// # let ufo = Font::load("path/to/in-font.ufo").expect("failed to load");
    /// let single_tab = WriteOptions::default();
    ///
    /// let two_tabs = WriteOptions::default()
    ///     .whitespace("\t\t");
    ///
    /// let spaces = WriteOptions::default()
    ///     .whitespace("  ");
    ///
    /// let spaces_and_singlequotes = WriteOptions::default()
    ///     .whitespace("  ")
    ///     .quote_char(QuoteChar::Single);
    /// ```
    ///
    /// and serialize to disk with the respective [`WriteOptions`] configuration:
    ///
    /// ```no_run
    /// # use norad::{Font, QuoteChar, WriteOptions};
    /// # let ufo = Font::load("path/to/in-font.ufo").expect("failed to load");
    /// # let single_tab = WriteOptions::default();
    /// # let two_tabs = WriteOptions::default()
    /// #   .whitespace("\t\t");
    /// # let spaces = WriteOptions::default()
    /// #    .whitespace("  ");
    /// # let spaces_and_singlequotes = WriteOptions::default()
    /// #   .whitespace("  ")
    /// #   .quote_char(QuoteChar::Single);
    /// // with single tab indentation (default)
    /// ufo.save_with_options("path/to/out-font1.ufo", &single_tab);
    ///
    /// // with two tab indentation
    /// ufo.save_with_options("path/to/out-font2.ufo", &two_tabs);
    ///
    /// // with two space indentation
    /// ufo.save_with_options("path/to/out-font3.ufo", &spaces);
    ///
    /// // with two space indentation and single quote XML declarations
    /// ufo.save_with_options("path/to/out-font4.ufo", &spaces_and_singlequotes);
    /// ```
    ///
    /// Note: This may fail; It runs validation for groups, fontinfo and
    /// data/images stores before overwriting anything, but glyph validation may
    /// still fail later. Instead of saving directly to the target path, it is a
    /// good idea to save to a temporary location and then move that to the
    /// target path if the save is successful.
    ///
    /// This _will_ fail if either the global or any glyph lib contains the
    /// `public.objectLibs` key, as object lib management must currently be done
    /// by norad.
    pub fn save_with_options(
        &self,
        path: impl AsRef<Path>,
        options: &WriteOptions,
    ) -> Result<(), FontWriteError> {
        let path = path.as_ref();
        self.save_impl(path, options)
    }

    fn save_impl(&self, path: &Path, options: &WriteOptions) -> Result<(), FontWriteError> {
        if self.meta.format_version != FormatVersion::V3 {
            return Err(FontWriteError::Downgrade);
        }

        if self.lib.contains_key(PUBLIC_OBJECT_LIBS_KEY) {
            return Err(FontWriteError::PreexistingPublicObjectLibsKey);
        }

        // Run various validators before touching the file system.
        validate_groups(&self.groups).map_err(FontWriteError::InvalidGroups)?;
        self.font_info.validate().map_err(FontWriteError::InvalidFontInfo)?;

        // Load all data and images before potentially deleting them from disk.
        // Abandon ship if any of them is in an error state.
        for (path, entry) in self.data.iter().chain(self.images.iter()) {
            if let Err(source) = entry {
                return Err(FontWriteError::InvalidStoreEntry { path: path.clone(), source });
            };
        }

        // TODO: run glif validation up front?

        // Now do the actual writing.
        if path.exists() {
            fs::remove_dir_all(path).map_err(FontWriteError::Cleanup)?;
        }
        fs::create_dir(path).map_err(FontWriteError::CreateUfoDir)?;

        // we want to always set ourselves as the creator when serializing,
        // but we also don't have mutable access to self.
        let metainfo_path = path.join(METAINFO_FILE);
        if self.meta.creator == Some(DEFAULT_METAINFO_CREATOR.into()) {
            write::write_xml_to_file(&metainfo_path, &self.meta, options)
                .map_err(|source| FontWriteError::CustomFile { name: METAINFO_FILE, source })?;
        } else {
            write::write_xml_to_file(&metainfo_path, &MetaInfo::default(), options)
                .map_err(|source| FontWriteError::CustomFile { name: METAINFO_FILE, source })?;
        }

        if !self.font_info.is_empty() {
            write::write_xml_to_file(&path.join(FONTINFO_FILE), &self.font_info, options)
                .map_err(|source| FontWriteError::CustomFile { name: FONTINFO_FILE, source })?;
        }

        // Object libs are treated specially. The UFO v3 format won't allow us
        // to store them inline, so they have to be placed into the font's lib
        // under the public.objectLibs parent key. To avoid mutation behind the
        // client's back, object libs are written out but not stored in
        // font.lib in-memory. Instead we clone the lib, add the object libs, and
        // write out that.

        let mut lib = self.lib.clone();
        let font_object_libs = self.font_info.dump_object_libs();
        if !font_object_libs.is_empty() {
            lib.insert(PUBLIC_OBJECT_LIBS_KEY.into(), font_object_libs.into());
        }
        if !lib.is_empty() {
            crate::util::recursive_sort_plist_keys(&mut lib);
            write::write_xml_to_file(&path.join(LIB_FILE), &lib, options)
                .map_err(|source| FontWriteError::CustomFile { name: LIB_FILE, source })?;
        }

        if !self.groups.is_empty() {
            write::write_xml_to_file(&path.join(GROUPS_FILE), &self.groups, options)
                .map_err(|source| FontWriteError::CustomFile { name: GROUPS_FILE, source })?;
        }

        if !self.kerning.is_empty() {
            let kerning_serializer = crate::kerning::KerningSerializer { kerning: &self.kerning };
            write::write_xml_to_file(&path.join(KERNING_FILE), &kerning_serializer, options)
                .map_err(|source| FontWriteError::CustomFile { name: KERNING_FILE, source })?;
        }

        if !self.features.is_empty() {
            // Normalize feature files with line feed line endings
            // This is consistent with the line endings serialized in glif and plist files
            let feature_file_path = path.join(FEATURES_FILE);
            if self.features.as_bytes().contains(&b'\r') {
                close_already::fs::write(&feature_file_path, self.features.replace("\r\n", "\n"))
                    .map_err(FontWriteError::FeatureFile)?;
            } else {
                close_already::fs::write(&feature_file_path, &self.features)
                    .map_err(FontWriteError::FeatureFile)?;
            }
        }

        let contents: Vec<(&str, &PathBuf)> =
            self.layers.iter().map(|l| (l.name.as_ref(), &l.path)).collect();
        write::write_xml_to_file(&path.join(LAYER_CONTENTS_FILE), &contents, options)
            .map_err(|source| FontWriteError::CustomFile { name: LAYER_CONTENTS_FILE, source })?;

        for layer in self.layers.iter() {
            let layer_path = path.join(&layer.path);
            layer.save_with_options(&layer_path, options).map_err(|source| {
                FontWriteError::Layer {
                    name: layer.name.to_string(),
                    path: layer_path,
                    source: Box::new(source),
                }
            })?;
        }

        if !self.data.is_empty() {
            let data_dir = path.join(DATA_DIR);
            for (data_path, contents) in self.data.iter() {
                let data = contents.expect("internal error: should have been checked");
                let destination = data_dir.join(data_path);
                let destination_parent = destination.parent().unwrap();
                fs::create_dir_all(destination_parent).map_err(|source| {
                    FontWriteError::CreateStoreDir { path: destination_parent.into(), source }
                })?;
                close_already::fs::write(&destination, &*data)
                    .map_err(|source| FontWriteError::Data { path: destination, source })?;
            }
        }

        if !self.images.is_empty() {
            let images_dir = path.join(IMAGES_DIR);
            fs::create_dir(&images_dir) // Only a flat directory.
                .map_err(|source| FontWriteError::CreateStoreDir {
                    path: images_dir.clone(),
                    source,
                })?;
            for (image_path, contents) in self.images.iter() {
                let data = contents.expect("internal error: should have been checked");
                let destination = images_dir.join(image_path);
                close_already::fs::write(&destination, &*data)
                    .map_err(|source| FontWriteError::Image { path: destination, source })?;
            }
        }

        Ok(())
    }

    /// Returns a reference to the default layer.
    pub fn default_layer(&self) -> &Layer {
        self.layers.default_layer()
    }

    /// Returns a mutable reference to the default layer.
    pub fn default_layer_mut(&mut self) -> &mut Layer {
        self.layers.default_layer_mut()
    }

    /// Returns an iterator over all layers in this font object.
    pub fn iter_layers(&self) -> impl Iterator<Item = &Layer> {
        self.layers.iter()
    }

    /// Returns an iterator over all the glyph names _in the default layer_.
    pub fn iter_names(&self) -> impl Iterator<Item = Name> + '_ {
        //FIXME: why not &Name here?
        self.layers.default_layer().glyphs.keys().cloned()
    }

    /// Returns a reference to the glyph with the given name _in the default
    /// layer_.
    pub fn get_glyph(&self, key: &str) -> Option<&Glyph> {
        self.default_layer().get_glyph(key)
    }

    /// Returns a mutable reference to the glyph with the given name
    /// _in the default layer_, if it exists.
    pub fn get_glyph_mut(&mut self, key: &str) -> Option<&mut Glyph> {
        self.default_layer_mut().get_glyph_mut(key)
    }

    /// Returns the total number of glyphs _in the default layer_.
    pub fn glyph_count(&self) -> usize {
        self.default_layer().len()
    }

    /// Return the font's global guidelines, stored in [`FontInfo`].
    pub fn guidelines(&self) -> &[Guideline] {
        self.font_info.guidelines.as_deref().unwrap_or(&[])
    }

    /// Returns a mutable reference to the font's global guidelines.
    ///
    /// These will be created if they do not already exist.
    pub fn guidelines_mut(&mut self) -> &mut Vec<Guideline> {
        self.font_info.guidelines.get_or_insert_with(Default::default)
    }
}

fn load_lib(lib_path: &Path) -> Result<plist::Dictionary, FontLoadError> {
    plist::Value::from_file(lib_path)
        .map_err(|source| FontLoadError::ParsePlist { name: LIB_FILE, source })?
        .into_dictionary()
        .ok_or(FontLoadError::LibFileMustBeDictionary)
}

fn load_fontinfo(
    fontinfo_path: &Path,
    meta: &MetaInfo,
    lib: &mut plist::Dictionary,
) -> Result<FontInfo, FontLoadError> {
    let font_info: FontInfo = FontInfo::from_file(fontinfo_path, meta.format_version, lib)
        .map_err(FontLoadError::FontInfo)?;
    Ok(font_info)
}

fn load_groups(groups_path: &Path) -> Result<Groups, FontLoadError> {
    let groups: Groups = plist::from_file(groups_path)
        .map_err(|source| FontLoadError::ParsePlist { name: GROUPS_FILE, source })?;
    validate_groups(&groups).map_err(FontLoadError::InvalidGroups)?;
    Ok(groups)
}

fn load_kerning(kerning_path: &Path) -> Result<Kerning, FontLoadError> {
    let kerning: Kerning = plist::from_file(kerning_path)
        .map_err(|source| FontLoadError::ParsePlist { name: KERNING_FILE, source })?;
    Ok(kerning)
}

fn load_features(features_path: &Path) -> Result<String, FontLoadError> {
    let features = fs::read_to_string(features_path).map_err(FontLoadError::FeatureFile)?;
    Ok(features)
}

fn load_layer_set(
    ufo_path: &Path,
    meta: &MetaInfo,
    glyph_names: &NameList,
    filter: &LayerFilter,
) -> Result<LayerContents, FontLoadError> {
    let layercontents_path = ufo_path.join(LAYER_CONTENTS_FILE);
    if meta.format_version == FormatVersion::V3 && !layercontents_path.exists() {
        return Err(FontLoadError::MissingLayerContentsFile);
    }
    LayerContents::load(ufo_path, glyph_names, filter)
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use tempfile::TempDir;

    use crate::error::LayerLoadError;

    use super::*;

    #[test]
    fn new_is_v3() {
        let font = Font::new();
        assert_eq!(font.meta.format_version, FormatVersion::V3);
    }

    #[test]
    fn downgrade_unsupported() {
        let dir = TempDir::new().unwrap();

        let mut font = Font::new();
        font.meta.format_version = FormatVersion::V1;
        assert!(font.save(&dir).is_err());
        font.meta.format_version = FormatVersion::V2;
        assert!(font.save(&dir).is_err());
        font.meta.format_version = FormatVersion::V3;
        assert!(font.save(&dir).is_ok());
    }

    #[test]
    fn loading() {
        let path = "testdata/MutatorSansLightWide.ufo";
        let font_obj = Font::load(path).unwrap();
        assert_eq!(font_obj.iter_layers().count(), 2);
        font_obj.layers.get("background").expect("missing layer");

        assert_eq!(
            font_obj.lib.get("com.typemytype.robofont.compileSettings.autohint"),
            Some(&plist::Value::Boolean(true))
        );
        assert_eq!(font_obj.groups.get("public.kern1.@MMK_L_A"), Some(&vec![Name::new_raw("A")]));

        #[allow(clippy::float_cmp)]
        {
            assert_eq!(font_obj.kerning.get("B").and_then(|k| k.get("H")), Some(&-40.0));
        }

        assert_eq!(font_obj.features, "# this is the feature from lightWide\n");
    }

    #[test]
    fn load_save_feature_file_line_endings() {
        let font_obj = Font::load("testdata/lineendings/Tester-LineEndings.ufo").unwrap();
        let tmp = TempDir::new().unwrap();
        let ufopath = tmp.path().join("test.ufo");
        let feapath = ufopath.join("features.fea");
        font_obj.save(ufopath).unwrap();
        let test_fea = fs::read_to_string(feapath).unwrap();
        let expected_fea = String::from("feature ss01 {\n    featureNames {\n        name \"Bogus feature\";\n        name 1 \"Bogus feature\";\n    };\n    sub one by two;\n} ss01;\n");
        assert_eq!(test_fea, expected_fea);
    }

    #[test]
    fn loading_invalid_ufo_dir_path() {
        let path = "totally/bogus/filepath/font.ufo";
        let font_load_res = Font::load(path);
        assert!(matches!(font_load_res, Err(FontLoadError::AccessUfoDir(_))));
    }

    #[test]
    fn loading_missing_metainfo_plist_path() {
        // This UFO source does not have a metainfo.plist file
        // This should raise an error
        let path = "testdata/ufo/Tester-MissingMetaInfo.ufo";
        let font_load_res = Font::load(path);
        assert!(matches!(font_load_res, Err(FontLoadError::MissingMetaInfoFile)));
    }

    #[test]
    fn loading_missing_layercontents_plist_path() {
        // This UFO source does not have a layercontents.plist file
        // This should raise an error
        let path = "testdata/ufo/Tester-MissingLayerContents.ufo";
        let font_load_res = Font::load(path);
        assert!(matches!(font_load_res, Err(FontLoadError::MissingLayerContentsFile)));
    }

    #[test]
    fn loading_missing_glyphs_contents_plist_path() {
        // This UFO source does not have contents.plist in the default glyphs
        // directory. This should raise an error
        let path = "testdata/ufo/Tester-MissingGlyphsContents.ufo";
        let font_load_res = Font::load(path);
        let Err(FontLoadError::Layer { source, .. }) = font_load_res else {
            panic!("expected FontLoadError, found '{:?}'", font_load_res);
        };
        if !matches!(source.deref(), LayerLoadError::MissingContentsFile) {
            panic!("expected MissingContentsFile, found '{:?}'", source);
        }
    }

    #[test]
    fn loading_missing_glyphs_contents_plist_path_background_layer() {
        // This UFO source has a contents.plist in the default glyphs directory
        // but not in the glyphs.background directory. This should raise an error
        let path = "testdata/ufo/Tester-MissingGlyphsContents-BackgroundLayer.ufo";
        let font_load_res = Font::load(path);
        let Err(FontLoadError::Layer { source, .. }) = font_load_res else {
            panic!("expected FontLoadError, found '{:?}'", font_load_res);
        };
        if !matches!(source.deref(), LayerLoadError::MissingContentsFile) {
            panic!("expected MissingContentsFile, found '{:?}'", source);
        }
    }

    #[test]
    fn data_request() {
        let path = "testdata/MutatorSansLightWide.ufo";
        let font_obj = Font::load_requested_data(path, DataRequest::none()).unwrap();
        assert_eq!(font_obj.iter_layers().count(), 1);
        assert!(font_obj.layers.default_layer().is_empty());
        assert_eq!(font_obj.lib, Plist::new());
        assert!(font_obj.groups.is_empty());
        assert!(font_obj.kerning.is_empty());
        assert!(font_obj.features.is_empty());
    }

    #[test]
    fn upconvert_ufov1_robofab_data() {
        let path = "testdata/fontinfotest_v1.ufo";
        let font = Font::load(path).unwrap();

        assert_eq!(font.meta.format_version, FormatVersion::V3);

        let font_info = font.font_info;
        assert_eq!(font_info.postscript_blue_fuzz, Some(1.));
        assert_eq!(font_info.postscript_blue_scale, Some(0.039625));
        assert_eq!(font_info.postscript_blue_shift, Some(7.));
        assert_eq!(
            font_info.postscript_blue_values,
            Some(vec![-10., 0., 482., 492., 694., 704., 739., 749.])
        );
        assert_eq!(font_info.postscript_other_blues, Some(vec![-260., -250.]));
        assert_eq!(font_info.postscript_family_blues, Some(vec![500.0, 510.0]));
        assert_eq!(font_info.postscript_family_other_blues, Some(vec![-260., -250.]));
        assert_eq!(font_info.postscript_force_bold, Some(true));
        assert_eq!(font_info.postscript_stem_snap_h, Some(vec![100., 120.]));
        assert_eq!(font_info.postscript_stem_snap_v, Some(vec![80., 90.]));

        assert_eq!(font.lib.keys().collect::<Vec<&String>>(), vec!["org.robofab.testFontLibData"]);

        assert_eq!(
            font.features,
            "@myClass = [A B];\n\nfeature liga {\n    sub A A by b;\n} liga;\n"
        );
    }

    #[test]
    fn upconversion_fontinfo_v123() {
        let ufo_v1 = Font::load("testdata/fontinfotest_v1.ufo").unwrap();
        let ufo_v2 = Font::load("testdata/fontinfotest_v2.ufo").unwrap();
        let ufo_v3 = Font::load("testdata/fontinfotest_v3.ufo").unwrap();

        assert_eq!(ufo_v1, ufo_v3);
        assert_eq!(ufo_v2, ufo_v3);
    }

    #[test]
    fn metainfo() {
        let path = "testdata/MutatorSansLightWide.ufo/metainfo.plist";
        let meta: MetaInfo = plist::from_file(path).expect("failed to load metainfo");
        assert_eq!(meta.creator, Some("org.robofab.ufoLib".into()));
    }

    #[test]
    fn serialize_metainfo() {
        use serde_test::{assert_ser_tokens, Token};

        let meta1 = MetaInfo::default();
        assert_ser_tokens(
            &meta1,
            &[
                Token::Struct { name: "MetaInfo", len: 2 },
                Token::Str("creator"),
                Token::Some,
                Token::Str(DEFAULT_METAINFO_CREATOR),
                Token::Str("formatVersion"),
                Token::U8(3),
                Token::StructEnd,
            ],
        );

        let meta2 = MetaInfo { format_version_minor: 123, ..Default::default() };
        assert_ser_tokens(
            &meta2,
            &[
                Token::Struct { name: "MetaInfo", len: 3 },
                Token::Str("creator"),
                Token::Some,
                Token::Str(DEFAULT_METAINFO_CREATOR),
                Token::Str("formatVersion"),
                Token::U8(3),
                Token::Str("formatVersionMinor"),
                Token::U32(123),
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn save_with_options_with_writeoptions_parameter() {
        let opt = WriteOptions::default();
        let ufo = Font::default();
        let tmp = TempDir::new().unwrap();
        ufo.save_with_options(tmp, &opt).unwrap()
    }
}
