use std::{error::Error, collections::HashMap};

use lazy_static::lazy_static;

pub trait CodeHighlighter {
    fn highlight(&self, text: &[u8]) -> Result<Vec<u8>, Box<dyn Error>>;
}

// use crate::generated_lang;
// use hlconfig_pregen::load_hlconfig;

// const CONFIG_DATA_RUST: &[u8] = include_bytes!("../../hlconfigs/rust.hlconfig");
// const CONFIG_DATA_CSHARP: &[u8] = include_bytes!("../../hlconfigs/c_sharp.hlconfig");

// lazy_static! {
//     static ref LANGS: HashMap<&'static str, tree_sitter::Language> = generated_lang::initialize_langs();
    
// }

pub fn get_configuration(lang_name: &str) -> Option<&'static tree_sitter_highlight::HighlightConfiguration> {
    
    // lazy_static!{
    //     // static ref RAW_CONFIGS: HashMap<&'static str, &'static [u8]> = {
    //     //     let mut configs = HashMap::<_, &[u8]>::new();
    //     //     configs.insert("rust", include_bytes!("../../hlconfigs/rust.hlconfig"));
    //     //     configs.insert("c_sharp", include_bytes!("../../hlconfigs/c_sharp.hlconfig"));
    //     //     configs.insert("bash", include_bytes!("../../hlconfigs/bash.hlconfig"));
    //     //     configs
    //     // };

    //     static ref CONFIGS: HashMap<&'static str, mdnya::LoadedHLLib> = {
    //         todo!()
    //     };
    // }

    let hl_lib = mdnya::load_hl_lib(format!("mdnya_hl_{}.dll", lang_name).into()).unwrap();
    let hl_lib = Box::leak(Box::new(hl_lib));

    let conf = hl_lib.get_config();

    // let conf = mdnya_hl_rust::hl_static();
    Some(conf)
    
    // let lang = LANGS.get(lang_name)?;
    // let data = RAW_CONFIGS.get(lang_name)?;
    // let (_, config) = load_hlconfig(data, *lang).unwrap();
    // println!("{}", config.query.pattern_count());
    
    // config.1
    // let config2 = 
    // tree_sitter_highlight::HighlightConfiguration::new(
    //     lang,
    //     generated_lang::HIGHLIGHT_QUERY_RUST,
    //     "",
    //     ""
    // ).unwrap()
    // Some(load_hlconfig(data, *lang).unwrap().1)
    // println!("{}", config2.query.pattern_count());

    // config2
}

pub fn highlight_code(source: &[u8], lang_name: &str) -> Result<Option<Vec<u8>>, Box<dyn Error>>{

    lazy_static! {
        static ref HL_CLASSES: Vec<String> = {
            [
                "attribute",
                "constant",
                "function.builtin",
                "function",
                "keyword",
                "operator",
                "property",
                "punctuation",
                "punctuation.bracket",
                "punctuation.delimiter",
                "string",
                "string.special",
                "tag",
                "type",
                "type.builtin",
                "variable",
                "variable.builtin",
                "variable.parameter",
                "number",
                "comment",
            ].iter().map(|s| s.to_string().replace('.', "-")).collect::<Vec<_>>()
        };

        // static ref RUST_CONFIG: &'static tree_sitter_highlight::HighlightConfiguration = get_configuration("rust");

        // static ref CONFIGS: HashMap<&'static str, tree_sitter_highlight::HighlightConfiguration> = generated_lang::initialize_configs();
    }
    
    let mut tshl = tree_sitter_highlight::Highlighter::new();
    let start_static = std::time::Instant::now();
    // let get = CONFIGS.get(lang_name);
    // let get = Some(&RUST_CONFIG);
    // let get = Some(get_configuration(lang_name));
    let end_static = std::time::Instant::now();
    println!("  static load time: {:?}", end_static - start_static);
    if let Some(config) = get_configuration(lang_name) {
        let hl = tshl.highlight(&config, source, None, |_| None)?;
        let mut renderer = tree_sitter_highlight::HtmlRenderer::new();
        renderer.render(hl, source, &|hl| HL_CLASSES[hl.0].as_bytes())?;
        Ok(Some(renderer.html))
    } else {
        Ok(None)
    }
}
