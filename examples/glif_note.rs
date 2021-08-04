use norad::*;

static NOTE_TEXT: &str = r#"
    def test:
        print("love it")

"#;
fn main() {
    let mut glif = Glyph::new_named("A");
    glif.note = Some(NOTE_TEXT.into());

    let encoded = glif.encode_xml().unwrap();
    let string = String::from_utf8(encoded).unwrap();
    print!("{}", string);
}
