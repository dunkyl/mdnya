use std::path::PathBuf;

fn main() {
    
    let ts_md_path: &PathBuf = &["..", "tree-sitters", "tree-sitter-c-sharp", "src"].iter().collect();

    // println!("cargo:rerun-if-changed={:?}", ts_md_path);
    cc::Build::new()
        .include(ts_md_path)
        .file(ts_md_path.join("parser.c"))
        .file(ts_md_path.join("scanner.c"))
        .compile("tree-sitter-csharp");
}