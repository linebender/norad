use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use norad::Ufo;

fn main() {
    let (input, output) = get_path_or_exit();

    let start_load = Instant::now();
    let mut my_ufo = Ufo::load(input).unwrap();
    let duration_load = start_load.elapsed();
    let duration_load_str = format_time(duration_load);

    my_ufo.meta.creator = "org.linebender.norad".to_string();

    let start_write = Instant::now();
    my_ufo.save(output).unwrap();
    let duration_write = start_write.elapsed();
    let duration_write_str = format_time(duration_write);

    println!("Loaded UFO in {}, wrote it in {}.", duration_load_str, duration_write_str);
}

fn get_path_or_exit() -> (PathBuf, PathBuf) {
    let mut args = env::args().skip(1);

    let input = match args.next().map(PathBuf::from) {
        Some(ref p) if p.exists() && p.extension() == Some(OsStr::new("ufo")) => p.to_owned(),
        _ => {
            eprintln!("Please supply a path to a UFO to read from");
            std::process::exit(1);
        }
    };
    let output = match args.next().map(PathBuf::from) {
        Some(ref p) if p.extension() == Some(OsStr::new("ufo")) => p.to_owned(),
        _ => {
            eprintln!("Please supply a path to write the UFO to");
            std::process::exit(1);
        }
    };

    (input, output)
}

fn format_time(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    format!("{}.{}s", secs, millis)
}
