//! Reading and writing designspace files.

#![deny(rustdoc::broken_intra_doc_links)]

use serde::Serialize;
use std::{fs::File, io::BufReader, path::Path};

use plist::Dictionary;

use crate::error::{DesignSpaceLoadError, DesignSpaceSaveError};
use crate::serde_xml_plist as serde_plist;
use crate::Name;

/// A [designspace].
///
/// [designspace]: https://fonttools.readthedocs.io/en/latest/designspaceLib/index.html
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "designspace")]
pub struct DesignSpaceDocument {
    /// Design space format version.
    #[serde(rename = "@format")]
    pub format: f32,
    /// One or more axes.
    #[serde(with = "serde_impls::axes", skip_serializing_if = "Vec::is_empty")]
    pub axes: Vec<Axis>,
    /// One or more rules.
    #[serde(default, skip_serializing_if = "Rules::is_empty")]
    pub rules: Rules,
    /// One or more sources.
    #[serde(with = "serde_impls::sources", skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Source>,
    /// One or more instances.
    #[serde(default, with = "serde_impls::instances", skip_serializing_if = "Vec::is_empty")]
    pub instances: Vec<Instance>,
    /// Additional arbitrary user data
    #[serde(default, with = "serde_plist", skip_serializing_if = "Dictionary::is_empty")]
    pub lib: Dictionary,
}

/// An [axis].
///
/// [axis]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#axis-element
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "axis")]
pub struct Axis {
    /// Name of the axis that is used in the location elements.
    #[serde(rename = "@name")]
    pub name: String,
    /// 4 letters. Some axis tags are registered in the OpenType Specification.
    #[serde(rename = "@tag")]
    pub tag: String,
    /// The default value for this axis, in user space coordinates.
    #[serde(rename = "@default")]
    pub default: f32,
    /// Records whether this axis needs to be hidden in interfaces.
    #[serde(default, rename = "@hidden", skip_serializing_if = "is_false")]
    pub hidden: bool,
    /// The minimum value for a continuous axis, in user space coordinates.
    #[serde(rename = "@minimum", skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f32>,
    /// The maximum value for a continuous axis, in user space coordinates.
    #[serde(rename = "@maximum", skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f32>,
    /// The possible values for a discrete axis, in user space coordinates.
    #[serde(rename = "@values", skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<f32>>,
    /// Mapping between user space coordinates and design space coordinates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<Vec<AxisMapping>>,
}

fn is_false(value: &bool) -> bool {
    !(*value)
}

/// Maps one input value (user space coord) to one output value (design space coord).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "map")]
pub struct AxisMapping {
    /// user space coordinate
    #[serde(rename = "@input")]
    pub input: f32,
    /// designspace coordinate
    #[serde(rename = "@output")]
    pub output: f32,
}

/// Describes the substitution [rules] of the Designspace.
///
/// [rules]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#rules-element
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rules {
    /// Indicates whether substitution rules should be applied before or after
    /// other glyph substitution features.
    #[serde(default, rename = "@processing")]
    pub processing: RuleProcessing,
    /// The rules.
    #[serde(default, rename = "rule")]
    pub rules: Vec<Rule>,
}

/// Indicates whether substitution rules should be applied before or after other
/// glyph substitution features.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleProcessing {
    /// Apply before other substitution features.
    #[default]
    First,
    /// Apply after other substitution features.
    Last,
}

/// Describes a single set of substitution rules.
///
/// Does not support standalone `<condition>` elements outside a
/// `<conditionset>`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    /// Name of the rule.
    #[serde(rename = "@name")]
    pub name: Option<String>,
    /// Condition sets. If any condition is true or the condition set is empty,
    /// the rule is applied.
    #[serde(rename = "conditionset")]
    pub condition_sets: Vec<ConditionSet>,
    /// Subtitutions (in, out).
    #[serde(rename = "sub")]
    pub substitutions: Vec<Substitution>,
}

/// Describes a single substitution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Substitution {
    /// Substitute this glyph...
    #[serde(rename = "@name")]
    pub name: Name,
    /// ...with this one.
    #[serde(rename = "@with")]
    pub with: Name,
}

/// Describes a set of conditions that must all be met for the rule to apply.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConditionSet {
    /// The conditions.
    #[serde(rename = "condition", default)]
    pub conditions: Vec<Condition>,
}

/// Describes a single condition.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    /// The name of the axis.
    #[serde(rename = "@name")]
    pub name: String,
    /// Lower bounds in design space coordinates.
    #[serde(rename = "@minimum", default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f32>,
    /// Upper bounds in design space coordinates.
    #[serde(rename = "@maximum", default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f32>,
}

/// A [source].
///
/// [source]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#id25
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "source")]
pub struct Source {
    /// The family name of the source font.
    #[serde(rename = "@familyname", skip_serializing_if = "Option::is_none")]
    pub familyname: Option<String>,
    /// The style name of the source font.
    #[serde(rename = "@stylename", skip_serializing_if = "Option::is_none")]
    pub stylename: Option<String>,
    /// A unique name that can be used to identify this font if it needs to be referenced elsewhere.
    #[serde(rename = "@name")]
    pub name: Option<String>,
    /// A path to the source file, relative to the root path of this document.
    ///
    /// The path can be at the same level as the document or lower.
    #[serde(rename = "@filename")]
    pub filename: String,
    /// The name of the layer in the source file.
    ///
    /// If no layer attribute is given assume the foreground layer should be used.
    #[serde(rename = "@layer", skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    /// Location in designspace coordinates.
    #[serde(with = "serde_impls::location")]
    pub location: Vec<Dimension>,
}

/// An [instance].
///
/// [instance]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#instance-element
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "instance")]
pub struct Instance {
    // per @anthrotype, contrary to spec, filename, familyname and stylename are optional
    /// The family name of the instance font. Corresponds with font.info.familyName
    #[serde(rename = "@familyname", skip_serializing_if = "Option::is_none")]
    pub familyname: Option<String>,
    /// The style name of the instance font. Corresponds with font.info.styleName
    #[serde(rename = "@stylename", skip_serializing_if = "Option::is_none")]
    pub stylename: Option<String>,
    /// A unique name that can be used to identify this font if it needs to be referenced elsewhere.
    #[serde(rename = "@name")]
    pub name: Option<String>,
    /// A path to the instance file, relative to the root path of this document. The path can be at the same level as the document or lower.
    #[serde(rename = "@filename", skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Corresponds with font.info.postscriptFontName
    #[serde(rename = "@postscriptfontname", skip_serializing_if = "Option::is_none")]
    pub postscriptfontname: Option<String>,
    /// Corresponds with styleMapFamilyName
    #[serde(rename = "@stylemapfamilyname", skip_serializing_if = "Option::is_none")]
    pub stylemapfamilyname: Option<String>,
    /// Corresponds with styleMapStyleName
    #[serde(rename = "@stylemapstylename", skip_serializing_if = "Option::is_none")]
    pub stylemapstylename: Option<String>,
    /// Location in designspace.
    #[serde(with = "serde_impls::location")]
    pub location: Vec<Dimension>,
    /// Arbitrary data about this instance
    #[serde(default, with = "serde_plist", skip_serializing_if = "Dictionary::is_empty")]
    pub lib: Dictionary,
}

/// A design space dimension.
///
/// [design space location]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#location-element-source
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "dimension")]
pub struct Dimension {
    /// Name of the axis, e.g. Weight.
    #[serde(rename = "@name")]
    pub name: String,
    /// Value on the axis in user coordinates.
    #[serde(rename = "@uservalue", skip_serializing_if = "Option::is_none")]
    pub uservalue: Option<f32>,
    /// Value on the axis in designcoordinates.
    #[serde(rename = "@xvalue", skip_serializing_if = "Option::is_none")]
    pub xvalue: Option<f32>,
    /// Separate value for anisotropic interpolations.
    #[serde(rename = "@yvalue", skip_serializing_if = "Option::is_none")]
    pub yvalue: Option<f32>,
}

impl DesignSpaceDocument {
    /// Load a designspace.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<DesignSpaceDocument, DesignSpaceLoadError> {
        let reader = BufReader::new(File::open(path).map_err(DesignSpaceLoadError::Io)?);
        quick_xml::de::from_reader(reader).map_err(DesignSpaceLoadError::DeError)
    }

    /// Save a designspace.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), DesignSpaceSaveError> {
        let mut buf = String::from("<?xml version='1.0' encoding='UTF-8'?>\n");
        let mut xml_writer = quick_xml::se::Serializer::new(&mut buf);
        xml_writer.indent(' ', 2);
        self.serialize(xml_writer)?;
        buf.push('\n'); // trailing newline
        close_already::fs::write(path, buf)?;
        Ok(())
    }
}

impl Rules {
    /// Returns `true` if there are no rules.
    fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

mod serde_impls {
    /// Produces a self-contained module to (de)serialise an XML list of a given type
    ///
    /// Example usage:
    /// ```ignore
    /// # use serde::{Serialize, Deserialize};
    ///
    /// // In XML, the locations are referred to as <dimension/>
    /// serde_from_field!(locations, dimension, Dimension);
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct DesignSpaceDocument {
    ///     #[serde(with = "locations")]
    ///     location: Vec<Dimension>,
    /// }
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct Dimension;
    /// ```
    ///
    /// the generated code is approximately:
    /// ```ignore
    /// pub(super) mod locations {
    ///     # use serde::{Deserialize, Deserializer, Serializer, Serialize};
    ///     # use norad::designspace::Dimension;
    ///     pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Dimension>, D::Error>
    ///     where
    ///         D: Deserializer<'de>,
    ///     {
    ///         #[derive(Deserialize)]
    ///         struct Helper {
    ///             dimension: Vec<Dimension>,
    ///         }
    ///         Helper::deserialize(deserializer).map(|x| x.dimension)
    ///     }
    ///
    ///     pub(crate) fn serialize<S>(
    ///         dimension: &[Dimension],
    ///         serializer: S,
    ///     ) -> Result<S::Ok, S::Error>
    ///     where
    ///         S: Serializer,
    ///     {
    ///         #[derive(Serialize)]
    ///         struct Helper<'a> {
    ///             dimension: &'a [Dimension],
    ///         }
    ///         let helper = Helper { dimension };
    ///         helper.serialize(serializer)
    ///     }
    /// }
    /// ```
    macro_rules! serde_from_field {
        ($mod_name:ident, $field_name:ident, $inner:path) => {
            pub(super) mod $mod_name {
                pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Vec<$inner>, D::Error>
                where
                    D: ::serde::Deserializer<'de>,
                {
                    use ::serde::Deserialize as _;
                    #[derive(::serde::Deserialize)]
                    struct Helper {
                        $field_name: Vec<$inner>,
                    }
                    Helper::deserialize(deserializer).map(|x| x.$field_name)
                }

                pub(crate) fn serialize<S>(
                    $field_name: &[$inner],
                    serializer: S,
                ) -> Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer,
                {
                    use ::serde::Serialize as _;
                    #[derive(::serde::Serialize)]
                    struct Helper<'a> {
                        $field_name: &'a [$inner],
                    }
                    let helper = Helper { $field_name };
                    helper.serialize(serializer)
                }
            }
        };
    }

    serde_from_field!(location, dimension, crate::designspace::Dimension);
    serde_from_field!(instances, instance, crate::designspace::Instance);
    serde_from_field!(axes, axis, crate::designspace::Axis);
    serde_from_field!(sources, source, crate::designspace::Source);
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use plist::Value;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    use crate::designspace::{AxisMapping, Dimension};

    use super::*;

    fn dim_name_xvalue(name: &str, xvalue: f32) -> Dimension {
        Dimension { name: name.to_string(), uservalue: None, xvalue: Some(xvalue), yvalue: None }
    }

    #[test]
    fn read_single_wght() {
        let ds = DesignSpaceDocument::load(Path::new("testdata/single_wght.designspace")).unwrap();
        assert_eq!(1, ds.axes.len());
        let axis = &ds.axes[0];
        assert_eq!(axis.minimum, Some(400.));
        assert_eq!(axis.maximum, Some(600.));
        assert_eq!(axis.default, 500.);
        assert_eq!(
            &vec![AxisMapping { input: 400., output: 100. }],
            ds.axes[0].map.as_ref().unwrap()
        );
        assert_eq!(1, ds.sources.len());
        let weight_100 = dim_name_xvalue("Weight", 100.);
        assert_eq!(vec![weight_100.clone()], ds.sources[0].location);
        assert_eq!(1, ds.instances.len());
        assert_eq!(vec![weight_100], ds.instances[0].location);
    }

    #[test]
    fn read_wght_variable() {
        let ds = DesignSpaceDocument::load("testdata/wght.designspace").unwrap();
        assert_eq!(1, ds.axes.len());
        assert!(ds.axes[0].map.is_none());
        assert_eq!(
            vec![
                ("TestFamily-Regular.ufo".to_string(), vec![dim_name_xvalue("Weight", 400.)]),
                ("TestFamily-Bold.ufo".to_string(), vec![dim_name_xvalue("Weight", 700.)]),
            ],
            ds.sources
                .into_iter()
                .map(|s| (s.filename, s.location))
                .collect::<Vec<(String, Vec<Dimension>)>>()
        );
    }

    // <https://github.com/linebender/norad/issues/300>
    #[test]
    fn load_with_no_instances() {
        DesignSpaceDocument::load("testdata/no_instances.designspace").unwrap();
    }

    #[test]
    fn load_with_no_source_name() {
        let ds = DesignSpaceDocument::load("testdata/optional_source_names.designspace").unwrap();
        assert!(ds.sources[0].name.is_none());
        assert_eq!(ds.sources[1].name.as_deref(), Some("Test Family Bold"));
    }

    #[test]
    fn load_with_no_instance_name() {
        let ds = DesignSpaceDocument::load("testdata/optional_instance_names.designspace").unwrap();
        assert_eq!(ds.instances[0].name.as_deref(), Some("Test Family Regular"));
        assert!(ds.instances[1].name.is_none());
    }

    #[test]
    fn load_lib() {
        let loaded = DesignSpaceDocument::load("testdata/wght.designspace").unwrap();
        assert_eq!(
            loaded.lib.get("org.linebender.hasLoadedLibCorrectly"),
            Some(&Value::String("Absolutely!".into()))
        );

        let params = loaded.instances[0]
            .lib
            .get("com.schriftgestaltung.customParameters")
            .and_then(Value::as_array)
            .unwrap();
        assert_eq!(params[0].as_array().unwrap()[0].as_string(), Some("xHeight"));
        assert_eq!(params[0].as_array().unwrap()[1].as_string(), Some("536"));
        assert_eq!(
            params[1].as_array().unwrap()[1].as_array().unwrap()[0].as_unsigned_integer(),
            Some(2)
        );
    }

    #[test]
    fn do_not_serialize_empty_lib() {
        let ds_initial = DesignSpaceDocument::load("testdata/single_wght.designspace").unwrap();
        let serialized = quick_xml::se::to_string(&ds_initial).expect("should serialize");

        assert!(!serialized.contains("<lib>"));
        assert!(!serialized.contains("<lib/>"));
    }

    #[test]
    fn load_save_round_trip() {
        // Given
        let dir = TempDir::new().unwrap();
        let ds_test_save_location = dir.path().join("wght.designspace");

        // When
        let ds_initial = DesignSpaceDocument::load("testdata/wght.designspace").unwrap();
        ds_initial.save(&ds_test_save_location).expect("failed to save designspace");
        let ds_after = DesignSpaceDocument::load(ds_test_save_location)
            .expect("failed to load saved designspace");

        // Then
        assert_eq!(ds_initial, ds_after);
    }

    #[test]
    fn load_save_round_trip_mutatorsans() {
        // Given
        let dir = TempDir::new().unwrap();
        let ds_test_save_location = dir.path().join("MutatorSans.designspace");

        // When
        let ds_initial = DesignSpaceDocument::load("testdata/MutatorSans.designspace").unwrap();
        ds_initial.save(&ds_test_save_location).expect("failed to save designspace");
        let ds_after = DesignSpaceDocument::load(ds_test_save_location)
            .expect("failed to load saved designspace");

        // Then
        assert_eq!(
            &ds_after.rules,
            &Rules {
                processing: RuleProcessing::Last,
                rules: vec![
                    Rule {
                        name: Some("fold_I_serifs".into()),
                        condition_sets: vec![ConditionSet {
                            conditions: vec![Condition {
                                name: "width".into(),
                                minimum: Some(0.0),
                                maximum: Some(328.0),
                            }],
                        }],
                        substitutions: vec![Substitution {
                            name: "I".into(),
                            with: "I.narrow".into()
                        }],
                    },
                    Rule {
                        name: Some("fold_S_terminals".into()),
                        condition_sets: vec![ConditionSet {
                            conditions: vec![
                                Condition {
                                    name: "width".into(),
                                    minimum: Some(0.0),
                                    maximum: Some(1000.0),
                                },
                                Condition {
                                    name: "weight".into(),
                                    minimum: Some(0.0),
                                    maximum: Some(500.0),
                                },
                            ],
                        }],
                        substitutions: vec![Substitution {
                            name: "S".into(),
                            with: "S.closed".into()
                        }],
                    },
                ]
            }
        );
        assert_eq!(ds_initial, ds_after);
    }

    #[test]
    fn accept_always_on_rules() {
        // Given
        let designspace =
            DesignSpaceDocument::load("testdata/MutatorSansAlwaysOnRules.designspace").unwrap();

        // Then
        assert_eq!(
            &designspace.rules,
            &Rules {
                processing: RuleProcessing::Last,
                rules: vec![
                    Rule {
                        name: Some("fold_I_serifs".into()),
                        condition_sets: vec![ConditionSet { conditions: vec![] }],
                        substitutions: vec![Substitution {
                            name: "I".into(),
                            with: "I.narrow".into()
                        }],
                    },
                    Rule {
                        name: Some("fold_S_terminals".into()),
                        condition_sets: vec![ConditionSet { conditions: vec![] }],
                        substitutions: vec![Substitution {
                            name: "S".into(),
                            with: "S.closed".into()
                        }],
                    },
                ]
            }
        );
    }
}
