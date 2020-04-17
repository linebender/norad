//! postscript glyph name utilities.
//!
//! This file relies on code that is generated in our build.rs script, which
//! is based on the Adobe Glyph List For New Fonts, at
//! https://github.com/adobe-type-tools/agl-aglfn/blob/master/aglfn.txt

include!(concat!(env!("OUT_DIR"), "/glyph_names_codegen.rs"));

/// Given a `char`, returns the postscript name for that `char`s glyph,
/// if one exists in the aglfn.
pub fn glyph_name_for_char(chr: char) -> Option<&'static str> {
    GLYPH_NAMES.get(&chr).map(|s| *s)
}

/// Given a glyph (represented as a &str), return the postcript name, if one
/// exists in aglfn.
///
/// This returns `None` if there is more than one `char` in the glyph.
///
/// This is a convenience method; we will more often have `&str` than `char`.
pub fn glyph_name_for_glyph(glyph: &str) -> Option<&'static str> {
    let mut chars = glyph.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => GLYPH_NAMES.get(&c).map(|s| *s),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test() {
        assert_eq!(glyph_name_for_char('c'), Some("c"));
        assert_eq!(glyph_name_for_glyph("c"), Some("c"));
        assert_eq!(glyph_name_for_char('C'), Some("C"));
        assert_eq!(glyph_name_for_glyph("C"), Some("C"));

        assert_eq!(glyph_name_for_char('é'), Some("eacute"));
        assert_eq!(glyph_name_for_glyph("é"), Some("eacute"));

        assert_eq!(glyph_name_for_char('<'), Some("less"));
        assert_eq!(glyph_name_for_glyph("ء"), None);
    }
}
