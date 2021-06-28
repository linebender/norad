use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use crate::font::{Groups, Kerning};
use crate::fontinfo::FontInfo;
use crate::names::NameList;
use crate::shared_types::IntegerOrFloat;
use crate::Error;

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

/// Migrate UFO v1 era feature and PostScript hinting data to the current data model. It re-reads
/// the lib.plist file to filter out the relevant data and then update the passed in lib, features
/// and fontinfo in-place. It tries to follow what [defcon is doing][1].
///
/// [1]: https://github.com/robotools/defcon/blob/76a7ac408e62f68c09eaf24ca6d9ad04523dd19c/Lib/defcon/objects/font.py#L1571-L1629
pub(crate) fn upconvert_ufov1_robofab_data(
    lib_path: &Path,
    lib: &mut plist::Dictionary,
    fontinfo: &mut FontInfo,
) -> Result<Option<String>, Error> {
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
        blue_fuzz: Option<IntegerOrFloat>,
        blue_scale: Option<f64>,
        blue_shift: Option<IntegerOrFloat>,
        blue_values: Option<Vec<Vec<IntegerOrFloat>>>,
        family_blues: Option<Vec<Vec<IntegerOrFloat>>>,
        family_other_blues: Option<Vec<Vec<IntegerOrFloat>>>,
        force_bold: Option<bool>,
        other_blues: Option<Vec<Vec<IntegerOrFloat>>>,
        h_stems: Option<Vec<IntegerOrFloat>>,
        v_stems: Option<Vec<IntegerOrFloat>>,
    }

    // Read lib.plist again because it is easier than pulling out the data manually.
    let lib_data: LibData = plist::from_file(lib_path)?;

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

        features.push_str(&"\n");

        for key in order {
            // Ignore non-existant keys because defcon does it, too.
            if let Some(txt) = features_split.get(&key) {
                features.push_str(&txt);
            }
        }
    }

    // Convert PostScript hinting data.
    if let Some(ps_hinting_data) = lib_data.ps_hinting_data {
        fontinfo.postscript_blue_fuzz = ps_hinting_data.blue_fuzz;
        fontinfo.postscript_blue_scale = ps_hinting_data.blue_scale;
        fontinfo.postscript_blue_shift = ps_hinting_data.blue_shift;
        if let Some(blue_values) = ps_hinting_data.blue_values {
            fontinfo.postscript_blue_values = Some(blue_values.into_iter().flatten().collect());
        };
        if let Some(other_blues) = ps_hinting_data.other_blues {
            fontinfo.postscript_other_blues = Some(other_blues.into_iter().flatten().collect());
        };
        if let Some(family_blues) = ps_hinting_data.family_blues {
            fontinfo.postscript_family_blues = Some(family_blues.into_iter().flatten().collect());
        };
        if let Some(family_other_blues) = ps_hinting_data.family_other_blues {
            fontinfo.postscript_family_other_blues =
                Some(family_other_blues.into_iter().flatten().collect());
        };
        fontinfo.postscript_force_bold = ps_hinting_data.force_bold;
        fontinfo.postscript_stem_snap_h = ps_hinting_data.h_stems;
        fontinfo.postscript_stem_snap_v = ps_hinting_data.v_stems;

        fontinfo.validate().map_err(|_| Error::FontInfoUpconversion)?;
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
    extern crate maplit;

    use super::*;
    use crate::font::{Font, FormatVersion};
    use crate::glyph::GlyphName;
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
        let glyph_set: NameList = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]
            .iter()
            .map(|s| GlyphName::from(*s))
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
        assert_eq!(ufo_v1.groups.unwrap(), groups_expected);
        assert_eq!(ufo_v2.groups.unwrap(), groups_expected);
        assert_eq!(ufo_v1.kerning.unwrap(), kerning_expected);
        assert_eq!(ufo_v2.kerning.unwrap(), kerning_expected);
    }
}
