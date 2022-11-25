use std::error::Error;

pub use tree_sitter_highlight::HighlightConfiguration as TSHLC;

mod ts_gen;

pub use ts_gen::language_rust;
use ts_gen::*;

use mdnya_hl::{generate_hlconfig, PregeneratedHLConfig};
pub use mdnya_hl::{HLLib, load_hlconfig, configure_tshlc};

pub fn generate_config_data() -> Result<PregeneratedHLConfig, Box<dyn Error>> {

    let config = configure_tshlc(language_rust(), HL_QUERY)?;

    let config_data = generate_hlconfig(LANG_NAME, config);

    Ok(config_data)

}