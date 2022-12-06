// use mdnya_hl::HLLib;
use mdnya_hl::TSHLang;
use mdnya_hl::load_hlconfig;

#[link(name="tree-sitter-csharp", kind="static")]
extern "C" { fn tree_sitter_c_sharp() -> tree_sitter::Language; }
fn get_language() -> tree_sitter::Language { unsafe { tree_sitter_c_sharp() } }

const RAW_CONFIG_DATA: &[u8] = include_bytes!(
    "../../../tree-sitter-builds/csharp.hlconfig"
);


// const RAW_CONFIG_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/csharp.hlconfig"));

const NAME: &str = "csharp";
const ALIASES : &[&str] = &["cs", "c#"];

#[cfg(feature = "dynamic")]
#[no_mangle]
pub extern "C" fn hl_lib() -> HLLib {
    HLLib {
        name: NAME.as_ptr(),
        name_size: NAME.len(),
        aliases: ALIASES.as_ptr() as *const *const u8,
        aliases_sizes: ALIASES.iter().map(|s| s.len()).collect::<Vec<_>>().as_ptr(),
        aliases_size: ALIASES.len(),
        config_data: RAW_CONFIG_DATA.as_ptr(),
        config_data_size: RAW_CONFIG_DATA.len(),
        language: get_language(),
    }
}

#[cfg(feature = "static")]
pub fn hl_static() -> TSHLang {
    unsafe {
        let config = load_hlconfig(RAW_CONFIG_DATA, &get_language()).unwrap();
        TSHLang::Static(NAME, ALIASES.iter().cloned().collect(), Box::leak(Box::new( config )))
    }
}
