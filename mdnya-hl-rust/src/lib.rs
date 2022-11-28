use tree_sitter_highlight::HighlightConfiguration;

use mdnya_hl::HLLib;
use mdnya_hl::CodeHighlighter;
use mdnya_hl::highlight;

use mdnya_hl_rust_gen::*;

const RAW_CONFIG_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rust.hlconfig"));

struct RustHighlighter {
    config: HighlightConfiguration,
}

impl CodeHighlighter for RustHighlighter {
    fn highlight(&self, text: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        highlight(text, &self.config)
    }
}

#[cfg(feature = "dynamic")]
#[no_mangle]
pub extern "C" fn hl_lib() -> HLLib {
    HLLib {
        config_data: RAW_CONFIG_DATA.as_ptr(),
        config_data_size: RAW_CONFIG_DATA.len(),
        language: language_rust(),
    }
}

#[cfg(feature = "static")]
pub fn hl_static() -> &'static impl CodeHighlighter {
    unsafe {
        let config = load_hlconfig(RAW_CONFIG_DATA, &language_rust()).unwrap();

        Box::leak(Box::new( RustHighlighter { config } ))
    }
}
