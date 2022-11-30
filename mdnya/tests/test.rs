extern crate mdnya;

#[test]
fn test() {
    let mdnya = mdnya::MDNya::new(false, None, 1, false);
    let input = include_bytes!("../../test.md");
    let mut output = std::io::stdout();
    let _ = mdnya.render(input, &mut output);
}