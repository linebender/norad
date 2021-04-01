#[cfg(feature = "druid")]
use druid::Data;

/// Taken together in order, these fields represent an affine transformation matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "druid", derive(Data))]
pub struct AffineTransform {
    pub x_scale: f32,
    pub xy_scale: f32,
    pub yx_scale: f32,
    pub y_scale: f32,
    pub x_offset: f32,
    pub y_offset: f32,
}

impl AffineTransform {
    ///  [1 0 0 1 0 0]; the identity transformation.
    fn identity() -> Self {
        AffineTransform {
            x_scale: 1.0,
            xy_scale: 0.,
            yx_scale: 0.,
            y_scale: 1.0,
            x_offset: 0.,
            y_offset: 0.,
        }
    }
}

impl Default for AffineTransform {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(feature = "kurbo")]
impl From<AffineTransform> for kurbo::Affine {
    fn from(src: AffineTransform) -> kurbo::Affine {
        kurbo::Affine::new([
            src.x_scale as f64,
            src.xy_scale as f64,
            src.yx_scale as f64,
            src.y_scale as f64,
            src.x_offset as f64,
            src.y_offset as f64,
        ])
    }
}

#[cfg(feature = "kurbo")]
impl From<kurbo::Affine> for AffineTransform {
    fn from(src: kurbo::Affine) -> AffineTransform {
        let coeffs = src.as_coeffs();
        AffineTransform {
            x_scale: coeffs[0] as f32,
            xy_scale: coeffs[1] as f32,
            yx_scale: coeffs[2] as f32,
            y_scale: coeffs[3] as f32,
            x_offset: coeffs[4] as f32,
            y_offset: coeffs[5] as f32,
        }
    }
}
