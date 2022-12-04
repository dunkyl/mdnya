use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    
    let lib_path: &PathBuf = &["..", "tree-sitters", "tree-sitter-c-sharp", "src"].iter().collect();

    // println!("cargo:rerun-if-changed={:?}", lib_path);
    cc::Build::new()
        .include(lib_path)
        .file(lib_path.join("parser.c"))
        .file(lib_path.join("scanner.c"))
        .compile("tree-sitter-csharp");
    Ok(())
}