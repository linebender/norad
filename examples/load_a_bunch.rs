use norad::Font;

fn main() {
    let _ = Font::load("/Users/rofls/dev/projects/fontville/fontfiles/NotoSans-Bold.ufo").unwrap();
    let _ =
        Font::load("/Users/rofls/dev/projects/fontville/fontfiles/NotoSans-Regular.ufo").unwrap();
    let _ = Font::load("/Users/rofls/dev/projects/fontville/fontfiles/NotoSans-Light.ufo").unwrap();
}
