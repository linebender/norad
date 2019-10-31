//! A small program that loads a UFO file and prints the glyph count.

use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use norad::Ufo;

fn main() {
    let path = get_path_or_exit();

    let start = Instant::now();
    let ufo = Ufo::load(&path).expect("failed to load file");

    let duration = start.elapsed();
    let time_str = format_time(duration);
    let font_name = ufo
        .font_info
        .as_ref()
        .and_then(|f| f.family_name.clone())
        .unwrap_or_else(|| "an unnamed font".into());

    println!("loaded {} glyphs from {} in {}.", ufo.glyph_count(), font_name, time_str);
}

fn get_path_or_exit() -> PathBuf {
    match env::args().skip(1).next().map(PathBuf::from) {
        Some(ref p) if p.exists() && p.extension() == Some(OsStr::new("ufo")) => p.to_owned(),
        Some(ref p) => {
            eprintln!("path {:?} is not an existing .glif file, exiting", p);
            std::process::exit(1);
        }
        None => {
            eprintln!("Please supply a path to a glif file");
            std::process::exit(1);
        }
    }
}

fn format_time(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    format!("{}.{}s", secs, millis)
}
