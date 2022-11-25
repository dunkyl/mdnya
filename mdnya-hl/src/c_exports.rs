// use tree_sitter_highlight::HighlightConfiguration as TSHLC;

#[derive(Debug)]
#[repr(C)]
pub struct HLLib {
    // pub name: *const u8,
    // pub name_size: usize,
    pub config_data: *const u8,
    pub config_data_size: usize,
    pub language: tree_sitter::Language,
}

// impl HLLib {

//     fn get_config(&self) -> &TSHLC {
//         todo!()
//     }

// }

// impl HLLib {
//     // fn get_name(&self) -> *const str {
//     //     unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.name, self.name_size)) }
//     // }

//     fn get_config_data(&self) -> &[u8] {
//         unsafe { std::slice::from_raw_parts(self.config_data, self.config_data_size) }
//     }
// }