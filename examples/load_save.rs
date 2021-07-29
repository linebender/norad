//! A small program that times the loading and saving of a UFO file.

use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use norad::Font;

static HELP: &str = "
USAGE:
    open_ufo PATH [OUTPATH]

If an OUTPATH is provided, the UFO will be saved after opening.
";

macro_rules! exit_err {
    ($($arg:tt)*) => ({
        eprintln!($($arg)*);
        eprintln!("{}", HELP);
        std::process::exit(1);
    })
}

fn main() {
    let args = Args::get_from_env_or_exit();

    let start = Instant::now();
    let ufo = Font::load(&args.path).expect("failed to load file");

    let duration = start.elapsed();
    let time_str = format_time(duration);
    let font_name = ufo
        .font_info
        .as_ref()
        .and_then(|f| f.family_name.clone())
        .unwrap_or_else(|| "an unnamed font".into());

    println!("loaded {} glyphs from {} in {}.", ufo.glyph_count(), font_name, time_str);

    if let Some(outpath) = args.outpath {
        let start = Instant::now();
        ufo.save(outpath).expect("failed to save UFO");
        let duration = start.elapsed();
        let time_str = format_time(duration);
        println!("wrote UFO to disk in {}", time_str);
    }
}

fn format_time(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    format!("{}.{}s", secs, millis)
}

struct Args {
    path: PathBuf,
    outpath: Option<PathBuf>,
}

impl Args {
    fn get_from_env_or_exit() -> Self {
        let mut args = env::args().skip(1);
        let path = match args.next().map(PathBuf::from) {
            Some(ref p) if p.exists() && p.extension() == Some(OsStr::new("ufo")) => p.to_owned(),
            Some(ref p) => exit_err!("path {:?} is not an existing .ufo file, exiting", p),
            None => exit_err!("Please supply a path to a .ufo file"),
        };

        let outpath = args.next().map(PathBuf::from);
        if outpath.as_ref().map(|p| p.exists()).unwrap_or(false) {
            exit_err!("outpath {} already exists, exiting", outpath.unwrap().display());
        }

        Args { path, outpath }
    }
}
