//! Build script to generate our glyph name lookup table.

use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use phf_codegen;

const OUT_FILE: &str = "glyph_names_codegen.rs";

fn main() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join(OUT_FILE);
    let mut file = BufWriter::new(File::create(&path).unwrap());
    let names = include_str!("resources/aglfn.txt");
    let mut map = phf_codegen::Map::new();
    for line in names.lines().filter(|l| !l.starts_with('#')) {
        let mut split = line.split(';');
        match (split.next(), split.next(), split.next(), split.next()) {
            (Some(cpoint), Some(ps_name), Some(_unic_name), None) => {
                let cpoint = u32::from_str_radix(cpoint, 16).unwrap();
                let cpoint = char::try_from(cpoint).unwrap();
                let name: Box<str> = format!("\"{}\"", ps_name).into();
                let name = Box::leak(name);
                map.entry(cpoint, name);
            }
            _ => panic!("malformed line: '{}'", line),
        }
    }

    writeln!(&mut file, "static GLYPH_NAMES: phf::Map<char, &'static str> = \n{};\n", map.build())
        .unwrap();
}
