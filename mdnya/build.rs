use std::path::PathBuf;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    
    let ts_md_path = &["..", "langs", "tree-sitter-markdown", "src"].iter().collect::<PathBuf>();

    // println!("cargo:rerun-if-changed={:?}", ts_md_path);
    let mut build = cc::Build::new();
    build.include(ts_md_path);
    build.file(ts_md_path.join("parser.c"));
    build.file(ts_md_path.join("scanner.cc"));
    build.compile("tree-sitter-markdown");
    Ok(())
}