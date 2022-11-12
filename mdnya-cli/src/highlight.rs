use std::{error::Error, collections::HashMap};

use lazy_static::lazy_static;

use crate::generated_lang;
use ts_pregen::load_hlconfig;

const CONFIG_RAW_DATA: &[u8] = include_bytes!("../../pregen/rust.hlconfig");

pub fn get_configuration(_lang_name: &str) -> &'static tree_sitter_highlight::HighlightConfiguration {
    let lang = generated_lang::language_rust();
    // let (_, config) = load_hlconfig(CONFIG_RAW_DATA, lang).unwrap();
    // println!("{}", config.query.pattern_count());
    
    // config.1
    // let config2 = 
    // tree_sitter_highlight::HighlightConfiguration::new(
    //     lang,
    //     generated_lang::HIGHLIGHT_QUERY_RUST,
    //     "",
    //     ""
    // ).unwrap()
    load_hlconfig(CONFIG_RAW_DATA, lang).unwrap().1
    // println!("{}", config2.query.pattern_count());

    // config2
}

pub fn highlight_code(source: &[u8], lang_name: &str) -> Result<Option<Vec<u8>>, Box<dyn Error>>{

    lazy_static! {
        static ref HL_CLASSES: Vec<String> = {
            generated_lang::HL_NAMES.iter().map(|s| s.to_string().replace('.', "-")).collect::<Vec<_>>()
        };

        static ref RUST_CONFIG: &'static tree_sitter_highlight::HighlightConfiguration = get_configuration("rust");

        static ref CONFIGS: HashMap<&'static str, tree_sitter_highlight::HighlightConfiguration> = generated_lang::initialize_configs();
    }
    
    let mut tshl = tree_sitter_highlight::Highlighter::new();
    let start_static = std::time::Instant::now();
    // let get = CONFIGS.get(lang_name);
    // let get = Some(&RUST_CONFIG);
    let get = Some(get_configuration(lang_name));
    let end_static = std::time::Instant::now();
    println!("  static load time: {:?}", end_static - start_static);
    if let Some(config) = get {
        let hl = tshl.highlight(&config, source, None, |_| None)?;
        let mut renderer = tree_sitter_highlight::HtmlRenderer::new();
        renderer.render(hl, source, &|hl| HL_CLASSES[hl.0].as_bytes())?;
        Ok(Some(renderer.html))
    } else {
        Ok(None)
    }
}
