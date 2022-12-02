use std::error::Error;

pub use tree_sitter_highlight::HighlightConfiguration as TSHLC;

use crate::dynamic;

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

// TODO: constify
const HL_CLASSES: &[&str] = &[
    "attribute",
    "constant",
    "function-builtin",
    "function",
    "keyword",
    "operator",
    "property",
    "punctuation",
    "punctuation-bracket",
    "punctuation-delimiter",
    "string",
    "string-special",
    "tag",
    "type",
    "type-builtin",
    "variable",
    "variable-builtin",
    "variable-parameter",
    "number",
    "comment",
];

pub enum TSHLang {
    Dynamic(dynamic::LoadedHLLib),
    Static(&'static str, Vec<&'static str>, &'static TSHLC),
}

//, names: &[&str]
#[cfg(feature = "generate")]
pub fn configure_tshlc(lang: tree_sitter::Language, hql: &str) -> Result<TSHLC, Box<dyn std::error::Error>> {
    let mut conf = TSHLC::new(lang, hql, "", "" )?;
    conf.configure(HL_NAMES);
    Ok(conf)
}

fn highlight(source: &[u8], cfg: &TSHLC ) -> Result<String, Box<dyn std::error::Error>> {
    let mut tshl = tree_sitter_highlight::Highlighter::new();
    let hl = tshl.highlight(cfg, source, None, |_| None)?;
    let mut renderer = tree_sitter_highlight::HtmlRenderer::new();
    renderer.render(hl, source, &|hl| HL_CLASSES[hl.0].as_bytes())?;
    Ok(String::from_utf8(renderer.html)?)
}

pub trait CodeHighlighter {
    fn highlight(&self, text: &[u8]) -> Result<String, Box<dyn Error>>;
    fn aliases(&self) -> Vec<&str>;
    fn name(&self) -> &str;
}

impl CodeHighlighter for TSHLang {
    fn highlight(&self, text: &[u8]) -> Result<String, Box<dyn Error>> {
        match self {
            TSHLang::Dynamic(hl) => highlight(text, hl.get_config()),
            TSHLang::Static(_, _, hl) => highlight(text, hl),
        }
    }
    fn aliases(&self) -> Vec<&str> {
        match self {
            TSHLang::Dynamic(hl) => hl.aliases(),
            TSHLang::Static(_, aliases, _) => 
                aliases.iter().map(|s| s.as_ref()).collect(),
        }
    }
    fn name(&self) -> &str {
        match self {
            TSHLang::Dynamic(hl) => hl.name(),
            TSHLang::Static(name, _, _) => name,
        }
    }
}

