use std::io::BufWriter;

extern crate mdnya;

#[test]
fn test() {
    let input = include_str!("../../test.md");
    let mut output = BufWriter::new(vec![]);
    let options = mdnya::MdnyaOptions::new(false, Some("section".into()), None, 1, true);
    let _ = mdnya::render_markdown(input, &mut output, options).unwrap();
    assert!(!output.buffer().is_empty());
}

#[test]
fn readme() {
    let input = include_str!("../../readme.md");
    let mut output = BufWriter::new(vec![]);
    let options = mdnya::MdnyaOptions::new(false, Some("section".into()), None, 1, true);
    let _ = mdnya::render_markdown(input, &mut output, options).unwrap();
    assert!(!output.into_inner().unwrap().is_empty());
}

#[test]
fn list() {
    let input = "- a\n- b\n- c\n";
    let mut output = BufWriter::new(vec![]);
    let options = mdnya::MdnyaOptions::new(false, None, None, 1, true);
    let _ = mdnya::render_markdown(input, &mut output, options).unwrap();
    let expected = "<ul>\n    <li>a\n    <li>b\n    <li>c\n</ul>\n";
    assert_eq!(String::from_utf8_lossy(&output.into_inner().unwrap()).to_string().as_str(), expected);
}