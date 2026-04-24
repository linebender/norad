use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use crate::error::{FontLoadError, GroupsValidationError};
use crate::Name;

/// A map of group name to a list of glyph names.
///
/// We use a [`BTreeMap`] because we need sorting for serialization.
pub type Groups = BTreeMap<Name, Vec<Name>>;

pub(crate) fn deserialize_groups<P: AsRef<Path>>(path: P) -> Result<Groups, FontLoadError> {
    struct GroupsDeHelper(Groups);

    impl<'de> serde::Deserialize<'de> for GroupsDeHelper {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            // Values decoded as Vec<String> so Name validation doesn't reject empty entries;
            // we filter them out and parse the rest, using serde's error type throughout.
            BTreeMap::<Name, Vec<String>>::deserialize(deserializer)?
                .into_iter()
                .map(|(k, v)| {
                    let members = v
                        .into_iter()
                        .filter(|s| !s.is_empty())
                        .map(|s| s.parse::<Name>().map_err(serde::de::Error::custom))
                        .collect::<Result<_, _>>()?;
                    Ok((k, members))
                })
                .collect::<Result<Groups, _>>()
                .map(GroupsDeHelper)
        }
    }

    plist::from_file::<_, GroupsDeHelper>(path.as_ref())
        .map(|h| h.0)
        .map_err(|source| FontLoadError::ParsePlist { name: "groups.plist", source })
}

/// Validate the contents of the groups.plist file according to the rules in the
/// [Unified Font Object v3 specification for groups.plist](http://unifiedfontobject.org/versions/ufo3/groups.plist/#specification).
pub(crate) fn validate_groups(groups_map: &Groups) -> Result<(), GroupsValidationError> {
    let mut kern1_set = HashSet::new();
    let mut kern2_set = HashSet::new();
    for (group_name, group_glyph_names) in groups_map {
        if group_name.is_empty() {
            return Err(GroupsValidationError::InvalidName);
        }

        if group_name.starts_with("public.kern1.") {
            if group_name.len() == 13 {
                // Prefix but no actual name.
                return Err(GroupsValidationError::InvalidName);
            }
            for glyph_name in group_glyph_names {
                if !kern1_set.insert(glyph_name) {
                    return Err(GroupsValidationError::OverlappingKerningGroups {
                        glyph_name: glyph_name.clone(),
                        group_name: group_name.clone(),
                    });
                }
            }
        } else if group_name.starts_with("public.kern2.") {
            if group_name.len() == 13 {
                // Prefix but no actual name.
                return Err(GroupsValidationError::InvalidName);
            }
            for glyph_name in group_glyph_names {
                if !kern2_set.insert(glyph_name) {
                    return Err(GroupsValidationError::OverlappingKerningGroups {
                        glyph_name: glyph_name.clone(),
                        group_name: group_name.clone(),
                    });
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_skip_empty_group_member() {
        let group = deserialize_groups("testdata/groups_empty_entries.plist").unwrap();
        assert_eq!(group.get("derpy_group").unwrap()[0], "hi");
    }
}
