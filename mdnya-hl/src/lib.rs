mod c_exports;
mod c_imports;
mod conversions;

pub use tree_sitter_highlight::HighlightConfiguration as TSHLC;

pub use c_exports::*;

pub use conversions::*;

const HL_NAMES: &[&str] = &[
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
];

//, names: &[&str]
pub fn configure_tshlc(lang: tree_sitter::Language, hql: &str) -> Result<TSHLC, Box<dyn std::error::Error>> {
    let mut conf = TSHLC::new(lang, hql, "", "" )?;
    conf.configure(HL_NAMES);
    Ok(conf)
}


