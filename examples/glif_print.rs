use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::{env, fs, io};

use failure::Error;
use quick_xml::{
    events::{attributes::Attribute, Event},
    Reader,
};

fn main() -> Result<(), io::Error> {
    let path = match env::args().skip(1).next().map(PathBuf::from) {
        Some(ref p) if p.exists() && p.extension() == Some(OsStr::new("glif")) => p.to_owned(),
        Some(ref p) => {
            eprintln!("path {:?} is not an existing .glif file, exiting", p);
            std::process::exit(1);
        }
        None => {
            eprintln!("Please supply a path to a glif file");
            std::process::exit(1);
        }
    };

    eprintln!("doing something with '{:?}'", &path);
    let xml = fs::read_to_string(&path)?;
    match print_tokens(&xml) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("error {}", e);
            std::process::exit(1);
        }
    }
}

fn print_tokens(xml: &str) -> Result<(), Error> {
    let mut reader = Reader::from_str(&xml);
    let mut buf = Vec::new();
    reader.trim_text(true);
    let mut level = 0;

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Decl(decl)) => {
                let version = decl.version()?;
                let version = std::str::from_utf8(&version)?;

                let slice: &[u8] = &[];
                let encoding = decl.encoding().unwrap_or(Ok(Cow::from(slice)))?;
                let encoding = std::str::from_utf8(&encoding)?;
                eprintln!("xml version {} encoding {}", version, encoding);
            }
            Ok(Event::Start(start)) => {
                let name = std::str::from_utf8(start.name())?;
                eprint!("{}<{}", spaces_for_level(level), name);
                for attr in start.attributes() {
                    let attr = attr?;
                    let key = std::str::from_utf8(&attr.key)?;
                    let value = attr.unescaped_value()?;
                    let value = reader.decode(&value)?;
                    eprint!(" {}=\"{}\"", key, value);
                }
                eprintln!(">");
                level += 1;
            }
            Ok(Event::End(end)) => {
                level -= 1;
                let name = std::str::from_utf8(end.name())?;
                eprintln!("{}</{}>", spaces_for_level(level), name);
            }
            Ok(Event::Empty(start)) => {
                let name = std::str::from_utf8(start.name())?;
                eprint!("{}<{}", spaces_for_level(level), name);
                for attr in start.attributes() {
                    let Attribute { key, value } = attr?;
                    let key = std::str::from_utf8(&key)?;
                    let value = std::str::from_utf8(&value)?;
                    eprint!(" {}=\"{}\"", key, value);
                }
                eprintln!("/>");
            }
            Ok(Event::Eof) => break,
            Ok(other) => eprintln!("{:?}", other),
            Err(e) => {
                eprintln!("error {:?}", e);
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
