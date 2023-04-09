extern crate mdnya;

#[test]
fn test() {
    let mut mdnya = mdnya::MDNya::new(false, Some("section".into()), 1, false);
    let input = std::fs::read_to_string("../test.md").unwrap();
    let output = Box::new(std::io::stdout());
    let _ = mdnya.render(&input, output).unwrap();
}

#[test]
fn readme() {
    let mut mdnya = mdnya::MDNya::new(false, None, 1, false);
    let input = include_str!("../../readme.md");
    let output = Box::new(std::io::stdout());
    let _ = mdnya.render(input, output).unwrap();
}