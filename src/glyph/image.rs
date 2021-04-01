use std::path::PathBuf;

use crate::{AffineTransform, Color};

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// Not an absolute / relative path, but the name of the image file.
    pub file_name: PathBuf,
    pub color: Option<Color>,
    pub transform: AffineTransform,
}
