use std::error::Error;

use serde::{Serialize, Deserialize};
use tree_sitter::Language;
use tree_sitter_highlight::HighlightConfiguration as TSHLC;

use crate::c_imports::{c_types, TextPredicate, IntermediateHLConf, HighlightConfiguration as CompatHLC};

#[derive(Serialize, Deserialize, Debug)]
pub struct PregeneratedHLConfig {
    // pub name: String,
    pub config: CompatHLC,
    pub regexes: Vec<String>,
    pub query_data: Vec<u8>,
    pub combined_injections_query_data: Option<Vec<u8>>,
}

#[cfg(feature = "generate")]
pub fn generate_hlconfig(config: TSHLC) -> PregeneratedHLConfig {

    let unsafe_view_ = unsafe {
        std::mem::transmute::<_, IntermediateHLConf>(config)
    };

    let unsafe_view = unsafe {
        CompatHLC::convert_from_intermediate(unsafe_view_)
    };

    let mut regexes = vec![];

    let ts_query = unsafe {
        let ptr = std::mem::transmute::<_, *const c_types::TSQuery>(unsafe_view.query.ptr);
        &*ptr
    };

    for predicates in &unsafe_view.query.text_predicates {
        for predicate in predicates.iter() {
            match predicate {
                TextPredicate::CaptureMatchString(_, re, _) => {
                    unsafe {
                        let regex = std::mem::transmute::<_, &regex::bytes::Regex>(re);
                        // println!(" {:?}", regex);
                        regexes.push(regex.as_str().into());
                    }
                }
                _ => ()
            }
        }
    }

    regexes.extend(unsafe_view.get_injections_regex());

    let query_data = bincode::serialize(ts_query).unwrap();

    let combined_injections_query_data = unsafe_view.get_injections_data();

    println!("ts query wildcards {}", ts_query.wildcard_root_pattern_count);
    // println!("ts query steps {}, {}", ts_query.steps.size, ts_query.steps.capacity);
    println!("ts pattern_maps count {}, {}", ts_query.pattern_map.size, ts_query.pattern_map.capacity);

    PregeneratedHLConfig {
        config: unsafe_view,
        regexes,
        query_data,
        combined_injections_query_data
    }
}

unsafe fn load_query(data: &[u8], lang_ptr: usize) -> Result<&'static c_types::TSQuery, Box<dyn Error>> {
    let q = Box::new(bincode::deserialize::<c_types::TSQuery>(&data)?);
    let q = Box::<c_types::TSQuery>::leak(q);
    q.language = lang_ptr;
    Ok(q)
}

pub unsafe fn load_hlconfig(data: &[u8], language: &Language) -> Result<TSHLC, Box<dyn Error>> 
{
    // transparently wrapped pointer to C struct TSLanguage
    let language_ptr = std::mem::transmute::<_, usize>(*language);

    let mut pregen_config: PregeneratedHLConfig = bincode::deserialize(data)?;

    // re-insert pointers
    pregen_config.config.language = language_ptr;
    pregen_config.config.query.ptr =
        std::mem::transmute::<_, usize>(
            load_query(&pregen_config.query_data, language_ptr)?
        );
    if let Some(query_data) = &pregen_config.combined_injections_query_data {
        let comb_inj_query = load_query(query_data, language_ptr)?;
        pregen_config.config.set_injections_data(comb_inj_query);
    }

    // re-insert regexes
    let mut regex_strs = pregen_config.regexes.into_iter();

    pregen_config.config.add_query_regex(&mut regex_strs);
    pregen_config.config.add_injections_regex(&mut regex_strs);

    // generic-compatible converted config
    let intermediate = pregen_config.config.convert_to_intermediate();

    Ok(std::mem::transmute::<_, TSHLC>(intermediate))
    
}