use std::{error::Error, collections::HashMap};

use lazy_static::lazy_static;

use crate::generated_lang;

pub fn highlight_code(source: &[u8], lang_name: &str) -> Result<Option<Vec<u8>>, Box<dyn Error>>{

    lazy_static! {
        static ref HL_CLASSES: Vec<String> = {
            generated_lang::HL_NAMES.iter().map(|s| s.to_string().replace('.', "-")).collect::<Vec<_>>()
        };

        static ref CONFIGS: HashMap<&'static str, tree_sitter_highlight::HighlightConfiguration> = generated_lang::initialize_configs();
    }
    
    let mut tshl = tree_sitter_highlight::Highlighter::new();
    if let Some(config) = CONFIGS.get(lang_name) {
        let hl = tshl.highlight(&config, source, None, |_| None)?;
        let mut renderer = tree_sitter_highlight::HtmlRenderer::new();
        renderer.render(hl, source, &|hl| HL_CLASSES[hl.0].as_bytes())?;
        Ok(Some(renderer.html))
    } else {
        Ok(None)
    }
}
