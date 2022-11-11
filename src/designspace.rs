//! Reading and writing designspace files.

#![deny(rustdoc::broken_intra_doc_links)]

use std::{fs::File, io::BufReader, path::Path};

use crate::error::DesignSpaceLoadError;

/// A [designspace]].
///
/// [designspace]: https://fonttools.readthedocs.io/en/latest/designspaceLib/index.html
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(from = "DesignSpaceDocumentXmlRepr")]
pub struct DesignSpaceDocument {
    /// Design space format version.
    pub format: f32,
    /// One or more axes.
    pub axes: Vec<Axis>,
    /// One or more sources.
    pub sources: Vec<Source>,
    /// One or more instances.
    pub instances: Vec<Instance>,
}

/// https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#overview
#[derive(Deserialize)]
#[serde(rename = "designspace")]
struct DesignSpaceDocumentXmlRepr {
    pub format: f32,
    pub axes: AxesXmlRepr,
    pub sources: SourcesXmlRepr,
    pub instances: InstancesXmlRepr,
}

impl From<DesignSpaceDocumentXmlRepr> for DesignSpaceDocument {
    fn from(xml_form: DesignSpaceDocumentXmlRepr) -> Self {
        DesignSpaceDocument {
            format: xml_form.format,
            axes: xml_form.axes.axis,
            sources: xml_form.sources.source,
            instances: xml_form.instances.instance,
        }
    }
}

/// https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#axes-element
#[derive(Deserialize)]
#[serde(rename = "axes")]
pub struct AxesXmlRepr {
    /// One or more axis definitions.
    pub axis: Vec<Axis>,
}

/// A [axis]].
///
/// [axis]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#axis-element
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename = "axis")]
pub struct Axis {
    /// Name of the axis that is used in the location elements.
    pub name: String,
    /// 4 letters. Some axis tags are registered in the OpenType Specification.
    pub tag: String,
    /// The default value for this axis, in user space coordinates.
    pub default: f32,
    /// Records whether this axis needs to be hidden in interfaces.
    #[serde(default)]
    pub hidden: bool,
    /// The minimum value for a continuous axis, in user space coordinates.
    pub minimum: Option<f32>,
    /// The maximum value for a continuous axis, in user space coordinates.
    pub maximum: Option<f32>,
    /// The possible values for a discrete axis, in user space coordinates.
    pub values: Option<Vec<f32>>,
    /// Mapping between user space coordinates and design space coordinates.
    pub map: Option<Vec<AxisMapping>>,
}

/// Maps one input value (user space coord) to one output value (design space coord).
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename = "map")]
pub struct AxisMapping {
    /// user space coordinate
    pub input: f32,
    /// designspace coordinate
    pub output: f32,
}

/// https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#sources-element
#[derive(Deserialize)]
#[serde(rename = "sources")]
struct SourcesXmlRepr {
    /// One or more sources.
    pub source: Vec<Source>,
}

/// A [source]].
///
/// [source]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#id25
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(from = "SourceXmlRepr")]
pub struct Source {
    /// The family name of the source font.
    pub familyname: Option<String>,
    /// The style name of the source font.
    pub stylename: Option<String>,
    /// A unique name that can be used to identify this font if it needs to be referenced elsewhere.
    pub name: String,
    /// A path to the source file, relative to the root path of this document. The path can be at the same level as the document or lower.
    pub filename: String,
    /// The name of the layer in the source file. If no layer attribute is given assume the foreground layer should be used.
    pub layer: Option<String>,
    /// Location in designspace coordinates.
    pub location: Vec<Dimension>,
}

/// https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#source-element
#[derive(Deserialize)]
#[serde(rename = "source")]
struct SourceXmlRepr {
    pub familyname: Option<String>,
    pub stylename: Option<String>,
    pub name: String,
    pub filename: String,
    pub layer: Option<String>,
    pub location: LocationXmlRepr,
}

impl From<SourceXmlRepr> for Source {
    fn from(xml_form: SourceXmlRepr) -> Self {
        Source {
            familyname: xml_form.familyname,
            stylename: xml_form.stylename,
            name: xml_form.name,
            filename: xml_form.filename,
            layer: xml_form.layer,
            location: xml_form.location.dimension,
        }
    }
}

/// https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#instances-element
#[derive(Deserialize)]
#[serde(rename = "instances")]
struct InstancesXmlRepr {
    /// One or more instances located somewhere in designspace.
    pub instance: Vec<Instance>,
}

/// An [instance]].
///
/// [instance]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#instance-element
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(from = "InstanceXmlRepr")]
pub struct Instance {
    // per @anthrotype, contrary to spec, filename, familyname and stylename are optional
    /// The family name of the instance font. Corresponds with font.info.familyName
    pub familyname: Option<String>,
    /// The style name of the instance font. Corresponds with font.info.styleName
    pub stylename: Option<String>,
    /// A unique name that can be used to identify this font if it needs to be referenced elsewhere.
    pub name: String,
    /// A path to the instance file, relative to the root path of this document. The path can be at the same level as the document or lower.
    pub filename: Option<String>,
    /// Corresponds with font.info.postscriptFontName
    pub postscriptfontname: Option<String>,
    /// Corresponds with styleMapFamilyName
    pub stylemapfamilyname: Option<String>,
    /// Corresponds with styleMapStyleName
    pub stylemapstylename: Option<String>,
    /// Location in designspace.
    pub location: Vec<Dimension>,
}

/// https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#instance-element
#[derive(Deserialize)]
struct InstanceXmlRepr {
    pub familyname: Option<String>,
    pub stylename: Option<String>,
    pub name: String,
    pub filename: Option<String>,
    pub postscriptfontname: Option<String>,
    pub stylemapfamilyname: Option<String>,
    pub stylemapstylename: Option<String>,
    pub location: LocationXmlRepr,
}

impl From<InstanceXmlRepr> for Instance {
    fn from(instance_xml: InstanceXmlRepr) -> Self {
        Instance {
            familyname: instance_xml.familyname,
            stylename: instance_xml.stylename,
            name: instance_xml.name,
            filename: instance_xml.filename,
            postscriptfontname: instance_xml.postscriptfontname,
            stylemapfamilyname: instance_xml.stylemapfamilyname,
            stylemapstylename: instance_xml.stylemapstylename,
            location: instance_xml.location.dimension,
        }
    }
}

/// https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#location-element-top-level-stat-label
#[derive(Deserialize)]
struct LocationXmlRepr {
    pub dimension: Vec<Dimension>,
}

/// A [design space dimension]].
///
/// [design space location]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#location-element-source
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Dimension {
    /// Name of the axis, e.g. Weight.
    pub name: String,
    /// Value on the axis in user coordinates.
    pub uservalue: Option<f32>,
    /// Value on the axis in designcoordinates.
    pub xvalue: Option<f32>,
    /// Separate value for anisotropic interpolations.
    pub yvalue: Option<f32>,
}

impl DesignSpaceDocument {
    /// Load a designspace.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<DesignSpaceDocument, DesignSpaceLoadError> {
        let reader = BufReader::new(File::open(path).map_err(DesignSpaceLoadError::Io)?);
        quick_xml::de::from_reader(reader).map_err(DesignSpaceLoadError::DeError)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pretty_assertions::assert_eq;

    use crate::designspace::{AxisMapping, Dimension};

    use super::DesignSpaceDocument;

    fn dim_name_xvalue(name: &str, xvalue: f32) -> Dimension {
        Dimension { name: name.to_string(), uservalue: None, xvalue: Some(xvalue), yvalue: None }
    }

    #[test]
    fn read_single_wght() {
        let ds = DesignSpaceDocument::load(Path::new("testdata/single_wght.designspace")).unwrap();
        assert_eq!(1, ds.axes.len());
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
        let ds = DesignSpaceDocument::load(Path::new("testdata/wght.designspace")).unwrap();
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
}
