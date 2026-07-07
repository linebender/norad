//! The [`FontSink`] trait for destination-agnostic UFO saving.

use std::io;
use std::path::Path;

/// A destination for UFO file data when saving.
///
/// This trait abstracts over how UFO files are written, allowing fonts to be
/// saved to directories on disk, zip archives, in-memory stores, or any
/// other destination.
///
/// Paths passed to methods are always relative to the UFO root, e.g.
/// `"metainfo.plist"`, `"glyphs/contents.plist"`, `"glyphs/A_.glif"`.
/// Intermediate directories are implied by file paths: writing
/// `"glyphs/A_.glif"` must succeed without any prior directory-creation
/// call. (Every directory in a valid UFO contains at least one file, so no
/// separate directory API is needed.)
///
/// Saving only ever writes files; it never removes anything. If the
/// destination may contain files not part of the font being saved (for
/// example a previously saved UFO directory), clearing it is the caller's
/// responsibility. ([`Font::save`] does this for paths.)
///
/// When the `rayon` feature is enabled, [`write`][Self::write] may be called
/// concurrently from multiple threads (hence `&self` and the `Sync` bound);
/// implementations with mutable state need interior mutability.
///
/// Two implementations are provided out of the box:
///
/// - A filesystem directory (a `&Path`) implements this trait directly, so you
///   can pass a path wherever a `FontSink` is expected.
/// - Any closure `Fn(&Path, &[u8]) -> Result<(), io::Error>` implements it
///   too, which is handy for a quick ad-hoc sink without defining a type:
///
/// ```
/// use std::collections::HashMap;
/// use std::io;
/// use std::path::{Path, PathBuf};
/// use std::sync::Mutex;
/// use norad::Font;
///
/// # fn example(font: &Font) -> Result<(), Box<dyn std::error::Error>> {
/// let files = Mutex::new(HashMap::<PathBuf, Vec<u8>>::new());
/// let sink = |path: &Path, data: &[u8]| {
///     files.lock().unwrap().insert(path.to_owned(), data.to_vec());
///     Ok(())
/// };
/// font.save_to_sink(&sink, &Default::default())?;
/// let files = files.into_inner().unwrap();
/// # Ok(())
/// # }
/// ```
///
/// [`Font::save`]: crate::Font::save
pub trait FontSink: Send + Sync {
    /// Write `data` to the file at the given relative path.
    ///
    /// This creates any missing intermediate directories, and replaces any
    /// existing file at the path.
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), io::Error>;
}

/// A directory on disk implements [`FontSink`] directly.
impl FontSink for &Path {
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), io::Error> {
        let full = self.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        close_already::fs::write(&full, data)
    }
}

// Allow closures as FontSink for convenience.
impl<F> FontSink for F
where
    F: Fn(&Path, &[u8]) -> Result<(), io::Error> + Send + Sync,
{
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), io::Error> {
        self(path, data)
    }
}
