mod c_exports;
mod c_imports;
mod conversions;
mod highlight;

pub use highlight::CodeHighlighter;
pub use highlight::configure_tshlc;
pub use highlight::TSHLang;

pub use c_exports::*;
pub use conversions::*;

#[cfg(feature = "dynamic")]
mod dynamic;

#[cfg(feature = "dynamic")]
pub use highlight::load_hl_lib;