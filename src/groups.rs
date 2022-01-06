use std::collections::{BTreeMap, HashSet};

use crate::error::GroupsValidationError;
use crate::Name;

/// A map of group name to a list of glyph names.
///
/// We use a [`BTreeMap`] because we need sorting for serialization.
pub type Groups = BTreeMap<String, Vec<Name>>;

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
                        glyph_name: glyph_name.to_string(),
                        group_name: group_name.to_string(),
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
                        glyph_name: glyph_name.to_string(),
                        group_name: group_name.to_string(),
                    });
                }
            }
        }
    }

    Ok(())
}
