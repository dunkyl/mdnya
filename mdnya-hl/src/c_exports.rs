// use tree_sitter_highlight::HighlightConfiguration as TSHLC;

#[derive(Debug)]
#[repr(C)]
pub struct HLLib {
    pub name: *const u8,
    pub name_size: usize,
    pub aliases: *const *const u8,
    pub aliases_sizes: *const usize,
    pub aliases_size: usize,
    pub config_data: *const u8,
    pub config_data_size: usize,
    pub language: tree_sitter::Language,
}