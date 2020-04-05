use std::collections::{BTreeMap, HashMap, HashSet};

use crate::glyph::GlyphName;
use crate::ufo::{Groups, Kerning};

/// Convert kerning groups and pairs from v1 and v2 informal conventions to v3 formal conventions.
/// Converted groups are added (duplicated) rather than replacing the old ones to preserve all data
/// that external entities might rely on. Kerning pairs are updated to reflect the new group names.
///
/// This is an adaptation from the fontTools.ufoLib reference implementation. It will not check if
/// the upgraded groups pass validation.
pub fn upconvert_kerning(
    groups: &Groups,
    kerning: &Kerning,
    glyph_set: &HashSet<GlyphName>,
) -> (Groups, Kerning) {
    // Gather known kerning groups based on the prefixes. This will catch groups that exist in
    // `groups` but are not referenced in `kerning`.
    let (mut groups_first, mut groups_second) = find_known_kerning_groups(&groups);

    // Make lists of groups referenced in kerning pairs, based on their side.
    for (first, seconds) in kerning {
        if groups.contains_key(first)
            && !glyph_set.contains(first.as_str())
            && !first.starts_with("public.kern1.")
        {
            groups_first.insert(first.to_string());
        }
        for second in seconds.keys() {
            if groups.contains_key(second)
                && !glyph_set.contains(second.as_str())
                && !second.starts_with("public.kern2.")
            {
                groups_second.insert(second.to_string());
            }
        }
    }

    // Duplicate kerning groups with a new name.
    let mut groups_new = groups.clone();

    let mut groups_first_old_to_new: HashMap<&String, String> = HashMap::new();
    for first in &groups_first {
        let first_new = make_unique_group_name(
            format!("public.kern1.{}", first.replace("@MMK_L_", "")),
            &groups_new,
        );
        groups_first_old_to_new.insert(first, first_new.to_string());
        groups_new.insert(first_new, groups_new.get(first).unwrap().clone());
    }
    let mut groups_second_old_to_new: HashMap<&String, String> = HashMap::new();
    for second in &groups_second {
        let second_new = make_unique_group_name(
            format!("public.kern2.{}", second.replace("@MMK_R_", "")),
            &groups_new,
        );
        groups_second_old_to_new.insert(second, second_new.to_string());
        groups_new.insert(second_new, groups_new.get(second).unwrap().clone());
    }

    // Update all kerning pairs that have an old kerning group in them with the new name.
    let mut kerning_new: Kerning = Kerning::new();

    for (first, seconds) in kerning {
        let first_new = groups_first_old_to_new.get(first).unwrap_or(first);
        let mut seconds_new: BTreeMap<String, f32> = BTreeMap::new();
        for (second, value) in seconds {
            let second_new = groups_second_old_to_new.get(second).unwrap_or(second);
            seconds_new.insert(second_new.to_string(), *value);
        }
        kerning_new.insert(first_new.to_string(), seconds_new);
    }

    (groups_new, kerning_new)
}

fn make_unique_group_name(name: String, existing_groups: &Groups) -> String {
    if !existing_groups.contains_key(&name) {
        return name;
    }

    let mut counter = 1;
    let mut new_name = name.to_string();
    while existing_groups.contains_key(&new_name) {
        new_name = format!("{}{}", name, counter);
        counter += 1;
    }

    new_name
}

fn find_known_kerning_groups(groups: &Groups) -> (HashSet<String>, HashSet<String>) {
    let mut groups_first: HashSet<String> = HashSet::new();
    let mut groups_second: HashSet<String> = HashSet::new();

    for name in groups.keys() {
        if name.starts_with("@MMK_L_") {
            groups_first.insert(name.to_string());
        } else if name.starts_with("@MMK_R_") {
            groups_second.insert(name.to_string());
        }
    }

    (groups_first, groups_second)
}

#[cfg(test)]
mod tests {
    extern crate maplit;

    use super::*;
    use crate::ufo::{FormatVersion, Ufo};
    use maplit::btreemap;

    #[test]
    fn test_upconvert_kerning_just_groups() {
        let groups: Groups = btreemap! {
            "@MMK_L_1".into() => vec!["a".into()],
            "@MMK_L_2".into() => vec!["b".into()],
            "@MMK_L_3".into() => vec!["c".into()],
            "@MMK_R_1".into() => vec!["d".into()],
            "@MMK_R_2".into() => vec!["e".into()],
            "@MMK_R_3".into() => vec!["f".into()],
            "@MMK_l_1".into() => vec!["g".into(), "h".into()],
            "@MMK_r_1".into() => vec!["i".into()],
            "@MMK_X_1".into() => vec!["j".into()],
            "foo".into() => vec![],
        };
        let kerning: Kerning = Kerning::new();
        let glyph_set: HashSet<GlyphName> = vec![
            "a".into(),
            "b".into(),
            "c".into(),
            "d".into(),
            "e".into(),
            "f".into(),
            "g".into(),
            "h".into(),
            "i".into(),
            "j".into(),
        ]
        .into_iter()
        .collect();

        let (groups_new, kerning_new) = upconvert_kerning(&groups, &kerning, &glyph_set);

        assert_eq!(
            groups_new,
            btreemap! {
                "@MMK_L_1".into() => vec!["a".into()],
                "@MMK_L_2".into() => vec!["b".into()],
                "@MMK_L_3".into() => vec!["c".into()],
                "@MMK_R_1".into() => vec!["d".into()],
                "@MMK_R_2".into() => vec!["e".into()],
                "@MMK_R_3".into() => vec!["f".into()],
                "@MMK_l_1".into() => vec!["g".into(),"h".into()],
                "@MMK_r_1".into() => vec!["i".into()],
                "@MMK_X_1".into() => vec!["j".into()],
                "foo".into() => vec![],
                "public.kern1.1".into() => vec!["a".into()],
                "public.kern1.2".into() => vec!["b".into()],
                "public.kern1.3".into() => vec!["c".into()],
                "public.kern2.1".into() => vec!["d".into()],
                "public.kern2.2".into() => vec!["e".into()],
                "public.kern2.3".into() => vec!["f".into()],
            }
        );
        assert_eq!(kerning_new, kerning);
    }

    #[test]
    fn test_upconvert_kerning_unknown_prefixes() {
        let groups: Groups = btreemap! {
            "BGroup".into() => vec!["B".into()],
            "CGroup".into() => vec!["C".into()],
            "DGroup".into() => vec!["D".into()],
        };
        let kerning: Kerning = btreemap! {
            "A".into() => btreemap!{
                "A".into() => 1.0,
                "B".into() => 2.0,
                "CGroup".into() => 3.0,
                "DGroup".into() => 4.0,
            },
            "BGroup".into() => btreemap!{
                "A".into() => 5.0,
                "B".into() => 6.0,
                "CGroup".into() => 7.0,
                "DGroup".into() => 8.0,
            },
            "CGroup".into() => btreemap!{
                "A".into() => 9.0,
                "B".into() => 10.0,
                "CGroup".into() => 11.0,
                "DGroup".into() => 12.0,
            },
        };
        let glyph_set: HashSet<GlyphName> = HashSet::new();

        let (groups_new, kerning_new) = upconvert_kerning(&groups, &kerning, &glyph_set);

        assert_eq!(
            groups_new,
            btreemap! {
                "BGroup".into() => vec!["B".into()],
                "CGroup".into() => vec!["C".into()],
                "DGroup".into() => vec!["D".into()],
                "public.kern1.BGroup".into() => vec!["B".into()],
                "public.kern1.CGroup".into() => vec!["C".into()],
                "public.kern2.CGroup".into() => vec!["C".into()],
                "public.kern2.DGroup".into() => vec!["D".into()],
            }
        );
        assert_eq!(
            kerning_new,
            btreemap! {
                "A".into()  => btreemap!{
                    "A".into() => 1.0,
                    "B".into() => 2.0,
                    "public.kern2.CGroup".into() => 3.0,
                    "public.kern2.DGroup".into() => 4.0,
                },
                "public.kern1.BGroup".into() => btreemap!{
                    "A".into() => 5.0,
                    "B".into() => 6.0,
                    "public.kern2.CGroup".into() => 7.0,
                    "public.kern2.DGroup".into() => 8.0,
                },
                "public.kern1.CGroup".into() => btreemap!{
                    "A".into() => 9.0,
                    "B".into() => 10.0,
                    "public.kern2.CGroup".into() => 11.0,
                    "public.kern2.DGroup".into() => 12.0,
                }
            }
        );
    }

    #[test]
    fn test_upconvert_kerning_known_prefixes() {
        let groups: Groups = btreemap! {
            "@MMK_L_BGroup".into() => vec!["B".into()],
            "@MMK_L_CGroup".into() => vec!["C".into()],
            "@MMK_L_XGroup".into() => vec!["X".into()],
            "@MMK_R_CGroup".into() => vec!["C".into()],
            "@MMK_R_DGroup".into() => vec!["D".into()],
            "@MMK_R_XGroup".into() => vec!["X".into()],
        };
        let kerning: Kerning = btreemap! {
            "A".into() => btreemap!{
                "A".into() => 1.0,
                "B".into() => 2.0,
                "@MMK_R_CGroup".into() => 3.0,
                "@MMK_R_DGroup".into() => 4.0,
            },
            "@MMK_L_BGroup".into() => btreemap!{
                "A".into() => 5.0,
                "B".into() => 6.0,
                "@MMK_R_CGroup".into() => 7.0,
                "@MMK_R_DGroup".into() => 8.0,
            },
            "@MMK_L_CGroup".into() => btreemap!{
                "A".into() => 9.0,
                "B".into() => 10.0,
                "@MMK_R_CGroup".into() => 11.0,
                "@MMK_R_DGroup".into() => 12.0,
            },
        };
        let glyph_set: HashSet<GlyphName> = HashSet::new();

        let (groups_new, kerning_new) = upconvert_kerning(&groups, &kerning, &glyph_set);

        assert_eq!(
            groups_new,
            btreemap! {
                "@MMK_L_BGroup".into() => vec!["B".into()],
                "@MMK_L_CGroup".into() => vec!["C".into()],
                "@MMK_L_XGroup".into() => vec!["X".into()],
                "@MMK_R_CGroup".into() => vec!["C".into()],
                "@MMK_R_DGroup".into() => vec!["D".into()],
                "@MMK_R_XGroup".into() => vec!["X".into()],
                "public.kern1.BGroup".into() => vec!["B".into()],
                "public.kern1.CGroup".into() => vec!["C".into()],
                "public.kern1.XGroup".into() => vec!["X".into()],
                "public.kern2.CGroup".into() => vec!["C".into()],
                "public.kern2.DGroup".into() => vec!["D".into()],
                "public.kern2.XGroup".into() => vec!["X".into()],
            }
        );
        assert_eq!(
            kerning_new,
            btreemap! {
                "A".into() => btreemap!{
                    "A".into() => 1.0,
                    "B".into() => 2.0,
                    "public.kern2.CGroup".into() => 3.0,
                    "public.kern2.DGroup".into() => 4.0,
                },
                "public.kern1.BGroup".into() => btreemap!{
                    "A".into() => 5.0,
                    "B".into() => 6.0,
                    "public.kern2.CGroup".into() => 7.0,
                    "public.kern2.DGroup".into() => 8.0,
                },
                "public.kern1.CGroup".into() => btreemap!{
                    "A".into() => 9.0,
                    "B".into() => 10.0,
                    "public.kern2.CGroup".into() => 11.0,
                    "public.kern2.DGroup".into() => 12.0,
                }
            }
        );
    }

    #[test]
    fn test_upconvert_kerning_mixed_prefixes() {
        let groups: Groups = btreemap! {
            "BGroup".into() => vec!["B".into()],
            "@MMK_L_CGroup".into() => vec!["C".into()],
            "@MMK_R_CGroup".into() => vec!["C".into()],
            "DGroup".into() => vec!["D".into()],
        };
        let kerning: Kerning = btreemap! {
            "A".into() => btreemap!{
                "A".into() => 1.0,
                "B".into() => 2.0,
                "@MMK_R_CGroup".into() => 3.0,
                "DGroup".into() => 4.0,
            },
            "BGroup".into() => btreemap!{
                "A".into() => 5.0,
                "B".into() => 6.0,
                "@MMK_R_CGroup".into() => 7.0,
                "DGroup".into() => 8.0,
            },
            "@MMK_L_CGroup".into() => btreemap!{
                "A".into() => 9.0,
                "B".into() => 10.0,
                "@MMK_R_CGroup".into() => 11.0,
                "DGroup".into() => 12.0,
            },
        };
        let glyph_set: HashSet<GlyphName> = HashSet::new();

        let (groups_new, kerning_new) = upconvert_kerning(&groups, &kerning, &glyph_set);

        assert_eq!(
            groups_new,
            btreemap! {
                "BGroup".into() => vec!["B".into()],
                "@MMK_L_CGroup".into() => vec!["C".into()],
                "@MMK_R_CGroup".into() => vec!["C".into()],
                "DGroup".into() => vec!["D".into()],
                "public.kern1.BGroup".into() => vec!["B".into()],
                "public.kern1.CGroup".into() => vec!["C".into()],
                "public.kern2.CGroup".into() => vec!["C".into()],
                "public.kern2.DGroup".into() => vec!["D".into()],
            }
        );
        assert_eq!(
            kerning_new,
            btreemap! {
                "A".into() => btreemap!{
                    "A".into() => 1.0,
                    "B".into() => 2.0,
                    "public.kern2.CGroup".into() => 3.0,
                    "public.kern2.DGroup".into() => 4.0,
                },
                "public.kern1.BGroup".into() => btreemap!{
                    "A".into() => 5.0,
                    "B".into() => 6.0,
                    "public.kern2.CGroup".into() => 7.0,
                    "public.kern2.DGroup".into() => 8.0,
                },
                "public.kern1.CGroup".into() => btreemap!{
                    "A".into() => 9.0,
                    "B".into() => 10.0,
                    "public.kern2.CGroup".into() => 11.0,
                    "public.kern2.DGroup".into() => 12.0,
                }
            }
        );
    }

    #[test]
    fn test_upconvert_kerning_glyphname_groupname() {
        let ufo_v1 =
            Ufo::load("testdata/upconversion_kerning/glyphname_groupname_UFOv1.ufo").unwrap();
        let ufo_v2 =
            Ufo::load("testdata/upconversion_kerning/glyphname_groupname_UFOv2.ufo").unwrap();

        let groups_expected: Groups = plist::from_file(
            "testdata/upconversion_kerning/glyphname_groupname_groups_expected.plist",
        )
        .unwrap();
        let kerning_expected: Kerning = plist::from_file(
            "testdata/upconversion_kerning/glyphname_groupname_kerning_expected.plist",
        )
        .unwrap();

        assert_eq!(ufo_v1.meta.format_version, FormatVersion::V3);
        assert_eq!(ufo_v2.meta.format_version, FormatVersion::V3);
        assert_eq!(ufo_v1.groups.unwrap(), groups_expected);
        assert_eq!(ufo_v2.groups.unwrap(), groups_expected);
        assert_eq!(ufo_v1.kerning.unwrap(), kerning_expected);
        assert_eq!(ufo_v2.kerning.unwrap(), kerning_expected);
    }
}
