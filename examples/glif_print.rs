//! A little tool for debugging glyph parsing
//!
//! You pass this a glyph, and it tries to load it. It will then write it
//! back out to xml, and print this to stdout; you can redirect this to a file
//! in order to inspect how a given glyph would be serialized.
//!
//! Afterwards it will print the xml tree to stderr, which may be useful when
//! debugging parse errors.

use std::ffi::OsStr;
use std::path::PathBuf;
use std::{env, fs, io};

use failure::Error;
use quick_xml::{
    events::{attributes::Attribute, Event},
    Reader,
};

use norad::Glyph;

fn main() -> Result<(), io::Error> {
    let path = match env::args().nth(1).map(PathBuf::from) {
        Some(ref p) if p.exists() && p.extension() == Some(OsStr::new("glif")) => p.to_owned(),
        Some(ref p) => {
            eprintln!("path {p:?} is not an existing .glif file, exiting");
            std::process::exit(1);
        }
        None => {
            eprintln!("Please supply a path to a glif file");
            std::process::exit(1);
        }
    };

    let glyph = Glyph::load(&path).unwrap();
    let to_xml = glyph.encode_xml().unwrap();
    let to_xml = String::from_utf8(to_xml).unwrap();
    // redirect this to a file to get the rewritten glif
    println!("{to_xml}");

    let xml = fs::read_to_string(&path)?;
    match print_tokens(&xml) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("error {e}");
            std::process::exit(1);
        }
    }
}

fn print_tokens(xml: &str) -> Result<(), Error> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    reader.config_mut().trim_text(true);
    let mut level = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Decl(decl)) => {
                let version = decl.version()?;
                let version = std::str::from_utf8(&version)?;

                let encoding = decl.encoding().transpose()?.unwrap_or_default();
                let encoding = std::str::from_utf8(&encoding)?;

                eprintln!("xml version {version} encoding {encoding}");
            }
            Ok(Event::Start(start)) => {
                let name = start.name();
                let name = std::str::from_utf8(name.as_ref())?;
                eprint!("{}<{}", spaces_for_level(level), name);
                for attr in start.attributes() {
                    let attr = attr?;
                    let key = std::str::from_utf8(attr.key.as_ref())?;
                    let value = attr.unescape_value()?;
                    eprint!(" {key}=\"{value}\"");
                }
                eprintln!(">");
                level += 1;
            }
            Ok(Event::End(end)) => {
                level -= 1;
                let name = end.name();
                let name = std::str::from_utf8(name.as_ref())?;
                eprintln!("{}</{}>", spaces_for_level(level), name);
            }
            Ok(Event::Empty(start)) => {
                let name = start.name();
                let name = std::str::from_utf8(name.as_ref())?;
                eprint!("{}<{}", spaces_for_level(level), name);
                for attr in start.attributes() {
                    let Attribute { key, value } = attr?;
                    let key = std::str::from_utf8(key.as_ref())?;
                    let value = std::str::from_utf8(&value)?;
                    eprint!(" {key}=\"{value}\"");
                }
                eprintln!("/>");
            }
            Ok(Event::Eof) => break,
            Ok(other) => eprintln!("{other:?}"),
            Err(e) => {
                eprintln!("error {e:?}");
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn spaces_for_level(level: usize) -> &'static str {
    let spaces = "                                                                                                                                                  ";
    let n_spaces = (level * 2).min(spaces.len());
    &spaces[..n_spaces]
}
