//! Loading a [`Font`] from a [`FontSource`] (non-filesystem).

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::data_request::LayerFilter;
use crate::error::{FontLoadError, LayerLoadError};
use crate::font::{
    Font, FormatVersion, MetaInfo, FEATURES_FILE, FONTINFO_FILE, GROUPS_FILE, KERNING_FILE,
    LIB_FILE, METAINFO_FILE,
};
use crate::font_source::FontSource;
use crate::fontinfo::FontInfo;
use crate::glyph::Glyph;
use crate::groups::validate_groups;
use crate::kerning::Kerning;
use crate::layer::{Layer, LayerContents, DEFAULT_GLYPHS_DIRNAME, DEFAULT_LAYER_NAME,
                   LAYER_CONTENTS_FILE};
use crate::name::Name;
use crate::shared_types::Plist;
use crate::DataRequest;

pub(crate) fn load_font(
    source: &impl FontSource,
    request: DataRequest,
) -> Result<Font, FontLoadError> {
    // --- metainfo.plist (required) ---
    let meta_str = read_required(source, Path::new(METAINFO_FILE))?;
    let meta: MetaInfo = plist::from_reader(Cursor::new(meta_str.as_bytes()))
        .map_err(|source| FontLoadError::ParsePlist { name: METAINFO_FILE, source })?;

    if meta.format_version != FormatVersion::V3 {
        return Err(FontLoadError::SourceUnsupportedFormatVersion);
    }

    // --- lib.plist (optional) ---
    let mut lib = if request.lib {
        match read_optional(source, Path::new(LIB_FILE))? {
            Some(s) => plist::Value::from_reader_xml(Cursor::new(s.as_bytes()))
                .map_err(|source| FontLoadError::ParsePlist { name: LIB_FILE, source })?
                .into_dictionary()
                .ok_or(FontLoadError::LibFileMustBeDictionary)?,
            None => Plist::new(),
        }
    } else {
        Plist::new()
    };

    // --- fontinfo.plist (optional) ---
    let font_info = match read_optional(source, Path::new(FONTINFO_FILE))? {
        Some(s) => {
            FontInfo::from_reader(Cursor::new(s.as_bytes()), &mut lib)
                .map_err(FontLoadError::FontInfo)?
        }
        None => Default::default(),
    };

    // --- groups.plist (optional) ---
    let groups = if request.groups {
        match read_optional(source, Path::new(GROUPS_FILE))? {
            Some(s) => {
                let g = crate::groups::deserialize_groups_from_reader(Cursor::new(s.as_bytes()))?;
                validate_groups(&g).map_err(FontLoadError::InvalidGroups)?;
                Some(g)
            }
            None => None,
        }
    } else {
        None
    };

    // --- kerning.plist (optional) ---
    let kerning = if request.kerning {
        match read_optional(source, Path::new(KERNING_FILE))? {
            Some(s) => {
                let k: Kerning = plist::from_reader(Cursor::new(s.as_bytes()))
                    .map_err(|source| FontLoadError::ParsePlist { name: KERNING_FILE, source })?;
                Some(k)
            }
            None => None,
        }
    } else {
        None
    };

    // --- features.fea (optional) ---
    let features = if request.features {
        read_optional(source, Path::new(FEATURES_FILE))?.unwrap_or_default()
    } else {
        String::new()
    };

    // --- layers ---
    let layers = load_layer_set(source, &request.layers)?;

    // data/ and images/ are not loaded via source; they require a filesystem path.
    Ok(Font {
        layers,
        meta,
        font_info,
        lib,
        groups: groups.unwrap_or_default(),
        kerning: kerning.unwrap_or_default(),
        features,
        data: Default::default(),
        images: Default::default(),
    })
}

fn load_layer_set(
    source: &impl FontSource,
    filter: &LayerFilter,
) -> Result<LayerContents, FontLoadError> {
    let to_load: Vec<(Name, PathBuf)> = match read_optional(source, Path::new(LAYER_CONTENTS_FILE))?
    {
        Some(s) => plist::from_reader(Cursor::new(s.as_bytes()))
            .map_err(|source| FontLoadError::ParsePlist { name: LAYER_CONTENTS_FILE, source })?,
        None => vec![(Name::new_raw(DEFAULT_LAYER_NAME), PathBuf::from(DEFAULT_GLYPHS_DIRNAME))],
    };

    let mut layers: Vec<_> = to_load
        .into_iter()
        .filter(|(name, path)| filter.should_load(name, path))
        .map(|(name, rel_path)| {
            load_layer(source, &rel_path, name.clone()).map_err(|source| FontLoadError::Layer {
                name: name.to_string(),
                path: rel_path,
                source: Box::new(source),
            })
        })
        .collect::<Result<_, _>>()?;

    if !filter.includes_default_layer() {
        layers.push(Layer::default());
    }

    let default_idx = layers
        .iter()
        .position(|l| l.path() == Path::new(DEFAULT_GLYPHS_DIRNAME))
        .ok_or(FontLoadError::MissingDefaultLayer)?;
    layers.rotate_left(default_idx);

    Ok(LayerContents::from_layers(layers))
}

fn load_layer(
    source: &impl FontSource,
    layer_dir: &Path,
    name: Name,
) -> Result<Layer, LayerLoadError> {
    // contents.plist (required for each layer)
    let contents_path = layer_dir.join("contents.plist");
    let contents_str = source
        .read_contents(&contents_path)
        .map_err(|e| LayerLoadError::Source {
            path: contents_path.clone(),
            source: Box::new(e),
        })?
        .ok_or(LayerLoadError::MissingContentsFile)?;

    let contents: BTreeMap<Name, PathBuf> = plist::from_reader(Cursor::new(contents_str.as_bytes()))
        .map_err(|source| LayerLoadError::ParsePlist { name: "contents.plist", source })?;
    let path_set = contents.values().map(|p| p.to_string_lossy().to_lowercase()).collect();

    // glyphs
    let glyphs: BTreeMap<Name, Glyph> = contents
        .iter()
        .map(|(glyph_name, glif_rel_path)| {
            let glif_path = layer_dir.join(glif_rel_path);
            let glif_str = source
                .read_contents(&glif_path)
                .map_err(|e| LayerLoadError::Source {
                    path: glif_path.clone(),
                    source: Box::new(e),
                })?
                .ok_or_else(|| LayerLoadError::Glyph {
                    name: glyph_name.to_string(),
                    path: glif_path.clone(),
                    source: crate::error::GlifLoadError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("glyph file not found: {}", glif_path.display()),
                    )),
                })?;

            Glyph::parse_raw(glif_str.as_bytes())
                .map_err(|source| LayerLoadError::Glyph {
                    name: glyph_name.to_string(),
                    path: glif_path,
                    source,
                })
                .map(|mut glyph| {
                    glyph.name = glyph_name.clone();
                    (glyph_name.clone(), glyph)
                })
        })
        .collect::<Result<_, _>>()?;

    // layerinfo.plist (optional)
    let layerinfo_path = layer_dir.join("layerinfo.plist");
    let (color, lib) = match source.read_contents(&layerinfo_path).ok().flatten() {
        Some(s) => Layer::parse_layer_info_from_reader(Cursor::new(s.as_bytes()))?,
        None => (None, Plist::new()),
    };

    let path = layer_dir.file_name().unwrap_or(layer_dir.as_os_str()).into();
    Ok(Layer::new_with_data(name, path, glyphs, contents, path_set, color, lib))
}

// --- helpers ---

fn read_required(source: &impl FontSource, path: &Path) -> Result<String, FontLoadError> {
    source
        .read_contents(path)
        .map_err(|e| FontLoadError::Source { path: path.into(), source: Box::new(e) })?
        .ok_or_else(|| FontLoadError::MissingFile { path: path.into() })
}

fn read_optional(
    source: &impl FontSource,
    path: &Path,
) -> Result<Option<String>, FontLoadError> {
    source
        .read_contents(path)
        .map_err(|e| FontLoadError::Source { path: path.into(), source: Box::new(e) })
}
