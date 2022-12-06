use std::io::Read;

extern crate mdnya;

#[test]
fn test() {
    let mut mdnya = mdnya::MDNya::new(false, Some("section".into()), 1, false);
    mdnya.add_highlighter(mdnya_hl_rust::hl_static());
    let input = std::fs::read_to_string("../test.md").unwrap();
    let output = Box::new(std::io::stdout());
    let _ = mdnya.render(input.as_bytes(), output);
}

#[test]
fn readme() {
    let mdnya = mdnya::MDNya::new(false, None, 1, false);
    let input = include_bytes!("../../readme.md");
    let output = Box::new(std::io::stdout());
    let _ = mdnya.render(input, output);
}