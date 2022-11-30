use std::path::PathBuf;

use libloading::*;
use tree_sitter_highlight::HighlightConfiguration as TSHLC;

use crate::c_exports::HLLib;
use crate::conversions::load_hlconfig;

pub struct LoadedHLLib {
    name: String,
    aliases: Vec<String>,
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
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn aliases(&self) -> Vec<&str> {
        self.aliases.iter().map(|s| s.as_str()).collect()
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

        let name = String::from_utf8_lossy(
            std::slice::from_raw_parts(hl.name, hl.name_size)
        ).to_string();

        let alias_sizes = std::slice::from_raw_parts(hl.aliases_sizes, hl.aliases_size);

        let aliases = std::slice::from_raw_parts(hl.aliases, hl.aliases_size)
            .iter()
            .zip(alias_sizes.iter())
            .map(|(alias, size)| {
                String::from_utf8_lossy(std::slice::from_raw_parts(*alias, *size))
                    .to_string()
            })
            .collect();

        Ok(LoadedHLLib {
            name,
            aliases,
            _lib: lib,
            _hl: hl,
            config
        })
    }
}