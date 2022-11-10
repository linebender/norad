//! Reading and writing designspace files.

#![deny(rustdoc::broken_intra_doc_links)]

use quick_xml::de::from_reader;
use std::{fs::File, io::BufReader, path::Path};

use crate::error::DesignSpaceLoadError;

/// A [designspace]].
///
/// [designspace]: https://fonttools.readthedocs.io/en/latest/designspaceLib/index.html
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename = "designspace")]
pub struct DesignSpaceDocument {
    pub format: f32,
    pub axes: Axes,
    pub sources: Sources,
    pub instances: Instances,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename = "axes")]
pub struct Axes {
    pub axis: Vec<Axis>,
}

/// A [axis]].
///
/// [axis]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#axis-element
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename = "axis")]
pub struct Axis {
    pub name: String,
    pub tag: String,
    pub default: f32,
    pub hidden: Option<bool>,
    pub minimum: Option<f32>,
    pub maximum: Option<f32>,
    pub values: Option<Vec<f32>>,
    pub map: Vec<AxisMapping>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename = "map")]
pub struct AxisMapping {
    pub input: f32,
    pub output: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename = "sources")]
pub struct Sources {
    pub source: Vec<Source>,
}

/// A [source]].
///
/// [source]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#id25
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename = "source")]
pub struct Source {
    pub familyname: Option<String>,
    pub stylename: Option<String>,
    pub name: String,
    pub filename: String,
    pub layer: Option<String>,
    pub location: Location,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename = "instances")]
pub struct Instances {
    pub instance: Vec<Instance>,
}

/// An [instance]].
///
/// [instance]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#instance-element
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Instance {
    pub familyname: String,
    pub stylename: String,
    pub name: String,
    pub filename: String,
    pub postscriptfontname: Option<String>,
    pub stylemapfamilyname: Option<String>,
    pub stylemapstylename: Option<String>,
    pub location: Location,
}

/// A [design space location]].
///
/// [design space location]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#location-element-source
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Location {
    pub dimension: Vec<Dimension>,
}

/// A [design space dimension]].
///
/// [design space location]: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#location-element-source
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Dimension {
    pub name: String,
    pub uservalue: Option<f32>,
    pub xvalue: Option<f32>,
    pub yvalue: Option<f32>,
}

impl DesignSpaceDocument {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<DesignSpaceDocument, DesignSpaceLoadError> {
        let reader = BufReader::new(File::open(path).map_err(DesignSpaceLoadError::Io)?);
        Ok(from_reader(reader).map_err(DesignSpaceLoadError::DeError)?)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pretty_assertions::assert_eq;

    use crate::designspace::{AxisMapping, Dimension};

    use super::DesignSpaceDocument;

    #[test]
    fn read_single_wght() {
        let ds = DesignSpaceDocument::load(Path::new("testdata/single_wght.designspace")).unwrap();
        assert_eq!(1, ds.axes.axis.len());
        assert_eq!(vec![AxisMapping { input: 400., output: 100. }], ds.axes.axis[0].map);
        assert_eq!(1, ds.sources.source.len());
        let weight_100 = Dimension {
            name: "Weight".to_string(),
            uservalue: None,
            xvalue: Some(100.),
            yvalue: None,
        };
        assert_eq!(vec![weight_100.clone()], ds.sources.source[0].location.dimension);
        assert_eq!(1, ds.instances.instance.len());
        assert_eq!(vec![weight_100], ds.instances.instance[0].location.dimension);
    }
}
