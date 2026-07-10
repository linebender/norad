//! The [`FontSource`] trait for source-agnostic UFO loading.

use std::io;
use std::path::{Path, PathBuf};

/// A source of UFO file data for loading.
///
/// This trait abstracts over how UFO files are accessed, allowing fonts to be
/// loaded from directories on disk, zip archives, in-memory stores, or any
/// other source.
///
/// Paths passed to methods are always relative to the UFO root, e.g.
/// `"metainfo.plist"`, `"glyphs/contents.plist"`, `"glyphs/A_.glif"`.
///
/// Two implementations are provided out of the box:
///
/// - A filesystem directory (a `&Path`) implements this trait directly, so you
///   can pass a path wherever a `FontSource` is expected.
/// - Any closure `Fn(&Path) -> Option<Result<Vec<u8>, io::Error>>` implements it
///   too, which is handy for a quick ad-hoc source without defining a type:
///
/// ```
/// use std::io;
/// use std::path::Path;
/// use norad::{DataRequest, Font};
///
/// # fn example(lookup: impl Fn(&Path) -> Option<Vec<u8>> + Sync) -> Result<(), Box<dyn std::error::Error>> {
/// let source = |path: &Path| lookup(path).map(Ok::<_, io::Error>);
/// let font = Font::load_from_source(&DataRequest::all(), &source)?;
/// # Ok(())
/// # }
/// ```
///
/// # Implementing
///
/// A simple in-memory implementation:
///
/// ```
/// use std::collections::HashMap;
/// use std::io;
/// use std::path::{Path, PathBuf};
/// use norad::FontSource;
///
/// struct MemorySource(HashMap<PathBuf, Vec<u8>>);
///
/// impl FontSource for MemorySource {
///     fn try_read(&self, path: &Path) -> Option<Result<Vec<u8>, io::Error>> {
///         self.0.get(path).cloned().map(Ok)
///     }
/// }
/// ```
pub trait FontSource: Sync {
    /// Try to read the contents of a file at the given relative path.
    ///
    /// Returns `None` if the file does not exist, `Some(Ok(data))` if the
    /// file was read successfully, or `Some(Err(..))` if the file exists but
    /// could not be read.
    fn try_read(&self, path: &Path) -> Option<Result<Vec<u8>, io::Error>>;

    /// Read the contents of a file, returning [`io::ErrorKind::NotFound`] if
    /// the file does not exist.
    fn read(&self, path: &Path) -> Result<Vec<u8>, io::Error> {
        self.try_read(path).unwrap_or_else(|| {
            Err(io::Error::new(io::ErrorKind::NotFound, path.display().to_string()))
        })
    }

    /// If this source is backed by a directory on disk, return the root path.
    ///
    /// This is used by data/image stores to enable lazy loading: when a path
    /// is available, the store records it and defers reading until access time.
    /// The default returns `None`, meaning the store will eagerly load all data.
    fn as_path(&self) -> Option<&Path> {
        None
    }

    /// List entries in a directory at the given relative path.
    ///
    /// Returns a [`DirEntry`] for each entry, carrying the entry name (not a
    /// full path). Callers should join with the directory path to get the full
    /// relative path.
    ///
    /// The default implementation returns [`io::ErrorKind::Unsupported`],
    /// meaning this source does not support directory enumeration. Data and
    /// image stores will be empty for such sources.
    fn list_dir(&self, _path: &Path) -> Result<Vec<DirEntry>, io::Error> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "this FontSource does not support directory listing",
        ))
    }
}

/// The name of a file or directory, relative to a parent (not a full path!)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirEntry {
    /// A file, carrying its name relative to the parent directory.
    File(PathBuf),
    /// A subdirectory, carrying its name relative to the parent directory.
    Dir(PathBuf),
}

/// A directory on disk implements [`FontSource`] directly.
impl FontSource for &Path {
    fn try_read(&self, path: &Path) -> Option<Result<Vec<u8>, io::Error>> {
        match std::fs::read(self.join(path)) {
            Ok(data) => Some(Ok(data)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => Some(Err(e)),
        }
    }

    fn as_path(&self) -> Option<&Path> {
        Some(self)
    }

    fn list_dir(&self, path: &Path) -> Result<Vec<DirEntry>, io::Error> {
        let full = self.join(path);
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&full)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let name = PathBuf::from(entry.file_name());
            entries.push(if metadata.is_dir() {
                DirEntry::Dir(name)
            } else {
                DirEntry::File(name)
            });
        }
        Ok(entries)
    }
}

// Allow closures as FontSource for convenience.
impl<F> FontSource for F
where
    F: Fn(&Path) -> Option<Result<Vec<u8>, io::Error>> + Sync,
{
    fn try_read(&self, path: &Path) -> Option<Result<Vec<u8>, io::Error>> {
        self(path)
    }
}
