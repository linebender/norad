//! A little tool for normalizing UFOs.
//!
//! It will scrub layer and lib data of a UFO in an opinionated way, as done
//! in the Cantarell font project, to show a real-world script.
//!
//! Call like `cargo run --release --example normalize some.ufo another.ufo`.

fn main() {
    for arg in std::env::args().skip(1) {
        let mut ufo = match norad::Font::load(&arg) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Loading UFO failed: {e}");
                std::process::exit(1);
            }
        };

        // Prune all non-foreground layers.
        let default_layer_name = ufo.layers.default_layer().name().clone();
        let to_remove: Vec<_> =
            ufo.layers.names().filter(|l| *l != &default_layer_name).cloned().collect();
        for layer_name in to_remove {
            ufo.layers.remove(&layer_name);
        }

        // Prune the foreground layer's lib.
        let default_layer = ufo.default_layer_mut();
        default_layer.lib.retain(|k, &mut _| {
            k.starts_with("public.") || k.starts_with("com.schriftgestaltung.layerId")
        });

        // Prune all glyphs' libs.
        for glyph in default_layer.iter_mut() {
            glyph.lib.retain(|k, &mut _| {
                (k.starts_with("public.")
                    || k.starts_with("com.schriftgestaltung.")
                    || k == "com.schriftgestaltung.componentsAlignment")
                    && k != "public.markColor"
            });
        }

        // Prune the UFO lib.
        ufo.lib.retain(|k, &mut _| {
            k.starts_with("public.")
                || k.starts_with("com.github.googlei18n.ufo2ft.")
                || k == "com.schriftgestaltung.appVersion"
                || k == "com.schriftgestaltung.fontMasterID"
                || k == "com.schriftgestaltung.customParameter.GSFont.disablesLastChange"
                || k == "com.schriftgestaltung.customParameter.GSFontMaster.paramArea"
                || k == "com.schriftgestaltung.customParameter.GSFontMaster.paramDepth"
                || k == "com.schriftgestaltung.customParameter.GSFontMaster.paramOver"
        });

        ufo.meta.creator = Some("org.linebender.norad".to_string());
        if let Err(e) = ufo.save(arg) {
            eprintln!("Saving UFO failed: {e}");
            std::process::exit(1);
        }
    }
}
