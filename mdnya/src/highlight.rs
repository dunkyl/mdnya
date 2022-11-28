use std::error::Error;

pub trait CodeHighlighter {
    fn highlight(&self, text: &[u8]) -> Result<Vec<u8>, Box<dyn Error>>;
}
