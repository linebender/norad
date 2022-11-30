//! Common utilities.

use std::fmt::Write as _;
use std::{collections::HashSet, path::PathBuf};

use crate::Name;

/// Given a `plist::Dictionary`, recursively sort keys.
///
/// This ensures we have a consistent serialization order.
pub(crate) fn recursive_sort_plist_keys(plist: &mut plist::Dictionary) {
    plist.sort_keys();
    for val in plist.values_mut() {
        if let Some(dict) = val.as_dictionary_mut() {
            recursive_sort_plist_keys(dict);
        }
    }
}

/// Given a glyph `name`, return an appropriate file name.
pub(crate) fn default_file_name_for_glyph_name(name: &Name, existing: &HashSet<String>) -> PathBuf {
    user_name_to_file_name(name, "", ".glif", |name| !existing.contains(name))
}

/// Given a layer `name`, return an appropriate file name.
pub(crate) fn default_file_name_for_layer_name(name: &Name, existing: &HashSet<String>) -> PathBuf {
    user_name_to_file_name(name, "glyphs.", "", |name| !existing.contains(name))
}

/// Given a `name`, return an appropriate file name.
///
/// This file name is computed via the [Common User Name to File Name Algorithm][algo]
/// defined in the UFO spec.
///
/// the `prefix` and `suffix` fields will be added to the start and end of the
/// generated path; for instance the `suffix` might be a file extension.
///
/// The `accept_path` closure is a way of indicating whether or not a candidate
/// path should be used. In general, this involves ensuring that the candidate path
/// does not already exist. Paths are always lowercased, and case insensitive.
///
/// # Panics
///
/// Panics if a case-insensitive file name clash was detected and no unique
/// value could be created after 99 numbering attempts.
///
/// [algo]: https://unifiedfontobject.org/versions/ufo3/conventions/#common-user-name-to-file-name-algorithm
pub fn user_name_to_file_name(
    name: impl AsRef<str>,
    prefix: &str,
    suffix: &str,
    mut accept_path: impl FnMut(&str) -> bool,
) -> PathBuf {
    let name = name.as_ref();
    let mut result = String::with_capacity(prefix.len() + name.len() + suffix.len());

    // Filter illegal characters from name.
    static SPECIAL_ILLEGAL: &[char] =
        &[':', '?', '"', '(', ')', '[', ']', '*', '/', '\\', '+', '<', '>', '|'];

    // Assert that the prefix and suffix are safe, as they should be controlled
    // by norad.
    debug_assert!(
        !prefix.chars().any(|c| SPECIAL_ILLEGAL.contains(&c)),
        "prefix must not contain illegal chars"
    );
    debug_assert!(
        suffix.is_empty() || suffix.starts_with('.'),
        "suffix must be empty or start with a period"
    );
    debug_assert!(
        !suffix.chars().any(|c| SPECIAL_ILLEGAL.contains(&c)),
        "suffix must not contain illegal chars"
    );
    debug_assert!(!suffix.ends_with(['.', ' ']), "suffix must not end in period or space");

    result.push_str(prefix);
    for c in name.chars() {
        match c {
            // Replace an initial period with an underscore if there is no
            // prefix to be added, e.g. for the bare glyph name ".notdef".
            '.' if result.is_empty() => result.push('_'),
            // Replace illegal characters with an underscore.
            c if SPECIAL_ILLEGAL.contains(&c) => result.push('_'),
            // Append an underscore to all uppercase characters.
            c if c.is_uppercase() => {
                result.push(c);
                result.push('_');
            }
            // Append the rest unchanged.
            c => result.push(c),
        }
    }

    // Test for reserved names and parts. The relevant part is the prefix + name
    // (or "stem") of the file, so e.g. "com1.glif" would be replaced by
    // "_com1.glif", but "hello.com1.glif", "com10.glif" and "acom1.glif" stay
    // as they are. For algorithmic simplicity, ignore the presence of the
    // suffix and potentially replace more than we strictly need to.
    //
    // List taken from
    // <https://docs.microsoft.com/en-gb/windows/win32/fileio/naming-a-file#naming-conventions>.
    static SPECIAL_RESERVED: &[&str] = &[
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    if let Some(stem) = result.split('.').next() {
        // At this stage, we only need to look for lowercase matches, as every
        // uppercase letter will be followed by an underscore, automatically
        // making the name safe.
        if SPECIAL_RESERVED.contains(&stem) {
            result.insert(0, '_');
        }
    }

    // Clip prefix + name to 255 characters.
    const MAX_LEN: usize = 255;
    if result.len().saturating_add(suffix.len()) > MAX_LEN {
        let mut boundary = MAX_LEN.saturating_sub(suffix.len());
        while !result.is_char_boundary(boundary) {
            boundary -= 1;
        }
        result.truncate(boundary);
    }

    // Replace trailing periods and spaces by underscores unless we have a
    // suffix (which we asserted is safe).
    if suffix.is_empty() && result.ends_with(['.', ' ']) {
        let mut boundary = result.len();
        for (i, c) in result.char_indices().rev() {
            if c != '.' && c != ' ' {
                break;
            }
            boundary = i;
        }
        let underscores = "_".repeat(result.len() - boundary);
        result.replace_range(boundary..result.len(), &underscores);
    }

    result.push_str(suffix);

    // Test for clashes. Use a counter with 2 digits to look for a name not yet
    // taken. The UFO specification recommends using 15 digits and lists a
    // second way should one exhaust them, but it is unlikely to be needed in
    // practice. 1e15 numbers is a ridicuously high number where holding all
    // those glyph names in memory would exhaust it.
    if !accept_path(&result.to_lowercase()) {
        // First, cut off the suffix (plus the space needed for the number
        // counter if necessary).
        const NUMBER_LEN: usize = 2;
        if result.len().saturating_sub(suffix.len()).saturating_add(NUMBER_LEN) > MAX_LEN {
            let mut boundary = MAX_LEN.saturating_sub(suffix.len()).saturating_sub(NUMBER_LEN);
            while !result.is_char_boundary(boundary) {
                boundary -= 1;
            }
            result.truncate(boundary);
        } else {
            // Cutting off the suffix should land on a `char` boundary.
            result.truncate(result.len().saturating_sub(suffix.len()));
        }

        let mut found_unique = false;
        for counter in 1..100u8 {
            write!(&mut result, "{:0>2}", counter).unwrap();
            result.push_str(suffix);
            if accept_path(&result.to_lowercase()) {
                //if !existing.contains(&result.to_lowercase()) {
                found_unique = true;
                break;
            }
            result.truncate(result.len().saturating_sub(suffix.len()) - NUMBER_LEN);
        }
        if !found_unique {
            // Note: if this is ever hit, try appending a UUIDv4 before panicing.
            panic!("Could not find a unique file name after 99 tries")
        }
    }

    result.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glif_stem(name: &str) -> String {
        let container: HashSet<String> = HashSet::new();
        default_file_name_for_glyph_name(&Name::new_raw(name), &container)
            .to_string_lossy()
            .trim_end_matches(".glif")
            .into()
    }

    fn file_name(name: &str, prefix: &str, suffix: &str) -> String {
        let container: HashSet<String> = HashSet::new();
        user_name_to_file_name(&Name::new_raw(name), prefix, suffix, |name| {
            !container.contains(name)
        })
        .to_string_lossy()
        .to_string()
    }

    #[test]
    fn path_for_name_basic() {
        assert_eq!(glif_stem("newGlyph.1"), "newG_lyph.1".to_string());
        assert_eq!(glif_stem("a"), "a".to_string());
        assert_eq!(glif_stem("A"), "A_".to_string());
        assert_eq!(glif_stem("AE"), "A_E_".to_string());
        assert_eq!(glif_stem("Ae"), "A_e".to_string());
        assert_eq!(glif_stem("ae"), "ae".to_string());
        assert_eq!(glif_stem("aE"), "aE_".to_string());
        assert_eq!(glif_stem("a.alt"), "a.alt".to_string());
        assert_eq!(glif_stem("A.alt"), "A_.alt".to_string());
        assert_eq!(glif_stem("A.Alt"), "A_.A_lt".to_string());
        assert_eq!(glif_stem("A.aLt"), "A_.aL_t".to_string());
        assert_eq!(glif_stem("A.alT"), "A_.alT_".to_string());
        assert_eq!(glif_stem("T_H"), "T__H_".to_string());
        assert_eq!(glif_stem("T_h"), "T__h".to_string());
        assert_eq!(glif_stem("t_h"), "t_h".to_string());
        assert_eq!(glif_stem("F_F_I"), "F__F__I_".to_string());
        assert_eq!(glif_stem("f_f_i"), "f_f_i".to_string());
        assert_eq!(glif_stem("Aacute_V.swash"), "A_acute_V_.swash".to_string());
        assert_eq!(glif_stem(".notdef"), "_notdef".to_string());
        assert_eq!(glif_stem("..notdef"), "_.notdef".to_string());
        assert_eq!(glif_stem("con"), "_con".to_string());
        assert_eq!(glif_stem("CON"), "C_O_N_".to_string());
        assert_eq!(glif_stem("con.alt"), "_con.alt".to_string());
        assert_eq!(glif_stem("alt.con"), "alt.con".to_string());
    }

    #[test]
    fn path_for_name_starting_dots() {
        assert_eq!(glif_stem("..notdef"), "_.notdef".to_string());
        assert_eq!(file_name(".notdef", "glyphs.", ""), "glyphs..notdef".to_string());
    }

    #[test]
    fn path_for_name_unicode() {
        assert_eq!(file_name("А Б ВГ абвг", "", ""), "А_ Б_ В_Г_ абвг".to_string());
    }

    #[test]
    fn path_for_name_reserved() {
        assert_eq!(file_name("con", "", ".glif"), "_con.glif".to_string());
        assert_eq!(file_name("Con", "", ".glif"), "C_on.glif".to_string());
        assert_eq!(file_name("cOn", "", ".glif"), "cO_n.glif".to_string());
        assert_eq!(file_name("con._", "", ".glif"), "_con._.glif".to_string());
        assert_eq!(file_name("alt.con", "", ".glif"), "alt.con.glif".to_string());
        assert_eq!(file_name("con", "con.", ".con"), "_con.con.con".to_string());

        assert_eq!(file_name("com1", "", ""), "_com1".to_string());
        assert_eq!(file_name("com1", "", ".glif"), "_com1.glif".to_string());
        assert_eq!(file_name("com1.", "", ".glif"), "_com1..glif".to_string());
        assert_eq!(file_name("com10", "", ".glif"), "com10.glif".to_string());
        assert_eq!(file_name("acom1", "", ".glif"), "acom1.glif".to_string());
        assert_eq!(file_name("com1", "hello.", ".glif"), "hello.com1.glif".to_string());
    }

    #[test]
    fn path_for_name_trailing_periods_spaces() {
        assert_eq!(file_name("alt.", "", ""), "alt_".to_string());
        assert_eq!(file_name("alt.", "", ".glif"), "alt..glif".to_string());
        assert_eq!(file_name("alt..  ", "", ".glif"), "alt..  .glif".to_string());
        assert_eq!(file_name("alt..  ", "", ""), "alt____".to_string());
        assert_eq!(file_name("alt..  a. ", "", ""), "alt..  a__".to_string());
    }

    #[test]
    fn path_for_name_max_length() {
        let spacy_glif_name = format!("{}.glif", " ".repeat(250));
        assert_eq!(file_name(&" ".repeat(255), "", ".glif"), spacy_glif_name);
        assert_eq!(file_name(&" ".repeat(256), "", ".glif"), spacy_glif_name);
        let dotty_glif_name = format!("_{}.glif", ".".repeat(249));
        assert_eq!(file_name(&".".repeat(255), "", ".glif"), dotty_glif_name);
        assert_eq!(file_name(&".".repeat(256), "", ".glif"), dotty_glif_name);
        let underscore_glif_name = "_".repeat(255);
        assert_eq!(file_name(&" ".repeat(255), "", ""), underscore_glif_name);
        assert_eq!(file_name(&".".repeat(255), "", ""), underscore_glif_name);
        assert_eq!(file_name(&" ".repeat(256), "", ""), underscore_glif_name);
        assert_eq!(file_name(&".".repeat(256), "", ""), underscore_glif_name);
        assert_eq!(file_name(&format!("{}💖", " ".repeat(254)), "", ".glif"), spacy_glif_name);
    }

    #[test]
    fn path_for_name_all_ascii() {
        let almost_all_ascii: String = (32..0x7F).map(|i| char::from_u32(i).unwrap()).collect();
        assert_eq!(glif_stem(&almost_all_ascii), " !_#$%&'____,-._0123456789_;_=__@A_B_C_D_E_F_G_H_I_J_K_L_M_N_O_P_Q_R_S_T_U_V_W_X_Y_Z____^_`abcdefghijklmnopqrstuvwxyz{_}~");
    }

    #[test]
    fn path_for_name_clashes() {
        let mut container = HashSet::new();
        let mut existing = HashSet::new();
        for name in ["Ab", "a_b"] {
            let path = user_name_to_file_name(&Name::new_raw(name), "", ".glif", |name| {
                !existing.contains(name)
            });
            existing.insert(path.to_string_lossy().to_string().to_lowercase());
            container.insert(path.to_string_lossy().to_string());
        }

        let mut container_expected = HashSet::new();
        container_expected.insert("A_b.glif".to_string());
        container_expected.insert("a_b01.glif".to_string());

        assert_eq!(container, container_expected);
    }

    #[test]
    fn path_for_name_clashes_max_len() {
        let mut container = HashSet::new();
        let mut existing = HashSet::new();
        for name in ["A".repeat(300), "a_".repeat(150)] {
            let path = user_name_to_file_name(&Name::new_raw(&name), "", ".glif", |name| {
                !existing.contains(name)
            });
            existing.insert(path.to_string_lossy().to_string().to_lowercase());
            container.insert(path.to_string_lossy().to_string());
        }

        let mut container_expected = HashSet::new();
        container_expected.insert(format!("{}.glif", "A_".repeat(125)));
        container_expected.insert(format!("{}01.glif", "a_".repeat(125)));

        assert_eq!(container, container_expected);
    }
}
