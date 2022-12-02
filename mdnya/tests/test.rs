extern crate mdnya;

#[test]
fn test() {
    let mut mdnya = mdnya::MDNya::new(false, Some("section".into()), 1, false);
    mdnya.add_highlighter(mdnya_hl_rust::hl_static());
    let input = include_bytes!("../../test.md");
    let output = Box::new(std::io::stdout());
    let _ = mdnya.render(input, output);
}

#[test]
fn readme() {
    let mdnya = mdnya::MDNya::new(false, None, 1, false);
    let input = include_bytes!("../../readme.md");
    let output = Box::new(std::io::stdout());
    let _ = mdnya.render(input, output);
}