use super::*;

#[test]
fn transform() {
    let transform = AffineTransform::default();
    assert_eq!(transform.x_scale, 1.0);
}
