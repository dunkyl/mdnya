use std::path::PathBuf;
use std::error::Error;

const TS_LIB_NAME: &str = "tree-sitter-rust";
const TS_LIB_PATH: &[&str] = &["..", "langs", TS_LIB_NAME, "src"];

trait WasiInclude {
    fn wasi_include(&mut self) -> &mut Self;
}

impl WasiInclude for cc::Build {
    fn wasi_include(&mut self) -> &mut Self {
        if std::env::var("TARGET").unwrap() == "wasm32-wasi" {
            std::env::set_var("AR", "llvm-ar");
            self.include(std::env::var("WASI_SDK_PATH").unwrap())
        } else {
            self
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let output_file_path = PathBuf::from(std::env::var("OUT_DIR")?).join("rust.hlconfig");
    let output_file = std::fs::File::create(output_file_path)?;
    let config_data = mdnya_hl_rust_gen::generate_config_data()?;

    bincode::serialize_into(output_file, &config_data)?;

    let lib_path = &TS_LIB_PATH.iter().collect::<PathBuf>();

    cc::Build::new()
        .include(lib_path)
        .wasi_include()
        .file(lib_path.join("parser.c"))
        .file(lib_path.join("scanner.c"))
        .compile(TS_LIB_NAME);

    Ok(())
}
