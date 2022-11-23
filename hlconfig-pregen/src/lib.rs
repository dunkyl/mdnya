use std::error::Error;

use serde::{Serialize, Deserialize};
use tree_sitter::Language;
use tree_sitter_highlight::HighlightConfiguration;
use ts_types::{c_types, RegexPlaceholder};

pub mod generated_lang;
pub mod ts_types;

#[derive(Serialize, Deserialize)]
pub struct PregeneratedHLConfig {
    pub name: String,
    pub config: ts_types::HighlightConfiguration,
    pub regexes: Vec<String>,
    pub query_data: Vec<u8>,
    pub combined_injections_query_data: Option<Vec<u8>>,
}

pub fn generate_hlconfig(name: &str, config: HighlightConfiguration) -> PregeneratedHLConfig {

    let unsafe_view_ = unsafe {
        std::mem::transmute::<_, ts_types::IntermediateHLConf>(config)
    };

    let unsafe_view = unsafe {
        ts_types::HighlightConfiguration::convert_from_intermediate(unsafe_view_)
    };

    let mut regexes = vec![];

    let ts_query = unsafe {
        let ptr = std::mem::transmute::<_, *const c_types::TSQuery>(unsafe_view.query.ptr);
        &*ptr
    };

    // let ts_query = Box::leak(Box::new(ts_query));

    for predicates in &unsafe_view.query.text_predicates {
        for predicate in predicates.iter() {
            match predicate {
                ts_types::TextPredicate::CaptureMatchString(_, re, _) => {
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
        name: name.into(),
        config: unsafe_view,
        regexes,
        query_data,
        combined_injections_query_data
    }
}

pub fn load_hlconfig(data: &[u8], language: Language) -> Result<(String, &'static HighlightConfiguration), Box<dyn Error>> {
    let language_ptr = unsafe {
        std::mem::transmute::<_, usize>(language)
    };

    let mut pregen_config: PregeneratedHLConfig = bincode::deserialize(data)?;

    pregen_config.config.language = language_ptr;
    let tsquery = Box::new(bincode::deserialize::<c_types::TSQuery>(&pregen_config.query_data)?);
    let tsquery = Box::<c_types::TSQuery>::leak(tsquery);
    tsquery.language = language_ptr;
    pregen_config.config.query.ptr = unsafe {
        std::mem::transmute::<_, usize>(tsquery)
    };

    if let Some(query_data) = &pregen_config.combined_injections_query_data {
        let comb_inj_query = Box::new(bincode::deserialize::<c_types::TSQuery>(query_data)?);
        let comb_inj_query = Box::<c_types::TSQuery>::leak(comb_inj_query);
        comb_inj_query.language = language_ptr;
        pregen_config.config.set_injections_data(comb_inj_query);
    }

    let mut regexes_iter = pregen_config.regexes.into_iter();

    for predicates in pregen_config.config.query.text_predicates.iter_mut() {
        for predicate in predicates.iter_mut() {
            match predicate {
                ts_types::TextPredicate::CaptureMatchString(a, _placeholder, b) => {
                    unsafe {
                        let regex = regex::bytes::Regex::new(regexes_iter.next().unwrap().as_str()).unwrap();

                        let regex_placeholder =  std::mem::transmute::<_, RegexPlaceholder>(regex);
                        *predicate = ts_types::TextPredicate::CaptureMatchString(*a, regex_placeholder, *b);

                    }
                }
                _ => ()
            }
        }
    }

    pregen_config.config.add_injections_regex(&mut regexes_iter);

    let name = pregen_config.name;

    let intermediate = unsafe {
        pregen_config.config.convert_to_intermediate()
    };

    let config = 
        unsafe {
            std::mem::transmute::<_, HighlightConfiguration>(intermediate)
        };
    
    let boxed = Box::new(config);

    Ok((name, Box::leak(boxed)))
}