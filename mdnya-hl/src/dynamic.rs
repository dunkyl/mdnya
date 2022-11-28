use std::path::PathBuf;

use libloading::*;
use tree_sitter_highlight::HighlightConfiguration as TSHLC;

use crate::c_exports::HLLib;
use crate::conversions::load_hlconfig;

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