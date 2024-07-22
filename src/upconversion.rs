use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde::Deserialize;

use crate::error::FontLoadError;
use crate::font::LIB_FILE;
use crate::fontinfo::FontInfo;
use crate::groups::Groups;
use crate::kerning::Kerning;
use crate::names::NameList;
use crate::Name;

/// Convert kerning groups and pairs from v1 and v2 informal conventions to
/// v3 formal conventions. Converted groups are added (duplicated) rather than
/// replacing the old ones to preserve all data that external entities might
/// rely on. Kerning pairs are updated to reflect the new group names.
///
/// This is an adaptation from the fontTools.ufoLib reference implementation.
/// It will not check if the upgraded groups pass validation.
pub(crate) fn upconvert_kerning(
    groups: &Groups,
    kerning: &Kerning,
    glyph_set: &NameList,
) -> (Groups, Kerning) {
    // Gather known kerning groups based on the prefixes. This will catch groups that exist in
    // `groups` but are not referenced in `kerning`.
    let (mut groups_first, mut groups_second) = find_known_kerning_groups(groups);

    // Make lists of groups referenced in kerning pairs, based on their side.
    for (first, seconds) in kerning {
        if groups.contains_key(first)
            && !glyph_set.contains(first)
            && !first.starts_with("public.kern1.")
        {
            groups_first.insert(first.clone());
        }
        for second in seconds.keys() {
            if groups.contains_key(second)
                && !glyph_set.contains(second)
                && !second.starts_with("public.kern2.")
            {
                groups_second.insert(second.clone());
            }
        }
    }

    // Duplicate kerning groups with a new name.
    let mut groups_new = groups.clone();

    let mut groups_first_old_to_new: HashMap<Name, Name> = HashMap::new();
    for first in &groups_first {
        let first_new = make_unique_group_name(
            Name::new(&format!("public.kern1.{}", first.replace("@MMK_L_", ""))).unwrap(),
            &groups_new,
        );
        groups_first_old_to_new.insert(first.clone(), first_new.clone());
        groups_new.insert(first_new, groups_new.get(first).unwrap().clone());
    }
    let mut groups_second_old_to_new: HashMap<Name, Name> = HashMap::new();
    for second in &groups_second {
        let second_new = make_unique_group_name(
            Name::new(&format!("public.kern2.{}", second.replace("@MMK_R_", ""))).unwrap(),
            &groups_new,
        );
        groups_second_old_to_new.insert(second.clone(), second_new.clone());
        groups_new.insert(second_new, groups_new.get(second).unwrap().clone());
    }

    // Update all kerning pairs that have an old kerning group in them with the new name.
    let mut kerning_new: Kerning = Kerning::new();

    for (first, seconds) in kerning {
        let first_new = groups_first_old_to_new.get(first).unwrap_or(first);
        let mut seconds_new: BTreeMap<Name, f64> = BTreeMap::new();
        for (second, value) in seconds {
            let second_new = groups_second_old_to_new.get(second).unwrap_or(second);
            seconds_new.insert(second_new.clone(), *value);
        }
        kerning_new.insert(first_new.clone(), seconds_new);
    }

    (groups_new, kerning_new)
}

fn make_unique_group_name(name: Name, existing_groups: &Groups) -> Name {
    if !existing_groups.contains_key(&name) {
        return name;
    }

    let mut counter = 1;
    let mut new_name = name.clone();
    while existing_groups.contains_key(&new_name) {
        new_name = Name::new(&format!("{}{}", name, counter)).unwrap();
        counter += 1;
    }

    new_name
}

fn find_known_kerning_groups(groups: &Groups) -> (HashSet<Name>, HashSet<Name>) {
    let mut groups_first: HashSet<Name> = HashSet::new();
    let mut groups_second: HashSet<Name> = HashSet::new();

    for name in groups.keys() {
        if name.starts_with("@MMK_L_") {
            groups_first.insert(name.clone());
        } else if name.starts_with("@MMK_R_") {
            groups_second.insert(name.clone());
        }
    }

    (groups_first, groups_second)
}

/// Migrate UFO v1 era feature and PostScript hinting data to the current data model. It re-reads
/// the lib.plist file to filter out the relevant data and then update the passed in lib, features
/// and fontinfo in-place. It tries to follow what [defcon is doing][1].
///
/// [1]: https://github.com/robotools/defcon/blob/76a7ac408e62f68c09eaf24ca6d9ad04523dd19c/Lib/defcon/objects/font.py#L1571-L1629
pub(crate) fn upconvert_ufov1_robofab_data(
    lib_path: &Path,
    lib: &mut plist::Dictionary,
    font_info: &mut FontInfo,
) -> Result<Option<String>, FontLoadError> {
    #[derive(Debug, Deserialize)]
    struct LibData {
        #[serde(rename = "org.robofab.postScriptHintData")]
        ps_hinting_data: Option<PsHintingData>,

        #[serde(rename = "org.robofab.opentype.classes")]
        feature_classes: Option<String>,
        #[serde(rename = "org.robofab.opentype.featureorder")]
        feature_order: Option<Vec<String>>,
        #[serde(rename = "org.robofab.opentype.features")]
        features: Option<HashMap<String, String>>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PsHintingData {
        blue_fuzz: Option<f64>,
        blue_scale: Option<f64>,
        blue_shift: Option<f64>,
        blue_values: Option<Vec<Vec<f64>>>,
        family_blues: Option<Vec<Vec<f64>>>,
        family_other_blues: Option<Vec<Vec<f64>>>,
        force_bold: Option<bool>,
        other_blues: Option<Vec<Vec<f64>>>,
        h_stems: Option<Vec<f64>>,
        v_stems: Option<Vec<f64>>,
    }

    // Read lib.plist again because it is easier than pulling out the data manually.
    let lib_data: LibData = plist::from_file(lib_path)
        .map_err(|source| FontLoadError::ParsePlist { name: LIB_FILE, source })?;

    // Convert features.
    let mut features = String::new();

    if let Some(feature_classes) = lib_data.feature_classes {
        features.push_str(&feature_classes);
    }

    if let Some(features_split) = lib_data.features {
        let order: Vec<String> = if let Some(feature_order) = lib_data.feature_order {
            feature_order
        } else {
            features_split.keys().cloned().collect::<Vec<String>>()
        };

        features.push('\n');

        for key in order {
            // Ignore non-existent keys because defcon does it, too.
            if let Some(txt) = features_split.get(&key) {
                features.push_str(txt);
            }
        }
    }

    // Convert PostScript hinting data.
    if let Some(ps_hinting_data) = lib_data.ps_hinting_data {
        font_info.postscript_blue_fuzz = ps_hinting_data.blue_fuzz;
        font_info.postscript_blue_scale = ps_hinting_data.blue_scale;
        font_info.postscript_blue_shift = ps_hinting_data.blue_shift;
        if let Some(blue_values) = ps_hinting_data.blue_values {
            font_info.postscript_blue_values = Some(blue_values.into_iter().flatten().collect());
        };
        if let Some(other_blues) = ps_hinting_data.other_blues {
            font_info.postscript_other_blues = Some(other_blues.into_iter().flatten().collect());
        };
        if let Some(family_blues) = ps_hinting_data.family_blues {
            font_info.postscript_family_blues = Some(family_blues.into_iter().flatten().collect());
        };
        if let Some(family_other_blues) = ps_hinting_data.family_other_blues {
            font_info.postscript_family_other_blues =
                Some(family_other_blues.into_iter().flatten().collect());
        };
        font_info.postscript_force_bold = ps_hinting_data.force_bold;
        font_info.postscript_stem_snap_h = ps_hinting_data.h_stems;
        font_info.postscript_stem_snap_v = ps_hinting_data.v_stems;

        font_info.validate().map_err(FontLoadError::FontInfoV1Upconversion)?;
    }

    lib.remove("org.robofab.postScriptHintData");
    lib.remove("org.robofab.opentype.classes");
    lib.remove("org.robofab.opentype.featureorder");
    lib.remove("org.robofab.opentype.features");

    if features.is_empty() {
        Ok(None)
    } else {
        Ok(Some(features))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        font::{Font, FormatVersion},
        Name,
    };
    use maplit::btreemap;

    // we don't want this in the crate because it can fail, but it is useful
    // for creating test data.
    impl<'a> From<&'a str> for Name {
        fn from(src: &str) -> Self {
            Name::new_raw(src)
        }
    }

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
        let glyph_set: NameList = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]
            .iter()
            .cloned()
            .map(Name::from)
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
        let glyph_set = NameList::default();

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
        let glyph_set = NameList::default();

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
        let glyph_set = NameList::default();

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
            Font::load("testdata/upconversion_kerning/glyphname_groupname_UFOv1.ufo").unwrap();
        let ufo_v2 =
            Font::load("testdata/upconversion_kerning/glyphname_groupname_UFOv2.ufo").unwrap();

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
        assert_eq!(ufo_v1.groups, groups_expected);
        assert_eq!(ufo_v2.groups, groups_expected);
        assert_eq!(ufo_v1.kerning, kerning_expected);
        assert_eq!(ufo_v2.kerning, kerning_expected);
    }
}
