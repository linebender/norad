//! Common utilities.

//NOTE: this is hacky, and intended mostly as a placeholder. It was adapted from
// https://github.com/unified-font-object/ufoLib/blob/master/Lib/ufoLib/filenames.py
/// given a glyph name, compute an appropriate file name.
pub fn default_file_name_for_glyph_name(name: impl AsRef<str>) -> String {
    let name = name.as_ref();
    user_name_to_file_name(name, "", ".glif")
}

/// given a layer name, compute an appropriate file name.
pub fn default_file_name_for_layer_name(name: &str) -> String {
    user_name_to_file_name(name, "glyphs.", "")
}

//FIXME: this needs to also handle duplicate names, probably by passing in some
// 'exists' fn, like: `impl Fn(&str) -> bool`
fn user_name_to_file_name(name: &str, prefix: &str, suffix: &str) -> String {
    static SPECIAL_ILLEGAL: &[char] = &['\\', '*', '+', '/', ':', '<', '>', '?', '[', ']', '|'];
    const MAX_LEN: usize = 255;

    let mut result = String::with_capacity(name.len() + prefix.len() + suffix.len());
    result.push_str(prefix);

    for c in name.chars() {
        match c {
            '.' if result.is_empty() => result.push('_'),
            c if (c as u32) < 32 || (c as u32) == 0x7f || SPECIAL_ILLEGAL.contains(&c) => {
                result.push('_')
            }
            c if c.is_ascii_uppercase() => {
                result.push(c);
                result.push('_');
            }
            c => result.push(c),
        }
    }

    //TODO: check for illegal names, duplicate names
    if result.len() + suffix.len() > MAX_LEN {
        let mut boundary = 255 - suffix.len();
        while !result.is_char_boundary(boundary) {
            boundary -= 1;
        }
        result.truncate(boundary);
    }
    result.push_str(suffix);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_for_name() {
        fn trimmed_name(name: &str) -> String {
            default_file_name_for_glyph_name(name).trim_end_matches(".glif").into()
        }

        assert_eq!(trimmed_name("newGlyph.1"), "newG_lyph.1".to_string());
        assert_eq!(trimmed_name("a"), "a".to_string());
        assert_eq!(trimmed_name("A"), "A_".to_string());
        assert_eq!(trimmed_name("AE"), "A_E_".to_string());
        assert_eq!(trimmed_name("Ae"), "A_e".to_string());
        assert_eq!(trimmed_name("ae"), "ae".to_string());
        assert_eq!(trimmed_name("aE"), "aE_".to_string());
        assert_eq!(trimmed_name("a.alt"), "a.alt".to_string());
        assert_eq!(trimmed_name("A.alt"), "A_.alt".to_string());
        assert_eq!(trimmed_name("A.Alt"), "A_.A_lt".to_string());
        assert_eq!(trimmed_name("A.aLt"), "A_.aL_t".to_string());
        assert_eq!(trimmed_name("A.alT"), "A_.alT_".to_string());
        assert_eq!(trimmed_name("T_H"), "T__H_".to_string());
        assert_eq!(trimmed_name("T_h"), "T__h".to_string());
        assert_eq!(trimmed_name("t_h"), "t_h".to_string());
        assert_eq!(trimmed_name("F_F_I"), "F__F__I_".to_string());
        assert_eq!(trimmed_name("f_f_i"), "f_f_i".to_string());
        assert_eq!(trimmed_name("Aacute_V.swash"), "A_acute_V_.swash".to_string());
        assert_eq!(trimmed_name(".notdef"), "_notdef".to_string());

        //FIXME: we're ignoring 'reserved filenames' for now
        //assert_eq!(trimmed_name("con"), "_con".to_string());
        //assert_eq!(trimmed_name("CON"), "C_O_N_".to_string());
        //assert_eq!(trimmed_name("con.alt"), "_con.alt".to_string());
        //assert_eq!(trimmed_name("alt.con"), "alt._con".to_string());
    }
}
