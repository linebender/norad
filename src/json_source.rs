//! [`FontSource`] implementation for reading a font in ufoLib2's JSON format.

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::error::JsonLoadError;
use crate::font_source::{list_dir_of_files, DirEntry, FontSource};
use crate::{Font, WriteOptions};

/// A [`FontSource`] for a font in [ufoLib2]'s JSON format.
///
/// Requires the `unstable-json` feature. This is not stable API, and may
/// change or break in any release, including patch releases.
///
/// The JSON is a single document rather than a tree of files, so this loads
/// it with [`Font::from_ufolib2_json`] and serves the files that
/// [`Font::save`] would write for that font. All of them are built up front
/// and held in memory.
///
/// [`Font::load`] uses this for paths with a `.json` extension, as does
/// `FontReader::open`.
///
/// Loading from this source gives the font that saving and reloading the
/// result of [`Font::from_ufolib2_json`] would: for instance, line endings in
/// the features become `\n`.
///
/// ```no_run
/// use norad::{FontReader, JsonSource};
///
/// let source = JsonSource::open("path/to/font.json").expect("failed to load");
/// let reader = FontReader::from_source(source).expect("failed to read");
/// ```
///
/// [ufoLib2]: https://github.com/fonttools/ufoLib2
pub struct JsonSource {
    files: HashMap<PathBuf, Vec<u8>>,
}

impl JsonSource {
    /// Returns a [`JsonSource`] for the font in the JSON file at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, JsonLoadError> {
        let bytes = std::fs::read(path.as_ref())?;
        Self::from_bytes(&bytes)
    }

    /// Returns a [`JsonSource`] for the font in `bytes`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, JsonLoadError> {
        let font = Font::from_ufolib2_json(bytes)?;
        let files = Mutex::new(HashMap::new());
        let sink = |path: &Path, data: &[u8]| {
            files.lock().unwrap().insert(path.to_owned(), data.to_vec());
            Ok(())
        };
        // Skip the checks [`Font::save_to_sink`] does first: the JSON loader
        // already did them, except for refusing a `public.objectLibs` key in
        // the font lib, which it keeps there when the font has no info, as
        // loading a .ufo without a fontinfo.plist does.
        font.write_to_sink_impl(&sink, &WriteOptions::default()).map_err(JsonLoadError::Write)?;
        Ok(JsonSource { files: files.into_inner().unwrap() })
    }
}

impl FontSource for JsonSource {
    fn try_read(&self, path: &Path) -> Option<Result<Vec<u8>, io::Error>> {
        self.files.get(path).cloned().map(Ok)
    }

    fn list_dir(&self, path: &Path) -> Result<Vec<DirEntry>, io::Error> {
        Ok(list_dir_of_files(self.files.keys().map(PathBuf::as_path), path))
    }
}

impl fmt::Debug for JsonSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JsonSource").field("files", &self.files.len()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DataRequest;

    fn load(json: &str) -> Font {
        let source = JsonSource::from_bytes(json.as_bytes()).unwrap();
        Font::load_from_source(&DataRequest::all(), &source).unwrap()
    }

    #[test]
    fn empty_font() {
        let source = JsonSource::from_bytes(b"{}").unwrap();
        let mut files: Vec<_> = source.files.keys().cloned().collect();
        files.sort();
        assert_eq!(
            files,
            ["glyphs/contents.plist", "layercontents.plist", "metainfo.plist"].map(PathBuf::from)
        );
        assert_eq!(load("{}"), Font::new());
    }

    /// Without info, `public.objectLibs` stays in the font lib, which saving
    /// would refuse.
    #[test]
    fn object_libs_in_font_lib_without_info() {
        let json = r#"{"lib": {"public.objectLibs": {"g1": {"k": "v"}}}}"#;
        let font = load(json);
        assert!(font.lib.contains_key("public.objectLibs"));
        assert_eq!(font, Font::from_ufolib2_json(json.as_bytes()).unwrap());
    }

    /// As when saving, features get `\n` line endings.
    #[test]
    fn line_endings() {
        let json = r#"{
            "features": "a\r\nb",
            "lib": {"k": "a\r\nb"},
            "layers": [{"glyphs": {"a": {"note": "a\r\nb"}}}]
        }"#;
        let mut font = load(json);
        assert_eq!(font.features, "a\nb");
        font.features = "a\r\nb".into();
        assert_eq!(font, Font::from_ufolib2_json(json.as_bytes()).unwrap());
    }

    #[test]
    fn list_dir() {
        let source = JsonSource::from_bytes(
            br#"{
                "layers": [
                    {"name": "public.default", "glyphs": {"a": {}}},
                    {"name": "background", "glyphs": {"a": {}}}
                ],
                "data": {"com.example/foo.txt": "aGVsbG8="}
            }"#,
        )
        .unwrap();
        let mut root = source.list_dir(Path::new("")).unwrap();
        root.sort();
        assert_eq!(
            root,
            [
                DirEntry::File("layercontents.plist".into()),
                DirEntry::File("metainfo.plist".into()),
                DirEntry::Dir("data".into()),
                DirEntry::Dir("glyphs".into()),
                DirEntry::Dir("glyphs.background".into()),
            ]
        );
        let mut glyphs = source.list_dir(Path::new("glyphs.background")).unwrap();
        glyphs.sort();
        assert_eq!(
            glyphs,
            [DirEntry::File("a.glif".into()), DirEntry::File("contents.plist".into())]
        );
        assert_eq!(
            source.list_dir(Path::new("data")).unwrap(),
            [DirEntry::Dir("com.example".into())]
        );
    }

    #[test]
    fn errors() {
        assert!(matches!(JsonSource::from_bytes(b"[]"), Err(JsonLoadError::Json(_))));
        let missing = Path::new("testdata/ufolib2_json/does_not_exist.json");
        assert!(matches!(JsonSource::open(missing), Err(JsonLoadError::Io(_))));
    }
}
