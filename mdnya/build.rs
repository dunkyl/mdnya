use std::path::PathBuf;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    
    let ts_md_path = &["..", "mdnya-hl-langs", "tree-sitters", "tree-sitter-markdown", "src"].iter().collect::<PathBuf>();

    // println!("cargo:rerun-if-changed={:?}", ts_md_path);
    cc::Build::new()
        .include(ts_md_path)
        .file(ts_md_path.join("parser.c"))
        .file(ts_md_path.join("scanner.cc"))
        .compile("tree-sitter-markdown");
    Ok(())
}