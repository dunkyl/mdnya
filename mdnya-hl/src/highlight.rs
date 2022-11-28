use std::error::Error;

pub use tree_sitter_highlight::HighlightConfiguration as TSHLC;

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

//, names: &[&str]
pub fn configure_tshlc(lang: tree_sitter::Language, hql: &str) -> Result<TSHLC, Box<dyn std::error::Error>> {
    let mut conf = TSHLC::new(lang, hql, "", "" )?;
    conf.configure(HL_NAMES);
    Ok(conf)
}

pub fn highlight(source: &[u8], cfg: &TSHLC ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut tshl = tree_sitter_highlight::Highlighter::new();
    let hl = tshl.highlight(cfg, source, None, |_| None)?;
    let mut renderer = tree_sitter_highlight::HtmlRenderer::new();
    renderer.render(hl, source, &|hl| HL_CLASSES[hl.0].as_bytes())?;
    Ok(renderer.html)
}

pub trait CodeHighlighter {
    fn highlight(&self, text: &[u8]) -> Result<Vec<u8>, Box<dyn Error>>;
}

