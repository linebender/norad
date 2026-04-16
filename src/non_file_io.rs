use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};
use std::error::Error as StdError;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::error::{FontLoadError, FontWriteError, GlifLoadError, LayerLoadError};
use crate::font::{
    Font, FormatVersion, MetaInfo, DATA_DIR, DEFAULT_METAINFO_CREATOR, FEATURES_FILE,
    FONTINFO_FILE, GROUPS_FILE, IMAGES_DIR, KERNING_FILE, LIB_FILE, METAINFO_FILE,
};
use crate::fontinfo::FontInfo;
use crate::glyph::Glyph;
use crate::groups::{validate_groups, Groups};
use crate::kerning::Kerning;
use crate::layer::LAYER_CONTENTS_FILE;
use crate::shared_types::{Plist, PUBLIC_OBJECT_LIBS_KEY};
use crate::write::{self, WriteOptions};
use crate::{DataRequest, LayerContents, Name};

type FeatureFilesLoadResult = Option<(String, BTreeMap<PathBuf, String>)>;

pub(crate) fn load_font_with_source<F, E>(
    request: DataRequest,
    source: &mut F,
) -> Result<Font, FontLoadError>
where
    F: FnMut(&Path) -> Result<Option<String>, E>,
    E: StdError + Send + Sync + 'static,
{
    let metainfo_str = read_required_text(source, Path::new(METAINFO_FILE), || {
        FontLoadError::MissingMetaInfoFile
    })?;
    let mut meta: MetaInfo = plist::from_reader_xml(metainfo_str.as_bytes())
        .map_err(|source| FontLoadError::ParsePlist { name: METAINFO_FILE, source })?;

    if meta.format_version != FormatVersion::V3 {
        return Err(FontLoadError::SourceUnsupportedFormatVersion);
    }

    let mut lib = if request.lib {
        match read_optional_text(source, Path::new(LIB_FILE))? {
            Some(lib_str) => plist::Value::from_reader_xml(lib_str.as_bytes())
                .map_err(|source| FontLoadError::ParsePlist { name: LIB_FILE, source })?
                .into_dictionary()
                .ok_or(FontLoadError::LibFileMustBeDictionary)?,
            None => Plist::new(),
        }
    } else {
        Plist::new()
    };

    let font_info =
        if let Some(fontinfo_str) = read_optional_text(source, Path::new(FONTINFO_FILE))? {
            let mut font_info: FontInfo = plist::from_reader_xml(fontinfo_str.as_bytes())
                .map_err(|source| FontLoadError::ParsePlist { name: FONTINFO_FILE, source })?;
            font_info
                .validate()
                .map_err(crate::error::FontInfoLoadError::InvalidData)
                .map_err(FontLoadError::FontInfo)?;
            font_info.load_object_libs(&mut lib).map_err(FontLoadError::FontInfo)?;
            font_info
        } else {
            Default::default()
        };

    let groups = if request.groups {
        match read_optional_text(source, Path::new(GROUPS_FILE))? {
            Some(groups_str) => {
                let groups: Groups = plist::from_reader_xml(groups_str.as_bytes())
                    .map_err(|source| FontLoadError::ParsePlist { name: GROUPS_FILE, source })?;
                validate_groups(&groups).map_err(FontLoadError::InvalidGroups)?;
                Some(groups)
            }
            None => None,
        }
    } else {
        None
    };

    let kerning = if request.kerning {
        match read_optional_text(source, Path::new(KERNING_FILE))? {
            Some(kerning_str) => {
                let kerning: Kerning = plist::from_reader_xml(kerning_str.as_bytes())
                    .map_err(|source| FontLoadError::ParsePlist { name: KERNING_FILE, source })?;
                Some(kerning)
            }
            None => None,
        }
    } else {
        None
    };

    let (features, feature_files) = if request.features {
        load_feature_files(&mut |path| read_optional_text(source, path), Path::new(FEATURES_FILE))?
            .unwrap_or_else(|| (String::new(), BTreeMap::new()))
    } else {
        (String::new(), BTreeMap::new())
    };

    let layers = load_layer_set_from_source(source, &request.layers)?;

    meta.format_version = FormatVersion::V3;

    Ok(Font {
        layers,
        meta,
        font_info,
        lib,
        groups: groups.unwrap_or_default(),
        kerning: kerning.unwrap_or_default(),
        features,
        feature_files,
        data: Default::default(),
        images: Default::default(),
    })
}

pub(crate) fn save_font_with_sink<F, E>(
    font: &Font,
    options: &WriteOptions,
    sink: &mut F,
) -> Result<(), FontWriteError>
where
    F: FnMut(&Path, &[u8]) -> Result<(), E>,
    E: StdError + Send + Sync + 'static,
{
    if font.meta.format_version != FormatVersion::V3 {
        return Err(FontWriteError::Downgrade);
    }

    if font.lib.contains_key(PUBLIC_OBJECT_LIBS_KEY) {
        return Err(FontWriteError::PreexistingPublicObjectLibsKey);
    }

    validate_groups(&font.groups).map_err(FontWriteError::InvalidGroups)?;
    font.font_info.validate().map_err(FontWriteError::InvalidFontInfo)?;

    for (path, entry) in font.data.iter().chain(font.images.iter()) {
        if let Err(source) = entry {
            return Err(FontWriteError::InvalidStoreEntry { path: path.clone(), source });
        }
    }

    let metainfo_value = if font.meta.creator == Some(DEFAULT_METAINFO_CREATOR.into()) {
        font.meta.clone()
    } else {
        MetaInfo::default()
    };
    write_sink_file(
        sink,
        Path::new(METAINFO_FILE),
        &write::write_xml_to_bytes(&metainfo_value, options)
            .map_err(|source| FontWriteError::CustomFile { name: METAINFO_FILE, source })?,
    )?;

    if !font.font_info.is_empty() {
        write_sink_file(
            sink,
            Path::new(FONTINFO_FILE),
            &write::write_xml_to_bytes(&font.font_info, options)
                .map_err(|source| FontWriteError::CustomFile { name: FONTINFO_FILE, source })?,
        )?;
    }

    let mut lib = font.lib.clone();
    let font_object_libs = font.font_info.dump_object_libs();
    if !font_object_libs.is_empty() {
        lib.insert(PUBLIC_OBJECT_LIBS_KEY.into(), font_object_libs.into());
    }
    if !lib.is_empty() {
        crate::util::recursive_sort_plist_keys(&mut lib);
        write_sink_file(
            sink,
            Path::new(LIB_FILE),
            &write::write_xml_to_bytes(&lib, options)
                .map_err(|source| FontWriteError::CustomFile { name: LIB_FILE, source })?,
        )?;
    }

    if !font.groups.is_empty() {
        write_sink_file(
            sink,
            Path::new(GROUPS_FILE),
            &write::write_xml_to_bytes(&font.groups, options)
                .map_err(|source| FontWriteError::CustomFile { name: GROUPS_FILE, source })?,
        )?;
    }

    if !font.kerning.is_empty() {
        let kerning_serializer = crate::kerning::KerningSerializer { kerning: &font.kerning };
        write_sink_file(
            sink,
            Path::new(KERNING_FILE),
            &write::write_xml_to_bytes(&kerning_serializer, options)
                .map_err(|source| FontWriteError::CustomFile { name: KERNING_FILE, source })?,
        )?;
    }

    if !font.features.is_empty() || !font.feature_files.is_empty() {
        write_sink_file(
            sink,
            Path::new(FEATURES_FILE),
            normalize_feature_text(&font.features).as_bytes(),
        )?;
        for (feature_path, contents) in &font.feature_files {
            write_sink_file(sink, feature_path, normalize_feature_text(contents).as_bytes())?;
        }
    }

    let contents: Vec<(&str, &PathBuf)> =
        font.layers.iter().map(|layer| (layer.name().as_ref(), &layer.path)).collect();
    write_sink_file(
        sink,
        Path::new(LAYER_CONTENTS_FILE),
        &write::write_xml_to_bytes(&contents, options)
            .map_err(|source| FontWriteError::CustomFile { name: LAYER_CONTENTS_FILE, source })?,
    )?;

    for layer in font.layers.iter() {
        let layer_path = Path::new(layer.path());
        layer.save_with_sink(layer_path, options, sink).map_err(|source| {
            FontWriteError::Layer {
                name: layer.name().to_string(),
                path: layer_path.to_path_buf(),
                source: Box::new(source),
            }
        })?;
    }

    if !font.data.is_empty() {
        for (data_path, contents) in font.data.iter() {
            let data = contents.expect("internal error: should have been checked");
            write_sink_file(sink, &Path::new(DATA_DIR).join(data_path), &data[..])?;
        }
    }

    if !font.images.is_empty() {
        for (image_path, contents) in font.images.iter() {
            let data = contents.expect("internal error: should have been checked");
            write_sink_file(sink, &Path::new(IMAGES_DIR).join(image_path), &data[..])?;
        }
    }

    Ok(())
}

fn read_optional_text<F, E>(source: &mut F, path: &Path) -> Result<Option<String>, FontLoadError>
where
    F: FnMut(&Path) -> Result<Option<String>, E>,
    E: StdError + Send + Sync + 'static,
{
    source(path).map_err(|source| FontLoadError::Source {
        path: path.to_path_buf(),
        source: Box::new(source),
    })
}

fn read_required_text<F, E, M>(
    source: &mut F,
    path: &Path,
    missing_error: M,
) -> Result<String, FontLoadError>
where
    F: FnMut(&Path) -> Result<Option<String>, E>,
    M: FnOnce() -> FontLoadError,
    E: StdError + Send + Sync + 'static,
{
    read_optional_text(source, path)?.ok_or_else(missing_error)
}

fn load_layer_set_from_source<F, E>(
    source: &mut F,
    filter: &crate::data_request::LayerFilter,
) -> Result<LayerContents, FontLoadError>
where
    F: FnMut(&Path) -> Result<Option<String>, E>,
    E: StdError + Send + Sync + 'static,
{
    let layercontents_str = read_required_text(source, Path::new(LAYER_CONTENTS_FILE), || {
        FontLoadError::MissingLayerContentsFile
    })?;
    let layer_descriptors: Vec<(Name, PathBuf)> =
        plist::from_reader_xml(layercontents_str.as_bytes())
            .map_err(|source| FontLoadError::ParsePlist { name: LAYER_CONTENTS_FILE, source })?;

    let mut layers = LayerContents::default();
    for (layer_name, layer_dir) in layer_descriptors {
        if !filter.should_load(&layer_name, &layer_dir) {
            continue;
        }

        let layer = if layer_dir == Path::new("glyphs") {
            layers.default_layer_mut()
        } else {
            let layer = layers.get_or_create_layer(layer_name.as_str()).map_err(|_| {
                FontLoadError::Layer {
                    name: layer_name.to_string(),
                    path: layer_dir.clone(),
                    source: Box::new(LayerLoadError::MissingContentsFile),
                }
            })?;
            layer.path = layer_dir.clone();
            layer
        };

        let contents_path = layer_dir.join("contents.plist");
        let contents_str =
            read_optional_text(source, &contents_path)?.ok_or(FontLoadError::Layer {
                name: layer_name.to_string(),
                path: contents_path.clone(),
                source: Box::new(LayerLoadError::MissingContentsFile),
            })?;

        let glyph_files: BTreeMap<Name, PathBuf> = plist::from_reader_xml(contents_str.as_bytes())
            .map_err(|source| FontLoadError::Layer {
                name: layer_name.to_string(),
                path: contents_path.clone(),
                source: Box::new(LayerLoadError::ParsePlist { name: "contents.plist", source }),
            })?;

        for (_glyph_name, glif_relative_path) in glyph_files {
            let glif_path = layer_dir.join(&glif_relative_path);
            let glif_contents =
                read_optional_text(source, &glif_path)?.ok_or_else(|| FontLoadError::Layer {
                    name: layer_name.to_string(),
                    path: glif_path.clone(),
                    source: Box::new(LayerLoadError::Glyph {
                        name: glif_relative_path.to_string_lossy().to_string(),
                        path: glif_path.clone(),
                        source: GlifLoadError::Io(io::Error::from(io::ErrorKind::NotFound)),
                    }),
                })?;
            let mut glyph = Glyph::parse_raw(glif_contents.as_bytes()).map_err(|source| {
                FontLoadError::Layer {
                    name: layer_name.to_string(),
                    path: glif_path.clone(),
                    source: Box::new(LayerLoadError::Glyph {
                        name: glif_relative_path.to_string_lossy().to_string(),
                        path: glif_path.clone(),
                        source,
                    }),
                }
            })?;
            glyph.name = Name::new_raw(&glyph.name);
            layer.insert_glyph(glyph);
        }
    }

    Ok(layers)
}

pub(crate) fn load_feature_files(
    source: &mut impl FnMut(&Path) -> Result<Option<String>, FontLoadError>,
    path: &Path,
) -> Result<FeatureFilesLoadResult, FontLoadError> {
    let Some(contents) = source(path)? else {
        return Ok(None);
    };

    let normalized_path = normalize_relative_path(path);
    let mut stack = HashSet::new();
    let mut included_files = BTreeMap::new();
    stack.insert(normalized_path.clone());
    collect_feature_includes(source, &normalized_path, &contents, &mut stack, &mut included_files)?;
    Ok(Some((contents, included_files)))
}

fn collect_feature_includes(
    source: &mut impl FnMut(&Path) -> Result<Option<String>, FontLoadError>,
    current_path: &Path,
    contents: &str,
    stack: &mut HashSet<PathBuf>,
    included_files: &mut BTreeMap<PathBuf, String>,
) -> Result<(), FontLoadError> {
    for line in contents.split_inclusive('\n') {
        if let Some(include_target) = parse_include_target(line) {
            let include_path =
                join_virtual_path(current_path.parent().unwrap_or(Path::new("")), &include_target);
            if stack.contains(&include_path) {
                return Err(FontLoadError::FeatureIncludeCycle { path: include_path });
            }
            if included_files.contains_key(&include_path) {
                continue;
            }

            let include_contents = source(&include_path)?.ok_or_else(|| {
                FontLoadError::MissingIncludedFeatureFile { path: include_path.clone() }
            })?;
            stack.insert(include_path.clone());
            collect_feature_includes(
                source,
                &include_path,
                &include_contents,
                stack,
                included_files,
            )?;
            stack.remove(&include_path);
            included_files.insert(include_path, include_contents);
        }
    }

    Ok(())
}

pub(crate) fn expand_feature_text(
    features: &str,
    feature_files: &BTreeMap<PathBuf, String>,
) -> Result<String, FontLoadError> {
    let mut stack = HashSet::new();
    let root_path = normalize_relative_path(Path::new(FEATURES_FILE));
    stack.insert(root_path.clone());
    expand_feature_text_from_map(&root_path, features, feature_files, &mut stack)
}

fn expand_feature_text_from_map(
    current_path: &Path,
    contents: &str,
    feature_files: &BTreeMap<PathBuf, String>,
    stack: &mut HashSet<PathBuf>,
) -> Result<String, FontLoadError> {
    let mut out = String::new();

    for line in contents.split_inclusive('\n') {
        if let Some(include_target) = parse_include_target(line) {
            let include_path =
                join_virtual_path(current_path.parent().unwrap_or(Path::new("")), &include_target);
            if !stack.insert(include_path.clone()) {
                return Err(FontLoadError::FeatureIncludeCycle { path: include_path });
            }
            let include_contents = feature_files.get(&include_path).ok_or_else(|| {
                FontLoadError::MissingIncludedFeatureFile { path: include_path.clone() }
            })?;
            out.push_str(&expand_feature_text_from_map(
                &include_path,
                include_contents,
                feature_files,
                stack,
            )?);
            stack.remove(&include_path);
        } else {
            out.push_str(line);
        }
    }

    Ok(out)
}

fn parse_include_target(line: &str) -> Option<PathBuf> {
    let trimmed = line.trim();
    if !trimmed.starts_with("include") || !trimmed.ends_with(';') {
        return None;
    }

    let open = trimmed.find('(')?;
    let close = trimmed.rfind(')')?;
    if close < open {
        return None;
    }

    let inner = trimmed[open + 1..close].trim();
    if inner.is_empty() {
        return None;
    }

    let inner = inner.trim_matches(|c| c == '"' || c == '\'');
    if inner.is_empty() {
        return None;
    }

    Some(normalize_relative_path(Path::new(inner)))
}

fn join_virtual_path(base: &Path, relative: &Path) -> PathBuf {
    if relative.is_absolute() {
        return normalize_relative_path(relative);
    }
    normalize_relative_path(&base.join(relative))
}

fn normalize_relative_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn write_sink_file<F, E>(sink: &mut F, path: &Path, bytes: &[u8]) -> Result<(), FontWriteError>
where
    F: FnMut(&Path, &[u8]) -> Result<(), E>,
    E: StdError + Send + Sync + 'static,
{
    sink(path, bytes).map_err(|source| FontWriteError::Sink {
        path: path.to_path_buf(),
        source: Box::new(source),
    })
}

fn normalize_feature_text(contents: &str) -> Cow<'_, str> {
    if contents.as_bytes().contains(&b'\r') {
        Cow::Owned(contents.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(contents)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::convert::Infallible;
    use std::ops::Deref;

    use pretty_assertions::assert_eq;

    use super::*;
    use crate::error::LayerLoadError;

    fn sample_ufo_entries() -> BTreeMap<PathBuf, String> {
        BTreeMap::from([
            (
                PathBuf::from("metainfo.plist"),
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>creator</key>
    <string>org.linebender.norad</string>
    <key>formatVersion</key>
    <integer>3</integer>
</dict>
</plist>
"#
                .into(),
            ),
            (
                PathBuf::from("layercontents.plist"),
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
    <array>
        <string>public.default</string>
        <string>glyphs</string>
    </array>
</array>
</plist>
"#
                .into(),
            ),
            (
                PathBuf::from("glyphs/contents.plist"),
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>A</key>
    <string>A_.glif</string>
</dict>
</plist>
"#
                .into(),
            ),
            (
                                PathBuf::from("glyphs/A_.glif"),
                r#"<?xml version='1.0' encoding='UTF-8'?>
<glyph name="A" format="2">
  <advance width="500"/>
</glyph>
"#
                .into(),
            ),
            (
                PathBuf::from("features.fea"),
                "languagesystem DFLT dflt;\ninclude( features/includes/shared.fea );\nfeature liga {\n    sub A A by A;\n} liga;\n".into(),
            ),
            (
                PathBuf::from("features/includes/shared.fea"),
                "@shared = [A];\n".into(),
            ),
        ])
    }

    #[test]
    fn load_with_source_preserves_structured_feature_includes() {
        let entries = sample_ufo_entries();
        let mut source =
            |path: &Path| -> Result<Option<String>, Infallible> { Ok(entries.get(path).cloned()) };

        let font = Font::load_with_source(DataRequest::all(), &mut source).unwrap();

        assert_eq!(font.glyph_count(), 1);
        assert_eq!(
            font.features,
            "languagesystem DFLT dflt;\ninclude( features/includes/shared.fea );\nfeature liga {\n    sub A A by A;\n} liga;\n"
        );
        assert_eq!(
            font.feature_files.get(Path::new("features/includes/shared.fea")),
            Some(&"@shared = [A];\n".to_string())
        );
        assert_eq!(
            font.features_expanded().unwrap(),
            "languagesystem DFLT dflt;\n@shared = [A];\nfeature liga {\n    sub A A by A;\n} liga;\n"
        );
    }

    #[test]
    fn save_with_sink_writes_structured_feature_files_and_round_trips() {
        let entries = sample_ufo_entries();
        let mut source =
            |path: &Path| -> Result<Option<String>, Infallible> { Ok(entries.get(path).cloned()) };
        let font = Font::load_with_source(DataRequest::all(), &mut source).unwrap();

        let mut written = BTreeMap::<PathBuf, Vec<u8>>::new();
        let mut sink = |path: &Path, bytes: &[u8]| -> Result<(), Infallible> {
            written.insert(path.to_path_buf(), bytes.to_vec());
            Ok(())
        };

        font.save_with_sink(&WriteOptions::default(), &mut sink).unwrap();

        let features =
            String::from_utf8(written.get(Path::new("features.fea")).unwrap().clone()).unwrap();
        assert_eq!(
            features,
            "languagesystem DFLT dflt;\ninclude( features/includes/shared.fea );\nfeature liga {\n    sub A A by A;\n} liga;\n"
        );
        let shared = String::from_utf8(
            written.get(Path::new("features/includes/shared.fea")).unwrap().clone(),
        )
        .unwrap();
        assert_eq!(shared, "@shared = [A];\n");

        let text_entries: BTreeMap<PathBuf, String> = written
            .into_iter()
            .map(|(path, bytes)| (path, String::from_utf8(bytes).unwrap()))
            .collect();
        let mut reload_source = |path: &Path| -> Result<Option<String>, Infallible> {
            Ok(text_entries.get(path).cloned())
        };
        let reloaded = Font::load_with_source(DataRequest::all(), &mut reload_source).unwrap();

        assert_eq!(reloaded.features, font.features);
        assert_eq!(reloaded.feature_files, font.feature_files);
        assert_eq!(reloaded.features_expanded().unwrap(), font.features_expanded().unwrap());
        assert_eq!(reloaded.glyph_count(), font.glyph_count());
    }

    #[test]
    fn load_with_source_reports_missing_glif_as_glyph_error() {
        let mut entries = sample_ufo_entries();
        entries.remove(Path::new("glyphs/A_.glif"));
        let mut source =
            |path: &Path| -> Result<Option<String>, Infallible> { Ok(entries.get(path).cloned()) };

        let err = Font::load_with_source(DataRequest::all(), &mut source).unwrap_err();

        let FontLoadError::Layer { source, .. } = err else {
            panic!("expected layer load error, found '{err:?}'");
        };
        let LayerLoadError::Glyph { source, .. } = source.deref() else {
            panic!("expected missing glif to surface as glyph error, found '{source:?}'");
        };
        assert!(matches!(source, GlifLoadError::Io(err) if err.kind() == io::ErrorKind::NotFound));
    }
}
