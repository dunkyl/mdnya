fn main() {
    println!("cargo:rustc-link-search=tree-sitter-builds");
    println!("cargo:rustc-link-lib=tree-sitter-bash");
}