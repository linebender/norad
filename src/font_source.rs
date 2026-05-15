//! The [`FontSource`] trait for non-filesystem UFO loading.

use std::path::Path;

/// A source of UFO file contents for non-filesystem loading.
///
/// Paths passed to [`read_contents`](FontSource::read_contents) are always relative to the UFO
/// root directory, e.g. `"metainfo.plist"`, `"glyphs/contents.plist"`,
/// `"glyphs/A_.glif"`.
///
/// # Implementing
///
/// The simplest implementation wraps a `HashMap<String, String>`:
///
/// ```
/// use std::collections::HashMap;
/// use std::path::Path;
/// use norad::FontSource;
///
/// struct MemorySource(HashMap<String, String>);
///
/// impl FontSource for MemorySource {
///     type Error = std::convert::Infallible;
///     fn read_contents(&self, path: &Path) -> Result<Option<String>, Self::Error> {
///         Ok(self.0.get(path.to_str().unwrap_or("")).cloned())
///     }
/// }
/// ```
///
/// A closure also works directly:
///
/// ```
/// use std::path::Path;
/// use norad::Font;
/// use norad::DataRequest;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let source = |path: &Path| Ok::<_, std::convert::Infallible>(None);
/// let font = Font::load_from_source(DataRequest::all(), &source)?;
/// # Ok(())
/// # }
/// ```
pub trait FontSource {
    /// The error type returned by [`read_contents`](FontSource::read_contents).
    type Error: std::error::Error + Send + Sync + 'static;

    /// Return the contents of the file at the given path, or `None` if it doesn't exist.
    fn read_contents(&self, path: &Path) -> Result<Option<String>, Self::Error>;
}

impl<F, E> FontSource for F
where
    F: Fn(&Path) -> Result<Option<String>, E>,
    E: std::error::Error + Send + Sync + 'static,
{
    type Error = E;
    fn read_contents(&self, path: &Path) -> Result<Option<String>, E> {
        self(path)
    }
}

