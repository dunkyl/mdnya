use tree_sitter::Language;

use mdnya_hl::{configure_tshlc, generate_hlconfig, PregeneratedHLConfig};

extern "C" { fn tree_sitter_c_sharp() -> Language; }
pub fn get_language() -> Language { unsafe { tree_sitter_c_sharp() } }
pub const HL_QUERY: &str = include_str!(
    "../../tree-sitters/tree-sitter-c-sharp/queries/highlights.scm");

pub fn get_config_data() -> PregeneratedHLConfig {

    let config = configure_tshlc(get_language(), HL_QUERY).unwrap();

    generate_hlconfig(config)

}