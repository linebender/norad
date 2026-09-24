//! Reading and writing designspace files.

#![deny(rustdoc::broken_intra_doc_links)]

use serde::{Deserialize, Serialize};
use std::path::Path;

use plist::Dictionary;

use crate::error::{DesignSpaceLoadError, DesignSpaceSaveError};
use crate::serde_xml_plist as serde_plist;
use crate::Name;

/// A [designspace].
///
/// [designspace]: https://fonttools.readthedocs.io/en/latest/designspaceLib/index.html
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "RawDesignSpaceDocument", into = "RawDesignSpaceDocument")]
pub struct DesignSpaceDocument {
    /// Design space format version.
    ///
    /// This is the version that was declared by the document that was loaded.
    /// When saving, the *minimum* format version required to represent the
    /// contents of this document is written out instead, matching the
    /// behaviour of fontTools.
    pub format: f32,
    /// One or more axes.
    pub axes: Vec<Axis>,
    /// Optional avar2-style axis mappings.
    pub axis_mappings: Option<AxisMappings>,
    /// The style name to use when all STAT labels of a location are elided.
    ///
    /// This is the `elidedfallbackname` attribute of the `<axes>` element.
    pub elided_fallback_name: Option<String>,
    /// Labels for freestanding locations, the analogue of STAT format 4 entries.
    pub location_labels: Vec<LocationLabel>,
    /// One or more rules.
    pub rules: Rules,
    /// One or more sources.
    pub sources: Vec<Source>,
    /// The variable fonts that can be built from this designspace.
    ///
    /// If this is empty, the whole designspace describes a single output.
    pub variable_fonts: Vec<VariableFont>,
    /// One or more instances.
    pub instances: Vec<Instance>,
    /// Additional arbitrary user data
    pub lib: Dictionary,
}

/// An [axis].
///
/// [axis]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#axis-element
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "RawAxis", into = "RawAxis")]
pub struct Axis {
    /// Name of the axis that is used in the location elements.
    pub name: String,
    /// 4 letters. Some axis tags are registered in the OpenType Specification.
    pub tag: String,
    /// The default value for this axis, in user space coordinates.
    pub default: f64,
    /// Records whether this axis needs to be hidden in interfaces.
    pub hidden: bool,
    /// The minimum value for a continuous axis, in user space coordinates.
    pub minimum: Option<f64>,
    /// The maximum value for a continuous axis, in user space coordinates.
    pub maximum: Option<f64>,
    /// The possible values for a discrete axis, in user space coordinates.
    pub values: Option<Vec<f64>>,
    /// Mapping between user space coordinates and design space coordinates.
    pub map: Option<Vec<AxisMapping>>,
    /// Localised UI strings for the axis name.
    pub label_names: Vec<LocalizedString>,
    /// The STAT ordering of this axis (`<labels ordering=..>`).
    pub axis_ordering: Option<u32>,
    /// The STAT labels for this axis.
    pub axis_labels: Vec<AxisLabel>,
}

/// Internal struct matching the XML structure of an `<axis>` element.
///
/// [`Axis`] hoists the contents of the nested `<labels>` element up into the
/// axis itself; this type mirrors the on-disk nesting.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "axis")]
struct RawAxis {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@tag")]
    tag: String,
    #[serde(rename = "@default")]
    default: f64,
    #[serde(default, rename = "@hidden", skip_serializing_if = "is_false")]
    hidden: bool,
    #[serde(rename = "@minimum", skip_serializing_if = "Option::is_none")]
    minimum: Option<f64>,
    #[serde(rename = "@maximum", skip_serializing_if = "Option::is_none")]
    maximum: Option<f64>,
    #[serde(rename = "@values", skip_serializing_if = "Option::is_none")]
    values: Option<Vec<f64>>,
    #[serde(rename = "labelname", default, skip_serializing_if = "Vec::is_empty")]
    label_names: Vec<LocalizedString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    map: Option<Vec<AxisMapping>>,
    #[serde(default, skip_serializing_if = "RawAxisLabels::is_empty")]
    labels: RawAxisLabels,
}

/// Internal struct matching the `<labels>` element of an `<axis>`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct RawAxisLabels {
    #[serde(rename = "@ordering", default, skip_serializing_if = "Option::is_none")]
    ordering: Option<u32>,
    #[serde(rename = "label", default)]
    labels: Vec<AxisLabel>,
}

impl RawAxisLabels {
    fn is_empty(&self) -> bool {
        self.ordering.is_none() && self.labels.is_empty()
    }
}

impl From<RawAxis> for Axis {
    fn from(raw: RawAxis) -> Self {
        Axis {
            name: raw.name,
            tag: raw.tag,
            default: raw.default,
            hidden: raw.hidden,
            minimum: raw.minimum,
            maximum: raw.maximum,
            values: raw.values,
            map: raw.map,
            label_names: raw.label_names,
            axis_ordering: raw.labels.ordering,
            axis_labels: raw.labels.labels,
        }
    }
}

impl From<Axis> for RawAxis {
    fn from(axis: Axis) -> Self {
        RawAxis {
            name: axis.name,
            tag: axis.tag,
            default: axis.default,
            hidden: axis.hidden,
            minimum: axis.minimum,
            maximum: axis.maximum,
            values: axis.values,
            label_names: axis.label_names,
            map: axis.map,
            labels: RawAxisLabels { ordering: axis.axis_ordering, labels: axis.axis_labels },
        }
    }
}

/// A STAT [axis label].
///
/// ```xml
/// <axis name="Weight" tag="wght" minimum="200" maximum="1000" default="400">
///   <labels ordering="1">
///     <label uservalue="400" userminimum="350" usermaximum="450" name="Regular" elidable="true">
///       <labelname xml:lang="de">Standard</labelname>
///     </label>
///     <label uservalue="700" name="Bold" linkeduservalue="400"/>
///   </labels>
/// </axis>
/// ```
///
/// [axis label]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#label-element-axis
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AxisLabel {
    /// The name of this label.
    #[serde(rename = "@name")]
    pub name: String,
    /// The value of this label, in user coordinates.
    #[serde(rename = "@uservalue")]
    pub user_value: f64,
    /// The lower end of the range this label applies to, in user coordinates.
    #[serde(rename = "@userminimum", default, skip_serializing_if = "Option::is_none")]
    pub user_minimum: Option<f64>,
    /// The upper end of the range this label applies to, in user coordinates.
    #[serde(rename = "@usermaximum", default, skip_serializing_if = "Option::is_none")]
    pub user_maximum: Option<f64>,
    /// Whether this label can be omitted from the style name.
    #[serde(rename = "@elidable", default, skip_serializing_if = "is_false")]
    pub elidable: bool,
    /// Whether this label should sort before its siblings in the STAT table.
    #[serde(rename = "@oldersibling", default, skip_serializing_if = "is_false")]
    pub older_sibling: bool,
    /// The user value of the label this one is linked to, e.g. by bold linking.
    #[serde(rename = "@linkeduservalue", default, skip_serializing_if = "Option::is_none")]
    pub linked_user_value: Option<f64>,
    /// Localised UI strings for this label's name.
    #[serde(rename = "labelname", default, skip_serializing_if = "Vec::is_empty")]
    pub label_names: Vec<LocalizedString>,
}

/// A [location label], which names a freestanding location.
///
/// These are the analogue of STAT format 4 entries.
///
/// ```xml
/// <labels>
///   <label name="Some Style" elidable="true">
///     <labelname xml:lang="fr">Un Style</labelname>
///     <location>
///       <dimension name="Weight" uservalue="300"/>
///       <dimension name="Width" uservalue="50"/>
///     </location>
///   </label>
/// </labels>
/// ```
///
/// [location label]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#label-element-top-level
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LocationLabel {
    /// The name of this label.
    #[serde(rename = "@name")]
    pub name: String,
    /// Whether this label can be omitted from the style name.
    #[serde(rename = "@elidable", default, skip_serializing_if = "is_false")]
    pub elidable: bool,
    /// Whether this label should sort before its siblings in the STAT table.
    #[serde(rename = "@oldersibling", default, skip_serializing_if = "is_false")]
    pub older_sibling: bool,
    /// Localised UI strings for this label's name.
    #[serde(rename = "labelname", default, skip_serializing_if = "Vec::is_empty")]
    pub label_names: Vec<LocalizedString>,
    /// The location this label names, in user coordinates (`uservalue`).
    #[serde(default, with = "serde_impls::location", skip_serializing_if = "Vec::is_empty")]
    pub location: Vec<Dimension>,
}

/// A [variable font] that can be built from this designspace.
///
/// ```xml
/// <variable-fonts>
///   <variable-font name="MyFont_WghtWdth" filename="MyFont[wght,wdth].ttf">
///     <axis-subsets>
///       <axis-subset name="Weight"/>
///       <axis-subset name="Width" userminimum="75" userdefault="100" usermaximum="125"/>
///       <axis-subset name="Italic" uservalue="0"/>
///     </axis-subsets>
///     <lib>
///       <dict>...</dict>
///     </lib>
///   </variable-font>
/// </variable-fonts>
/// ```
///
/// [variable font]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#variable-font-element
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VariableFont {
    /// The name of this variable font.
    #[serde(rename = "@name")]
    pub name: String,
    /// The file name of this variable font, if it differs from the name.
    #[serde(rename = "@filename", default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// The subset of the designspace's axes that this font covers.
    #[serde(
        rename = "axis-subsets",
        default,
        with = "serde_impls::axis_subsets",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub axis_subsets: Vec<AxisSubset>,
    /// Additional arbitrary user data.
    #[serde(default, with = "serde_plist", skip_serializing_if = "Dictionary::is_empty")]
    pub lib: Dictionary,
}

/// An [axis subset] of a variable font.
///
/// An `<axis-subset>` element covers either a range of the axis (by default
/// the whole axis) or a single value on it, distinguished by its attributes;
/// see [`SubsetValue`].
///
/// ```xml
/// <axis-subset name="Weight"/>
/// <axis-subset name="Width" userminimum="75" userdefault="100" usermaximum="125"/>
/// <axis-subset name="Optical" userminimum="12"/>
/// <axis-subset name="Italic" uservalue="0"/>
/// ```
///
/// [axis subset]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#axis-subset-element
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawAxisSubset", into = "RawAxisSubset")]
pub struct AxisSubset {
    /// The name of the axis being subset.
    pub name: String,
    /// How much of the axis is included in the variable font.
    pub value: SubsetValue,
}

/// The portion of an axis included in a variable font.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SubsetValue {
    /// A range of the axis, in user coordinates.
    ///
    /// This is the `userminimum`/`userdefault`/`usermaximum` form. Each value
    /// is optional, and missing values are taken from the parent axis; if all
    /// are missing, the subset is the whole axis (see [`SubsetValue::is_full`]).
    Range {
        /// The lower end of the range, in user coordinates.
        ///
        /// If missing, this is the axis minimum.
        minimum: Option<f64>,
        /// The default value, in user coordinates.
        ///
        /// If missing, this is the axis default, clamped to the range.
        default: Option<f64>,
        /// The upper end of the range, in user coordinates.
        ///
        /// If missing, this is the axis maximum.
        maximum: Option<f64>,
    },
    /// A single point on the axis, in user coordinates (`uservalue`).
    Discrete(f64),
}

impl Default for SubsetValue {
    fn default() -> Self {
        SubsetValue::Range { minimum: None, default: None, maximum: None }
    }
}

impl SubsetValue {
    /// Returns `true` if this subset covers the whole axis.
    ///
    /// This is the case when it is a [`SubsetValue::Range`] with no values
    /// set, which is written as an `<axis-subset>` with only a name.
    pub fn is_full(&self) -> bool {
        *self == SubsetValue::default()
    }
}

/// Internal struct matching the XML structure of an `<axis-subset>` element.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct RawAxisSubset {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@userminimum", default, skip_serializing_if = "Option::is_none")]
    user_minimum: Option<f64>,
    #[serde(rename = "@userdefault", default, skip_serializing_if = "Option::is_none")]
    user_default: Option<f64>,
    #[serde(rename = "@usermaximum", default, skip_serializing_if = "Option::is_none")]
    user_maximum: Option<f64>,
    #[serde(rename = "@uservalue", default, skip_serializing_if = "Option::is_none")]
    user_value: Option<f64>,
}

/// An error encountered while interpreting an `<axis-subset>` element.
#[derive(Clone, Debug, PartialEq)]
struct AxisSubsetError(&'static str);

impl std::fmt::Display for AxisSubsetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl TryFrom<RawAxisSubset> for AxisSubset {
    type Error = AxisSubsetError;

    fn try_from(raw: RawAxisSubset) -> Result<Self, Self::Error> {
        let RawAxisSubset { name, user_minimum, user_default, user_maximum, user_value } = raw;
        let value = match user_value {
            Some(user_value) => {
                if user_minimum.is_some() || user_default.is_some() || user_maximum.is_some() {
                    return Err(AxisSubsetError(
                        "axis-subset element with uservalue must not have \
                         userminimum/userdefault/usermaximum",
                    ));
                }
                SubsetValue::Discrete(user_value)
            }
            None => SubsetValue::Range {
                minimum: user_minimum,
                default: user_default,
                maximum: user_maximum,
            },
        };
        Ok(AxisSubset { name, value })
    }
}

impl From<AxisSubset> for RawAxisSubset {
    fn from(subset: AxisSubset) -> Self {
        let AxisSubset { name, value } = subset;
        match value {
            SubsetValue::Range { minimum, default, maximum } => RawAxisSubset {
                name,
                user_minimum: minimum,
                user_default: default,
                user_maximum: maximum,
                user_value: None,
            },
            SubsetValue::Discrete(user_value) => {
                RawAxisSubset { name, user_value: Some(user_value), ..Default::default() }
            }
        }
    }
}

fn is_false(value: &bool) -> bool {
    !(*value)
}

/// Localised string for UI use.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LocalizedString {
    /// Language tag, e.g. `"fa-IR"`.
    // `quick-xml` strips the `xml:` prefix, so the incoming attribute is just
    // `lang`.  We keep `xml:lang` as the primary name used when serializing, but
    // add an alias to `lang` to be used when deserializing.
    #[serde(rename = "@xml:lang", alias = "@lang")]
    pub language: String,
    /// The label name
    #[serde(rename = "$text")]
    pub string: String,
}

/// Maps one input value (user space coord) to one output value (design space coord).
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "map")]
pub struct AxisMapping {
    /// user space coordinate
    #[serde(rename = "@input")]
    pub input: f64,
    /// designspace coordinate
    #[serde(rename = "@output")]
    pub output: f64,
}

/// A group of [axis mappings] for avar2-style mappings.
///
/// [axis mappings]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#mappings-element
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AxisMappings {
    /// Optional description of this mappings group.
    #[serde(rename = "@description", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The individual axis mappings.
    #[serde(default, rename = "mapping")]
    pub mappings: Vec<AxisMappingEntry>,
}

impl AxisMappings {
    /// Returns `true` if there are no mappings.
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }
}

/// A single axis mapping entry with input and output locations.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AxisMappingEntry {
    /// Optional description of this mapping.
    #[serde(rename = "@description", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The input location in designspace coordinates.
    #[serde(with = "serde_impls::location")]
    pub input: Vec<Dimension>,
    /// The output location in designspace coordinates.
    #[serde(with = "serde_impls::location")]
    pub output: Vec<Dimension>,
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
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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
/// Supports both modern `<conditionset>` wrappers and legacy standalone
/// `<condition>` elements directly inside `<rule>`. Bare conditions are
/// collected into a single implicit condition set, matching fonttools behavior.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "RawRule", into = "RawRule")]
pub struct Rule {
    /// Name of the rule.
    pub name: Option<String>,
    /// Condition sets. If any condition set is true or is empty,
    /// the rule is applied.
    pub condition_sets: Vec<ConditionSet>,
    /// Substitutions (in, out).
    pub substitutions: Vec<Substitution>,
}

/// Internal deserialization helper that captures both `<conditionset>` and
/// bare `<condition>` children of a `<rule>` element.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct RawRule {
    #[serde(rename = "@name", skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(rename = "conditionset", default, skip_serializing_if = "Vec::is_empty")]
    condition_sets: Vec<ConditionSet>,
    /// Legacy bare conditions outside a `<conditionset>`.
    #[serde(rename = "condition", default, skip_serializing_if = "Vec::is_empty")]
    conditions: Vec<Condition>,
    #[serde(rename = "sub", default, skip_serializing_if = "Vec::is_empty")]
    substitutions: Vec<Substitution>,
}

impl From<RawRule> for Rule {
    fn from(raw: RawRule) -> Self {
        let mut condition_sets = raw.condition_sets;
        // Legacy format: bare <condition> elements outside <conditionset>
        // are wrapped into a single implicit condition set.
        if !raw.conditions.is_empty() {
            condition_sets.push(ConditionSet { conditions: raw.conditions });
        }
        Rule { name: raw.name, condition_sets, substitutions: raw.substitutions }
    }
}

impl From<Rule> for RawRule {
    fn from(rule: Rule) -> Self {
        RawRule {
            name: rule.name,
            condition_sets: rule.condition_sets,
            conditions: Vec::new(),
            substitutions: rule.substitutions,
        }
    }
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
    pub minimum: Option<f64>,
    /// Upper bounds in design space coordinates.
    #[serde(rename = "@maximum", default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
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
    #[serde(rename = "@name", skip_serializing_if = "Option::is_none")]
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
    /// Localised UI strings for the family name of the source font.
    #[serde(rename = "familyname", default, skip_serializing_if = "Vec::is_empty")]
    pub localised_family_names: Vec<LocalizedString>,
    /// Location in designspace coordinates.
    #[serde(default, with = "serde_impls::location", skip_serializing_if = "Vec::is_empty")]
    pub location: Vec<Dimension>,
}

/// An [instance].
///
/// [instance]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#instance-element
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawInstance", into = "RawInstance")]
pub struct Instance {
    // per @anthrotype, contrary to spec, filename, familyname and stylename are optional
    /// The family name of the instance font. Corresponds with font.info.familyName
    pub familyname: Option<String>,
    /// The style name of the instance font. Corresponds with font.info.styleName
    pub stylename: Option<String>,
    /// A unique name that can be used to identify this font if it needs to be referenced elsewhere.
    pub name: Option<String>,
    /// A path to the instance file, relative to the root path of this document. The path can be at the same level as the document or lower.
    pub filename: Option<String>,
    /// Corresponds with font.info.postscriptFontName
    pub postscriptfontname: Option<String>,
    /// Corresponds with styleMapFamilyName
    pub stylemapfamilyname: Option<String>,
    /// Corresponds with styleMapStyleName
    pub stylemapstylename: Option<String>,
    /// The name of a [`LocationLabel`] that gives this instance's location.
    ///
    /// This is mutually exclusive with [`location`][Self::location]: loading a
    /// document that specifies both is an error (as in fontTools), and when
    /// saving, `location` is omitted if `location_label` is set.
    pub location_label: Option<String>,
    /// Localised UI strings for the style name of the instance font.
    pub localised_style_names: Vec<LocalizedString>,
    /// Localised UI strings for the family name of the instance font.
    pub localised_family_names: Vec<LocalizedString>,
    /// Localised UI strings for the styleMapStyleName of the instance font.
    pub localised_style_map_style_names: Vec<LocalizedString>,
    /// Localised UI strings for the styleMapFamilyName of the instance font.
    pub localised_style_map_family_names: Vec<LocalizedString>,
    /// Location in designspace.
    pub location: Vec<Dimension>,
    /// Arbitrary data about this instance
    pub lib: Dictionary,
}

/// Internal (de)serialization helper for [`Instance`] that enforces the
/// exclusivity of the `location` attribute and the `<location>` element.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "instance")]
struct RawInstance {
    #[serde(rename = "@familyname", skip_serializing_if = "Option::is_none")]
    familyname: Option<String>,
    #[serde(rename = "@stylename", skip_serializing_if = "Option::is_none")]
    stylename: Option<String>,
    #[serde(rename = "@name", skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(rename = "@filename", skip_serializing_if = "Option::is_none")]
    filename: Option<String>,
    #[serde(rename = "@postscriptfontname", skip_serializing_if = "Option::is_none")]
    postscriptfontname: Option<String>,
    #[serde(rename = "@stylemapfamilyname", skip_serializing_if = "Option::is_none")]
    stylemapfamilyname: Option<String>,
    #[serde(rename = "@stylemapstylename", skip_serializing_if = "Option::is_none")]
    stylemapstylename: Option<String>,
    #[serde(rename = "@location", default, skip_serializing_if = "Option::is_none")]
    location_label: Option<String>,
    #[serde(rename = "stylename", default, skip_serializing_if = "Vec::is_empty")]
    localised_style_names: Vec<LocalizedString>,
    #[serde(rename = "familyname", default, skip_serializing_if = "Vec::is_empty")]
    localised_family_names: Vec<LocalizedString>,
    #[serde(rename = "stylemapstylename", default, skip_serializing_if = "Vec::is_empty")]
    localised_style_map_style_names: Vec<LocalizedString>,
    #[serde(rename = "stylemapfamilyname", default, skip_serializing_if = "Vec::is_empty")]
    localised_style_map_family_names: Vec<LocalizedString>,
    #[serde(default, with = "serde_impls::location", skip_serializing_if = "Vec::is_empty")]
    location: Vec<Dimension>,
    #[serde(default, with = "serde_plist", skip_serializing_if = "Dictionary::is_empty")]
    lib: Dictionary,
}

impl TryFrom<RawInstance> for Instance {
    type Error = InstanceError;

    fn try_from(raw: RawInstance) -> Result<Self, Self::Error> {
        if raw.location_label.is_some() && !raw.location.is_empty() {
            return Err(InstanceError(
                "instance element must have at most one of the location attribute or the location element",
            ));
        }
        Ok(Instance {
            familyname: raw.familyname,
            stylename: raw.stylename,
            name: raw.name,
            filename: raw.filename,
            postscriptfontname: raw.postscriptfontname,
            stylemapfamilyname: raw.stylemapfamilyname,
            stylemapstylename: raw.stylemapstylename,
            location_label: raw.location_label,
            localised_style_names: raw.localised_style_names,
            localised_family_names: raw.localised_family_names,
            localised_style_map_style_names: raw.localised_style_map_style_names,
            localised_style_map_family_names: raw.localised_style_map_family_names,
            location: raw.location,
            lib: raw.lib,
        })
    }
}

impl From<Instance> for RawInstance {
    fn from(instance: Instance) -> Self {
        // Match fontTools: a location label takes precedence over an explicit
        // location.
        let location =
            if instance.location_label.is_some() { Vec::new() } else { instance.location };
        RawInstance {
            familyname: instance.familyname,
            stylename: instance.stylename,
            name: instance.name,
            filename: instance.filename,
            postscriptfontname: instance.postscriptfontname,
            stylemapfamilyname: instance.stylemapfamilyname,
            stylemapstylename: instance.stylemapstylename,
            location_label: instance.location_label,
            localised_style_names: instance.localised_style_names,
            localised_family_names: instance.localised_family_names,
            localised_style_map_style_names: instance.localised_style_map_style_names,
            localised_style_map_family_names: instance.localised_style_map_family_names,
            location,
            lib: instance.lib,
        }
    }
}

/// Error produced when an `<instance>` element is malformed.
#[derive(Debug)]
struct InstanceError(&'static str);

impl std::fmt::Display for InstanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
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
    pub uservalue: Option<f64>,
    /// Value on the axis in designcoordinates.
    #[serde(rename = "@xvalue", skip_serializing_if = "Option::is_none")]
    pub xvalue: Option<f64>,
    /// Separate value for anisotropic interpolations.
    #[serde(rename = "@yvalue", skip_serializing_if = "Option::is_none")]
    pub yvalue: Option<f64>,
}

impl DesignSpaceDocument {
    /// Load a designspace.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<DesignSpaceDocument, DesignSpaceLoadError> {
        let reader =
            std::io::BufReader::new(std::fs::File::open(path).map_err(DesignSpaceLoadError::Io)?);
        Self::load_from_reader(reader)
    }

    /// Load a designspace from a reader.
    pub fn load_from_reader(
        reader: impl std::io::BufRead,
    ) -> Result<DesignSpaceDocument, DesignSpaceLoadError> {
        quick_xml::de::from_reader(reader).map_err(DesignSpaceLoadError::DeError)
    }

    /// Save a designspace.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), DesignSpaceSaveError> {
        close_already::fs::write(path, self.serialize_to_string()?)?;
        Ok(())
    }

    /// Save a designspace to a writer.
    pub fn save_to_writer(
        &self,
        mut writer: impl std::io::Write,
    ) -> Result<(), DesignSpaceSaveError> {
        writer.write_all(self.serialize_to_string()?.as_bytes())?;
        Ok(())
    }

    /// Serialize the XML to a string.
    pub fn serialize_to_string(&self) -> Result<String, DesignSpaceSaveError> {
        let mut buf = String::from("<?xml version='1.0' encoding='UTF-8'?>\n");
        let mut xml_writer = quick_xml::se::Serializer::new(&mut buf);
        xml_writer.indent(' ', 2);
        self.serialize(xml_writer)?;
        buf.push('\n'); // trailing newline
        Ok(buf)
    }

    /// The minimum format version required to represent this document.
    ///
    /// Mirrors fontTools' `_getEffectiveFormatTuple`: the declared format is
    /// used unless the document contains features that require a newer one.
    fn effective_format(&self) -> f32 {
        let mut format = self.format;
        let needs_v5 = self.axes.iter().any(|axis| {
            axis.values.is_some() || axis.axis_ordering.is_some() || !axis.axis_labels.is_empty()
        }) || !self.location_labels.is_empty()
            || !self.variable_fonts.is_empty()
            || self.sources.iter().any(|source| !source.localised_family_names.is_empty())
            || self.instances.iter().any(|instance| {
                instance.location_label.is_some()
                    || instance.location.iter().any(|dim| dim.uservalue.is_some())
            });
        if needs_v5 {
            format = format.max(5.0);
        }
        if self.axis_mappings.as_ref().is_some_and(|mappings| !mappings.is_empty()) {
            format = format.max(5.1);
        }
        format
    }
}

impl Rules {
    /// Returns `true` if there are no rules.
    fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// Always write the format as `major.minor`, e.g. `5.0` rather than `5`.
fn serialize_format<S: serde::Serializer>(format: &f32, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&format!("{format:.1}"))
}

/// Internal struct matching the XML structure for (de)serialization.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "designspace")]
struct RawDesignSpaceDocument {
    #[serde(rename = "@format", serialize_with = "serialize_format")]
    format: f32,
    #[serde(default, skip_serializing_if = "RawAxes::is_empty")]
    axes: RawAxes,
    #[serde(
        rename = "labels",
        default,
        with = "serde_impls::location_labels",
        skip_serializing_if = "Vec::is_empty"
    )]
    location_labels: Vec<LocationLabel>,
    #[serde(default, skip_serializing_if = "Rules::is_empty")]
    rules: Rules,
    #[serde(default, with = "serde_impls::sources", skip_serializing_if = "Vec::is_empty")]
    sources: Vec<Source>,
    #[serde(
        rename = "variable-fonts",
        default,
        with = "serde_impls::variable_fonts",
        skip_serializing_if = "Vec::is_empty"
    )]
    variable_fonts: Vec<VariableFont>,
    #[serde(default, with = "serde_impls::instances", skip_serializing_if = "Vec::is_empty")]
    instances: Vec<Instance>,
    #[serde(default, with = "serde_plist", skip_serializing_if = "Dictionary::is_empty")]
    lib: Dictionary,
}

/// Internal container for axes and their optional mappings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct RawAxes {
    #[serde(rename = "@elidedfallbackname", default, skip_serializing_if = "Option::is_none")]
    elided_fallback_name: Option<String>,
    #[serde(default, rename = "axis")]
    axis: Vec<Axis>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mappings: Option<AxisMappings>,
}

impl RawAxes {
    fn is_empty(&self) -> bool {
        self.elided_fallback_name.is_none()
            && self.axis.is_empty()
            && self.mappings.as_ref().is_none_or(|m| m.is_empty())
    }
}

impl From<RawDesignSpaceDocument> for DesignSpaceDocument {
    fn from(raw: RawDesignSpaceDocument) -> Self {
        Self {
            format: raw.format,
            axes: raw.axes.axis,
            axis_mappings: raw.axes.mappings,
            elided_fallback_name: raw.axes.elided_fallback_name,
            location_labels: raw.location_labels,
            rules: raw.rules,
            sources: raw.sources,
            variable_fonts: raw.variable_fonts,
            instances: raw.instances,
            lib: raw.lib,
        }
    }
}

impl From<DesignSpaceDocument> for RawDesignSpaceDocument {
    fn from(doc: DesignSpaceDocument) -> Self {
        let format = doc.effective_format();
        Self {
            format,
            axes: RawAxes {
                elided_fallback_name: doc.elided_fallback_name,
                axis: doc.axes,
                mappings: doc.axis_mappings,
            },
            location_labels: doc.location_labels,
            rules: doc.rules,
            sources: doc.sources,
            variable_fonts: doc.variable_fonts,
            instances: doc.instances,
            lib: doc.lib,
        }
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
            serde_from_field!(@impl $mod_name, $field_name, $inner);
        };
        // The `$field_name = "xml-name"` form is for element names that are not
        // valid Rust identifiers, such as `variable-font`.
        ($mod_name:ident, $field_name:ident = $xml_name:literal, $inner:path) => {
            serde_from_field!(@impl $mod_name, $field_name, $inner, $xml_name);
        };
        (@impl $mod_name:ident, $field_name:ident, $inner:path $(, $xml_name:literal)?) => {
            pub(super) mod $mod_name {
                pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Vec<$inner>, D::Error>
                where
                    D: ::serde::Deserializer<'de>,
                {
                    use serde::Deserialize as _;
                    #[derive(::serde::Deserialize)]
                    struct Helper {
                        $(#[serde(rename = $xml_name)])?
                        #[serde(default)]
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
                    use serde::Serialize as _;
                    #[derive(::serde::Serialize)]
                    struct Helper<'a> {
                        $(#[serde(rename = $xml_name)])?
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
    serde_from_field!(sources, source, crate::designspace::Source);
    serde_from_field!(location_labels, label, crate::designspace::LocationLabel);
    serde_from_field!(
        variable_fonts,
        variable_font = "variable-font",
        crate::designspace::VariableFont
    );
    serde_from_field!(axis_subsets, axis_subset = "axis-subset", crate::designspace::AxisSubset);
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use plist::Value;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    use crate::designspace::{AxisMapping, Dimension};

    use super::*;

    fn dim_name_xvalue(name: &str, xvalue: f64) -> Dimension {
        Dimension { name: name.to_string(), uservalue: None, xvalue: Some(xvalue), yvalue: None }
    }

    #[test]
    fn load_from_reader_matches_load() {
        let from_path = DesignSpaceDocument::load("testdata/wght.designspace").unwrap();
        let bytes = std::fs::read("testdata/wght.designspace").unwrap();
        let from_reader = DesignSpaceDocument::load_from_reader(bytes.as_slice()).unwrap();
        assert_eq!(from_path, from_reader);
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
        assert!(ds.axes[0].label_names.is_empty());
    }

    #[test]
    fn read_label_names() {
        let ds = DesignSpaceDocument::load("testdata/labelname_wght.designspace").unwrap();
        assert_eq!(1, ds.axes.len());
        assert!(!ds.axes[0].label_names.is_empty());

        assert_eq!(ds.axes[0].label_names[0].language, "fa-IR");
        assert_eq!(ds.axes[0].label_names[0].string, "قطر");

        assert_eq!(ds.axes[0].label_names[1].language, "en");
        assert_eq!(ds.axes[0].label_names[1].string, "Weight");
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
    fn load_save_round_trip_label_names() {
        // Given
        let dir = TempDir::new().unwrap();
        let ds_test_save_location = dir.path().join("labelname_wght.designspace");

        // When
        let ds_initial = DesignSpaceDocument::load("testdata/labelname_wght.designspace").unwrap();
        ds_initial.save(&ds_test_save_location).expect("failed to save designspace");

        let ds_after = DesignSpaceDocument::load(ds_test_save_location.clone())
            .expect("failed to load saved designspace");

        // Then
        assert_eq!(ds_initial, ds_after);

        // Check the raw file content to ensure 'xml:lang' which gets stripped on deserialization
        // is correctly serialized.
        let saved_content = std::fs::read_to_string(&ds_test_save_location)
            .expect("Failed to read saved designspace file");
        assert!(saved_content.contains("xml:lang=\"fa-IR\""));
        assert!(saved_content.contains("xml:lang=\"en\""),);
    }

    #[test]
    fn accept_bare_conditions_in_rule() {
        // Legacy format: <condition> elements directly inside <rule> without <conditionset>
        let designspace = DesignSpaceDocument::load("testdata/BareConditions.designspace").unwrap();

        assert_eq!(
            &designspace.rules,
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

    #[test]
    fn load_axis_mappings() {
        // Given
        let ds = DesignSpaceDocument::load("testdata/with_mappings.designspace").unwrap();
        let mappings = ds.axis_mappings.as_ref().expect("should have axis_mappings");

        // Then
        assert_eq!(mappings.description.as_deref(), Some("Test avar2 mappings"));
        assert_eq!(mappings.mappings.len(), 2);
        let m1 = &mappings.mappings[0];
        assert_eq!(m1.description.as_deref(), Some("Heavy at wide gets less heavy"));
        assert_eq!(m1.input.len(), 2);
        assert_eq!(m1.input[0].name, "Weight");
        assert_eq!(m1.input[0].xvalue, Some(700.0));
        assert_eq!(m1.input[1].name, "Width");
        assert_eq!(m1.input[1].xvalue, Some(125.0));
        assert_eq!(m1.output.len(), 1);
        assert_eq!(m1.output[0].name, "Weight");
        assert_eq!(m1.output[0].xvalue, Some(680.0));
        assert!(mappings.mappings[1].description.is_none());
    }

    #[test]
    fn load_save_round_trip_with_mappings() {
        // Given
        let dir = TempDir::new().unwrap();
        let ds_test_save_location = dir.path().join("with_mappings.designspace");

        // When
        let ds_initial = DesignSpaceDocument::load("testdata/with_mappings.designspace").unwrap();
        ds_initial.save(&ds_test_save_location).expect("failed to save designspace");
        let ds_after = DesignSpaceDocument::load(&ds_test_save_location)
            .expect("failed to load saved designspace");

        // Then
        assert_eq!(ds_initial, ds_after);
        let saved_content = std::fs::read_to_string(&ds_test_save_location)
            .expect("Failed to read saved designspace file");
        assert!(saved_content.contains("<mappings"));
        assert!(saved_content.contains("<mapping"));
        assert!(saved_content.contains("<input>"));
        assert!(saved_content.contains("<output>"));
    }

    #[test]
    fn designspace_without_mappings_has_none() {
        let ds = DesignSpaceDocument::load("testdata/wght.designspace").unwrap();
        assert!(ds.axis_mappings.is_none());
    }

    const DECIMAL_VALUES: &str = r#"<?xml version='1.0' encoding='UTF-8'?>
<designspace format="5.0">
  <axes>
    <axis name="Weight" tag="wght" minimum="-1" default="-0.55" maximum="1.125">
      <map input="100" output="-1"/>
      <map input="400" output="-0.55"/>
      <map input="500" output="0.1"/>
      <map input="900" output="1.125"/>
    </axis>
  </axes>
  <rules>
    <rule name="alt">
      <conditionset>
        <condition name="Weight" minimum="0.1" maximum="0.55"/>
      </conditionset>
      <sub name="a" with="a.alt"/>
    </rule>
  </rules>
  <sources>
    <source filename="Light.ufo">
      <location>
        <dimension name="Weight" xvalue="-0.55"/>
      </location>
    </source>
  </sources>
  <instances>
    <instance filename="Thin.ufo">
      <location>
        <dimension name="Weight" uservalue="400" xvalue="0.1" yvalue="-0.55"/>
      </location>
    </instance>
  </instances>
</designspace>
"#;

    #[test]
    fn read_decimal_values_exactly() {
        let ds: DesignSpaceDocument = quick_xml::de::from_str(DECIMAL_VALUES).unwrap();
        let axis = &ds.axes[0];
        assert_eq!(axis.default, -0.55_f64);
        assert_eq!(axis.maximum, Some(1.125_f64));
        let map = axis.map.as_ref().unwrap();
        assert_eq!(map[1].output, -0.55_f64);
        assert_eq!(map[2].output, 0.1_f64);
        let condition = &ds.rules.rules[0].condition_sets[0].conditions[0];
        assert_eq!(condition.minimum, Some(0.1_f64));
        assert_eq!(condition.maximum, Some(0.55_f64));
        assert_eq!(ds.sources[0].location[0].xvalue, Some(-0.55_f64));
        let dim = &ds.instances[0].location[0];
        assert_eq!(dim.uservalue, Some(400.0_f64));
        assert_eq!(dim.xvalue, Some(0.1_f64));
        assert_eq!(dim.yvalue, Some(-0.55_f64));
    }

    #[test]
    fn save_decimal_values_exactly() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("decimal.designspace");
        let ds: DesignSpaceDocument = quick_xml::de::from_str(DECIMAL_VALUES).unwrap();
        ds.save(&path).unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains(r#"default="-0.55""#));
        assert!(saved.contains(r#"output="0.1""#));
        assert!(saved.contains(r#"minimum="0.1""#));
        assert!(saved.contains(r#"xvalue="-0.55""#));
        assert!(!saved.contains("0.550000011920929"));
        assert!(!saved.contains("0.10000000149011612"));
        assert_eq!(ds, DesignSpaceDocument::load(&path).unwrap());
    }

    #[test]
    fn save_preserves_more_than_f32_precision() {
        let ds = DesignSpaceDocument::load("testdata/MutatorSans.designspace").unwrap();
        let saved = quick_xml::se::to_string(&ds).unwrap();
        assert!(saved.contains(r#"xvalue="35.329171""#));
        assert!(saved.contains(r#"xvalue="854.834192""#));
    }

    fn dim_uservalue(name: &str, uservalue: f64) -> Dimension {
        Dimension { name: name.to_string(), uservalue: Some(uservalue), xvalue: None, yvalue: None }
    }

    fn lstr(lang: &str, s: &str) -> LocalizedString {
        LocalizedString { language: lang.into(), string: s.into() }
    }

    fn label(name: &str, user_value: f64) -> AxisLabel {
        AxisLabel { name: name.into(), user_value, ..Default::default() }
    }

    fn ranged(label: AxisLabel, min: f64, max: f64) -> AxisLabel {
        AxisLabel { user_minimum: Some(min), user_maximum: Some(max), ..label }
    }

    fn subset(name: &str, value: SubsetValue) -> AxisSubset {
        AxisSubset { name: name.into(), value }
    }

    #[test]
    fn v5_axes_and_labels() {
        let ds = DesignSpaceDocument::load("testdata/v5.designspace").unwrap();

        assert_eq!(ds.elided_fallback_name.as_deref(), Some("Regular"));
        assert_eq!(ds.format, 5.1);

        // Weight axis.
        let weight = &ds.axes[0];
        assert_eq!(
            weight.axis_labels,
            vec![
                AxisLabel {
                    label_names: vec![lstr("de", "Extraleicht"), lstr("fr", "Extra léger")],
                    ..ranged(label("Extra Light", 200.), 200., 250.)
                },
                ranged(label("Light", 300.), 250., 350.),
                AxisLabel { elidable: true, ..ranged(label("Regular", 400.), 350., 450.) },
                ranged(label("Semi Bold", 600.), 450., 650.),
                ranged(label("Bold", 700.), 650., 850.),
                ranged(label("Black", 900.), 850., 900.),
                AxisLabel {
                    elidable: true,
                    linked_user_value: Some(700.),
                    ..label("Regular", 400.)
                },
                AxisLabel { linked_user_value: Some(400.), ..label("Bold", 700.) },
            ]
        );
        assert_eq!(weight.map.as_ref().unwrap().len(), 6);
        assert_eq!(weight.label_names, vec![lstr("en", "Wéíght"), lstr("fa-IR", "قطر")]);
        assert_eq!(weight.axis_ordering, None);

        // Width axis.
        let width = &ds.axes[1];
        assert!(width.hidden);
        assert_eq!(width.axis_ordering, Some(1));
        assert_eq!(
            width.axis_labels,
            vec![
                label("Condensed", 50.),
                AxisLabel { elidable: true, older_sibling: true, ..label("Normal", 100.) },
                label("Wide", 125.),
                AxisLabel { user_minimum: Some(150.), ..label("Extra Wide", 150.) },
            ]
        );

        // Italic axis (discrete).
        let italic = &ds.axes[2];
        assert_eq!(italic.values, Some(vec![0.0, 1.0]));
        assert_eq!(italic.minimum, None);
        assert_eq!(italic.maximum, None);
        assert_eq!(
            italic.axis_labels,
            vec![
                AxisLabel { elidable: true, linked_user_value: Some(1.), ..label("Roman", 0.) },
                label("Italic", 1.),
            ]
        );

        // avar2 axis mappings.
        let mappings = ds.axis_mappings.as_ref().expect("should have axis_mappings");
        assert_eq!(mappings.mappings.len(), 1);
        assert_eq!(mappings.mappings[0].description.as_deref(), Some("hello"));
        assert_eq!(mappings.mappings[0].input.len(), 2);
        assert_eq!(mappings.mappings[0].output, vec![dim_name_xvalue("Weight", 870.0)]);
    }

    #[test]
    fn v5_location_labels() {
        let ds = DesignSpaceDocument::load("testdata/v5.designspace").unwrap();
        assert_eq!(
            ds.location_labels,
            vec![
                LocationLabel {
                    name: "Some Style".into(),
                    older_sibling: true,
                    label_names: vec![lstr("fr", "Un Style")],
                    location: vec![
                        dim_uservalue("Weight", 300.0),
                        dim_uservalue("Width", 50.0),
                        dim_uservalue("Italic", 0.0),
                    ],
                    ..Default::default()
                },
                LocationLabel {
                    name: "Other".into(),
                    elidable: true,
                    location: vec![
                        dim_uservalue("Weight", 700.0),
                        dim_uservalue("Width", 100.0),
                        dim_uservalue("Italic", 1.0),
                    ],
                    ..Default::default()
                },
            ]
        );
    }

    #[test]
    fn v5_variable_fonts() {
        let ds = DesignSpaceDocument::load("testdata/v5.designspace").unwrap();

        let got: Vec<(&str, Option<&str>, Vec<AxisSubset>)> = ds
            .variable_fonts
            .iter()
            .map(|vf| (vf.name.as_str(), vf.filename.as_deref(), vf.axis_subsets.clone()))
            .collect();
        assert_eq!(
            got,
            vec![
                (
                    "Test_WghtWdth",
                    Some("Test_WghtWdth_different_from_name.ttf"),
                    vec![
                        subset("Weight", SubsetValue::default()),
                        subset("Width", SubsetValue::default())
                    ],
                ),
                ("Test_Wght", None, vec![subset("Weight", SubsetValue::default())]),
                (
                    "TestCd_Wght",
                    None,
                    vec![
                        subset("Weight", SubsetValue::default()),
                        subset("Width", SubsetValue::Discrete(0.0)),
                    ],
                ),
                (
                    "TestWd_Wght",
                    None,
                    vec![
                        subset("Weight", SubsetValue::default()),
                        subset("Width", SubsetValue::Discrete(1000.0)),
                    ],
                ),
                (
                    "TestItalic_Wght",
                    None,
                    vec![
                        subset("Weight", SubsetValue::default()),
                        subset("Italic", SubsetValue::Discrete(1.0)),
                    ],
                ),
                (
                    "TestRB_Wght",
                    None,
                    vec![
                        subset(
                            "Weight",
                            SubsetValue::Range {
                                minimum: Some(400.0),
                                default: Some(400.0),
                                maximum: Some(700.0),
                            }
                        ),
                        subset("Italic", SubsetValue::Discrete(0.0)),
                    ],
                ),
            ]
        );

        // Libs, checked by key.
        assert_eq!(
            ds.variable_fonts[0].lib.get("com.vtt.source"),
            Some(&Value::String("sources/vtt/Test_WghtWdth.vtt".into()))
        );
        let font_info = ds.variable_fonts[0]
            .lib
            .get("public.fontInfo")
            .and_then(Value::as_dictionary)
            .expect("should have public.fontInfo dict");
        assert_eq!(font_info.get("familyName"), Some(&Value::String("My Font Narrow VF".into())));
        assert_eq!(
            ds.variable_fonts[1].lib.get("com.vtt.source"),
            Some(&Value::String("sources/vtt/Test_Wght.vtt".into()))
        );
        for vf in &ds.variable_fonts[2..] {
            assert!(vf.lib.is_empty(), "{}: {:?}", vf.name, vf.lib);
        }
    }

    #[test]
    fn v5_sources_and_instances() {
        let ds = DesignSpaceDocument::load("testdata/v5.designspace").unwrap();

        // Sources.
        assert_eq!(ds.sources.len(), 4);
        assert_eq!(
            ds.sources[0].localised_family_names,
            vec![lstr("fr", "Montserrat"), lstr("ja", "モンセラート")]
        );
        assert!(ds.sources[1].localised_family_names.is_empty());
        assert_eq!(ds.sources[2].layer.as_deref(), Some("supports"));

        // Instances.
        assert_eq!(ds.instances.len(), 6);

        let instance0 = &ds.instances[0];
        assert_eq!(
            instance0.localised_style_names,
            vec![lstr("fr", "Demigras"), lstr("ja", "半ば")]
        );
        assert_eq!(
            instance0.localised_family_names,
            vec![lstr("fr", "Montserrat"), lstr("ja", "モンセラート")]
        );
        assert_eq!(instance0.localised_style_map_style_names, vec![lstr("de", "Standard")]);
        assert_eq!(
            instance0.localised_style_map_family_names,
            vec![lstr("de", "Montserrat Halbfett"), lstr("ja", "モンセラート SemiBold")]
        );
        assert_eq!(
            instance0.lib.get("com.coolDesignspaceApp.binaryData"),
            Some(&Value::Data(b"<binary gunk>".to_vec()))
        );

        let instance1 = &ds.instances[1];
        assert_eq!(instance1.location[1].xvalue, Some(400.0));
        assert_eq!(instance1.location[1].yvalue, Some(300.0));

        let instance2 = &ds.instances[2];
        assert_eq!(instance2.location_label.as_deref(), Some("Some Style"));
        assert!(instance2.location.is_empty());

        let instance4 = &ds.instances[4];
        assert_eq!(
            instance4.location,
            vec![
                dim_name_xvalue("Weight", 10.0),
                dim_uservalue("Width", 100.0),
                dim_name_xvalue("Italic", 0.0),
            ]
        );

        let instance5 = &ds.instances[5];
        assert_eq!(
            instance5.location,
            vec![
                dim_uservalue("Weight", 300.0),
                dim_uservalue("Width", 130.0),
                dim_uservalue("Italic", 1.0),
            ]
        );
        let font_info = instance5
            .lib
            .get("public.fontInfo")
            .and_then(Value::as_dictionary)
            .expect("should have public.fontInfo dict");
        let name_records = font_info.get("openTypeNameRecords").and_then(Value::as_array).unwrap();
        assert_eq!(
            name_records[0]
                .as_dictionary()
                .and_then(|d| d.get("nameID"))
                .and_then(Value::as_unsigned_integer),
            Some(7)
        );

        // Top-level lib.
        assert_eq!(
            ds.lib.get("com.coolDesignspaceApp.previewSize"),
            Some(&Value::Integer(30.into()))
        );
    }

    #[test]
    fn load_save_round_trip_v5() {
        // Given
        let dir = TempDir::new().unwrap();
        let ds_test_save_location = dir.path().join("v5.designspace");

        // When
        let ds_initial = DesignSpaceDocument::load("testdata/v5.designspace").unwrap();
        ds_initial.save(&ds_test_save_location).expect("failed to save designspace");
        let ds_after = DesignSpaceDocument::load(&ds_test_save_location)
            .expect("failed to load saved designspace");

        // Then
        assert_eq!(ds_initial, ds_after);

        let saved_content = std::fs::read_to_string(&ds_test_save_location)
            .expect("Failed to read saved designspace file");
        assert!(saved_content.contains(r#"format="5.1""#));
        assert!(saved_content.contains(r#"elidedfallbackname="Regular""#));
        assert!(saved_content.contains("<variable-fonts>"));
        assert!(saved_content.contains(
            r#"<variable-font name="Test_WghtWdth" filename="Test_WghtWdth_different_from_name.ttf">"#
        ));
        assert!(saved_content.contains("<axis-subsets>"));
        assert!(saved_content.contains(
            r#"<axis-subset name="Weight" userminimum="400" userdefault="400" usermaximum="700"/>"#
        ));
        assert!(saved_content.contains(r#"<axis-subset name="Width" uservalue="0"/>"#));
        assert!(saved_content.contains(r#"<labels ordering="1">"#));
        assert!(saved_content.contains(r#"linkeduservalue="700""#));
        assert!(saved_content.contains(r#"oldersibling="true""#));
        assert!(saved_content.contains(r#"elidable="true""#));
        assert!(saved_content.contains(r#"values="0 1""#));
        assert!(saved_content.contains(r#"<instance location="Some Style"/>"#));
        assert!(saved_content.contains(r#"<familyname xml:lang="ja">モンセラート</familyname>"#));
        assert!(saved_content.contains(
            r#"<stylemapfamilyname xml:lang="de">Montserrat Halbfett</stylemapfamilyname>"#
        ));

        // Deprecated instance/source children are dropped on save.
        assert!(!saved_content.contains("<glyphs>"));
        assert!(!saved_content.contains("<kerning"));
        assert!(!saved_content.contains("<info"));
    }

    #[test]
    fn axis_subset_variants() {
        // Given
        let vf = VariableFont {
            name: "vf".into(),
            filename: None,
            axis_subsets: vec![
                AxisSubset { name: "Whole".into(), value: SubsetValue::default() },
                AxisSubset {
                    name: "Tent".into(),
                    value: SubsetValue::Range {
                        minimum: Some(1.0),
                        default: Some(2.0),
                        maximum: Some(3.0),
                    },
                },
                AxisSubset {
                    name: "Partial".into(),
                    value: SubsetValue::Range { minimum: Some(5.0), default: None, maximum: None },
                },
                AxisSubset { name: "Discrete".into(), value: SubsetValue::Discrete(4.0) },
            ],
            lib: Dictionary::new(),
        };

        // When
        let xml = quick_xml::se::to_string(&vf).expect("should serialize");
        let round_tripped: VariableFont =
            quick_xml::de::from_str(&xml).expect("should deserialize");

        // Then
        assert_eq!(vf, round_tripped);
        assert!(xml.contains(r#"<axis-subset name="Whole"/>"#), "unexpected xml: {xml}");
        assert!(
            xml.contains(r#"<axis-subset name="Partial" userminimum="5"/>"#),
            "unexpected xml: {xml}"
        );
        let full: Vec<bool> = vf.axis_subsets.iter().map(|subset| subset.value.is_full()).collect();
        assert_eq!(full, [true, false, false, false]);

        // An axis-subset can't mix uservalue with userminimum/userdefault/usermaximum.
        let mixed = r#"<variable-font name="x"><axis-subsets><axis-subset name="a" uservalue="1" userminimum="1" userdefault="1" usermaximum="2"/></axis-subsets></variable-font>"#;
        let err = quick_xml::de::from_str::<VariableFont>(mixed).unwrap_err();
        assert!(err.to_string().contains("axis-subset"), "unexpected error: {err}");
    }

    #[test]
    fn effective_format_on_save() {
        fn fmt_of(doc: &DesignSpaceDocument) -> String {
            let xml = quick_xml::se::to_string(doc).expect("should serialize");
            let needle = "format=\"";
            let start = xml.find(needle).expect("should have format attr") + needle.len();
            let end = xml[start..].find('"').expect("unterminated format attr");
            xml[start..start + end].to_string()
        }

        let axis = Axis {
            name: "Weight".into(),
            tag: "wght".into(),
            default: 400.0,
            ..Default::default()
        };
        let source = Source { filename: "Test.ufo".into(), ..Default::default() };
        let dim = Dimension { name: "Weight".into(), xvalue: Some(400.0), ..Default::default() };

        let base = DesignSpaceDocument {
            format: 4.1,
            axes: vec![axis.clone()],
            sources: vec![source.clone()],
            ..Default::default()
        };
        assert_eq!(fmt_of(&base), "4.1");

        // variable_fonts bumps to 5, axis_mappings further bumps to 5.1.
        let mut doc = base.clone();
        doc.variable_fonts.push(VariableFont { name: "VF".into(), ..Default::default() });
        assert_eq!(fmt_of(&doc), "5.0");

        doc.axis_mappings = Some(AxisMappings {
            mappings: vec![AxisMappingEntry {
                input: vec![dim.clone()],
                output: vec![dim.clone()],
                ..Default::default()
            }],
            ..Default::default()
        });
        assert_eq!(fmt_of(&doc), "5.1");

        // As in fontTools, elidedfallbackname alone does not bump the version.
        let mut doc = base.clone();
        doc.elided_fallback_name = Some("Regular".into());
        assert_eq!(fmt_of(&doc), "4.1");

        // Each of the other v5 triggers bumps 4.1 -> 5.0 in isolation.
        let mut doc = base.clone();
        doc.axes[0].values = Some(vec![0.0, 1.0]);
        assert_eq!(fmt_of(&doc), "5.0");

        let mut doc = base.clone();
        doc.axes[0].axis_ordering = Some(1);
        assert_eq!(fmt_of(&doc), "5.0");

        let mut doc = base.clone();
        doc.axes[0].axis_labels =
            vec![AxisLabel { name: "Foo".into(), user_value: 1.0, ..Default::default() }];
        assert_eq!(fmt_of(&doc), "5.0");

        let mut doc = base.clone();
        doc.location_labels = vec![LocationLabel { name: "Foo".into(), ..Default::default() }];
        assert_eq!(fmt_of(&doc), "5.0");

        let mut doc = base.clone();
        doc.sources[0].localised_family_names =
            vec![LocalizedString { language: "fr".into(), string: "Foo".into() }];
        assert_eq!(fmt_of(&doc), "5.0");

        let mut doc = base.clone();
        doc.instances = vec![Instance { location_label: Some("Foo".into()), ..Default::default() }];
        assert_eq!(fmt_of(&doc), "5.0");

        let mut doc = base.clone();
        doc.instances = vec![Instance {
            location: vec![Dimension {
                name: "Weight".into(),
                uservalue: Some(1.0),
                ..Default::default()
            }],
            ..Default::default()
        }];
        assert_eq!(fmt_of(&doc), "5.0");

        // A doc already at 5.1 with no v5 data isn't lowered.
        let stayed_51 = DesignSpaceDocument {
            format: 5.1,
            axes: vec![axis],
            sources: vec![source],
            ..Default::default()
        };
        assert_eq!(fmt_of(&stayed_51), "5.1");
    }

    #[test]
    fn instance_with_both_location_forms_is_rejected() {
        let xml = r#"<designspace format="5.0"><instances>
            <instance name="x" location="Some Style">
              <location><dimension name="Weight" xvalue="400"/></location>
            </instance>
        </instances></designspace>"#;
        let err = DesignSpaceDocument::load_from_reader(xml.as_bytes()).unwrap_err();
        assert!(err.to_string().contains("at most one of the location"), "{err}");
    }

    #[test]
    fn location_label_wins_over_location_on_save() {
        let doc = DesignSpaceDocument {
            format: 5.0,
            instances: vec![Instance {
                location_label: Some("Some Style".into()),
                location: vec![dim_name_xvalue("Weight", 400.)],
                ..Default::default()
            }],
            ..Default::default()
        };
        let xml = quick_xml::se::to_string(&doc).unwrap();
        assert!(xml.contains(r#"<instance location="Some Style"/>"#), "{xml}");
        assert!(!xml.contains("<location>"), "{xml}");
    }

    #[test]
    fn labels_and_sources_without_location_load() {
        // fontTools omits an empty <location> element for both of these.
        let xml = r#"<designspace format="5.0">
            <labels><label name="Bare"/></labels>
            <sources><source filename="a.ufo"/></sources>
        </designspace>"#;
        let ds = DesignSpaceDocument::load_from_reader(xml.as_bytes()).unwrap();
        assert_eq!(ds.location_labels[0].name, "Bare");
        assert!(ds.location_labels[0].location.is_empty());
        assert!(ds.sources[0].location.is_empty());
        let saved = quick_xml::se::to_string(&ds).unwrap();
        assert!(!saved.contains("<location"), "{saved}");
    }

    #[test]
    fn instance_without_location_loads() {
        let xml = r#"<designspace format="5.0"><instances><instance name="x"/></instances></designspace>"#;
        let ds = DesignSpaceDocument::load_from_reader(xml.as_bytes()).unwrap();
        assert!(ds.instances[0].location.is_empty());
        assert_eq!(ds.instances[0].location_label, None);
    }
}
