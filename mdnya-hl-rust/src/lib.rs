// use tree_sitter_highlight::HighlightConfiguration;

// use mdnya_hl::HLLib;

// mod ts_rust_mod;
// use ts_rust_mod::*;

// use mdnya_hl_rust_gen::*;

const RAW_CONFIG_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rust.hlconfig"));

extern "C" { fn tree_sitter_rust() -> usize; }

// unsafe extern "C" fn load_hl_config() -> *const HighlightConfiguration {
//     let lang = ts_rust_mod::language_rust();
//     mdnya_hl::load_hlconfig(RAW_CONFIG_DATA, lang).unwrap()
// }

#[derive(Debug)]
#[repr(C)]
pub struct HLLib {
    // pub name: *const u8,
    // pub name_size: usize,
    pub config_data: *const u8,
    pub config_data_size: usize,
    pub language: usize,//tree_sitter::Language,
}

#[no_mangle]
pub extern "C" fn hl_lib() -> HLLib {
    HLLib {
        // name: ts_rust_mod::LANG_NAME.as_ptr(),
        // name_size: ts_rust_mod::LANG_NAME.len(),
        // get_config: load_hl_config,
        // get_config_data: RAW_CONFIG_DATA.as_ptr(),
        // get_language_ptr: ts_rust_mod::language_rust,
        config_data: RAW_CONFIG_DATA.as_ptr(),
        config_data_size: RAW_CONFIG_DATA.len(),
        language: unsafe { tree_sitter_rust() },
    }
}

// pub fn hl_static() -> &'static TSHLC {
//     unsafe {
//         let conf = load_hlconfig(RAW_CONFIG_DATA, &language_rust()).unwrap();
//         Box::leak(Box::new(conf))
//     }
// }
