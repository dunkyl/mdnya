mod c_exports;
mod c_imports;
mod conversions;
mod highlight;

pub use highlight::CodeHighlighter;
pub use highlight::configure_tshlc;
pub use highlight::highlight;

pub use c_exports::*;

pub use conversions::*;

#[cfg(feature = "dynamic")]
mod dynamic;

#[cfg(feature = "dynamic")]
impl CodeHighlighter for crate::dynamic::LoadedHLLib {
    fn highlight(&self, source: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        highlight::highlight(source, &self.get_config())
    }
}




