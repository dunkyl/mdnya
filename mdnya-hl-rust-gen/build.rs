use std::error::Error;
use std::path::PathBuf;
use std::io::Write;

const LANG_NAME: &str = "rust";
const TS_LIB_NAME: &str = "tree-sitter-rust";
const TS_LIB_PATH: &[&str] = &["..", "langs", TS_LIB_NAME, "src"];

fn codegen() -> Result<(), Box<dyn Error>> {
    let mut ts_rust_mod = std::fs::File::create(PathBuf::from(std::env::var("OUT_DIR")?).join("ts_gen.rs"))?;

    writeln!(ts_rust_mod, "use tree_sitter::Language;")?;

    writeln!(ts_rust_mod, "pub const LANG_NAME: &str = \"{LANG_NAME}\";")?;

    writeln!(ts_rust_mod, "extern \"C\" {{ fn tree_sitter_{LANG_NAME}() -> Language; }}")?;
    writeln!(ts_rust_mod, "pub fn language_{LANG_NAME}() -> Language {{ unsafe {{ tree_sitter_{LANG_NAME}() }} }}")?;

    let hl_scm_path_relative = [TS_LIB_PATH, &["..", "queries", "highlights.scm"]].concat().iter().collect::<PathBuf>();
    let hl_scm_path = hl_scm_path_relative.canonicalize()?;

    writeln!(ts_rust_mod, "pub const HL_QUERY: &str = include_str!(r\"{}\");", hl_scm_path.to_str().unwrap())?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    
    let lib_path = &TS_LIB_PATH.iter().collect::<PathBuf>();

    println!("cargo:rerun-if-changed={:?}", lib_path);
    cc::Build::new()
        .include(lib_path)
        .file(lib_path.join("parser.c"))
        .file(lib_path.join("scanner.c"))
        .compile(TS_LIB_NAME);
    codegen()?;

    println!("{}", std::env::var("TARGET").unwrap());
    Ok(())
}