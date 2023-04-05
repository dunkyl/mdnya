extern crate mdnya;

#[test]
fn test() {
    let mut mdnya = mdnya::MDNya::new(false, Some("section".into()), 1, false);
    // mdnya.add_highlighter(mdnya_hl_rust::hl_static());
    let input = std::fs::read_to_string("../test.md").unwrap();
    let output = Box::new(std::io::stdout());
    let _ = mdnya.render(input.as_bytes(), output);
}

// #[test]
// fn dll() {
//     let mut mdnya = mdnya::MDNya::new(false, Some("section".into()), 1, false);
//     println!("hi0");
//     let rust_hl_dynamic = mdnya_hl::load_hl_lib("../../mdnya_hl_rust.dll");
//     println!("hi1");
//     // mdnya.add_highlighter(rust_hl_dynamic);
//     println!("hi2");
//     let input = std::fs::read_to_string("../test.md").unwrap();
//     let output = Box::new(std::io::stdout());
//     let _ = mdnya.render(input.as_bytes(), output);
//     println!("hi");
// }

#[test]
fn readme() {
    let mut mdnya = mdnya::MDNya::new(false, None, 1, false);
    let input = include_bytes!("../../readme.md");
    let output = Box::new(std::io::stdout());
    let _ = mdnya.render(input, output);
}