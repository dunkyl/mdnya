use std::path::PathBuf;

use libloading::*;
use tree_sitter_highlight::HighlightConfiguration as TSHLC;

use mdnya_hl::{HLLib, load_hlconfig};

extern "C" { fn tree_sitter_markdown() -> tree_sitter::Language; }

pub fn language_markdown() -> tree_sitter::Language {
    unsafe { tree_sitter_markdown() }
}

pub struct LoadedHLLib {
    // --- kept for lifetimes
    _lib: Library,
    _hl: HLLib,
    // ---
    // impl contains raw pointers to data in previous members
    config: TSHLC,
}

impl LoadedHLLib {
    pub fn get_config(&self) -> &TSHLC {
        &self.config
    }
}

pub fn load_hl_lib<'a>(path: PathBuf) -> Result<LoadedHLLib, Box<dyn std::error::Error>> {
    unsafe {
        let lib = Library::new(path)?;
        let hl = {
            let hl_lib: Symbol<unsafe extern "C" fn() -> HLLib> = lib.get(b"hl_lib\0")?;
            hl_lib()
        };

        let config = load_hlconfig(
            std::slice::from_raw_parts(hl.config_data, hl.config_data_size),
            &hl.language,
        )?;

        Ok(LoadedHLLib {
            _lib: lib,
            _hl: hl,
            config
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {

        println!("{}", std::env::current_dir().unwrap().display());

        let rust_lib = "mdnya_hl_rust.dll";
        let rust_lib_path = std::env::current_dir().unwrap().join("..").join("target").join("debug").join(rust_lib);
        println!("{}", rust_lib_path.display());
        let result = load_hl_lib(rust_lib.into());
        // assert_eq!(result.unwrap().name(), "rust");
        println!("{:?}", result.unwrap().config.query.pattern_count());

    }
}
