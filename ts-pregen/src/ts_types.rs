use std::sync::Arc;

use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize)]
pub struct Placeholder {
    _unused: [u8; 4],
}

#[derive(Serialize, Deserialize)]
#[repr(transparent)]
pub struct PtrPlaceholder {
    _unused: [u8; std::mem::size_of::<&Placeholder>()],
}

pub mod c_types {
    use std::fmt;

    use serde::{ser::{SerializeStruct, SerializeSeq}, de::{Visitor, SeqAccess, MapAccess}, Deserializer};

    use super::*;

    #[repr(C)]
    #[derive(Serialize, Deserialize, Clone, Copy)]
    struct TSSymbol(u16);

    #[repr(C)]
    #[derive(Serialize, Deserialize, Clone, Copy)]
    struct TSFieldId(u16);

    #[derive(Serialize, Deserialize)]
    #[repr(C)]
    struct TSFieldMapEntry {
        field_id: TSFieldId,
        child_index: u8,
        inherited: bool,
    }

    #[derive(Serialize, Deserialize)]
    #[repr(C)]
    struct TSFieldMapSlice {
        index: u16,
        length: u16,
    }

    #[derive(Serialize, Deserialize)]
    #[repr(C)]
    struct TSSymbolMetadata {
        visible: bool,
        named: bool,
        supertype: bool,
    }

    #[derive(Serialize, Deserialize)]
    #[repr(C)]
    struct Entry {
        count: u8,
        reusable: bool,
    }

    #[derive(Serialize, Deserialize)]
    #[repr(C)]
    pub struct TSLanguage {}

    #[derive(Serialize, Deserialize, Clone, Copy)]
    #[repr(C)]
    struct Slice {
        offset: u32,
        length: u32,
    }

    #[derive(Clone, Copy)]
    #[repr(C)]
    pub struct TSArray<T> {
        pub contents: *const T,
        pub size: u32,
        pub capacity: u32,
    }

    impl<T> Serialize for TSArray<T>
    where
        T: Serialize + Copy,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            // let mut state = serializer.serialize_struct("TSArray", 3)?;
            // state.serialize_field("size", &self.size)?;
            // state.serialize_field("capacity", &self.capacity)?;

            // let mut contents = Vec::with_capacity(self.size as usize);
            // for i in 0..self.size {
            //     contents.push(unsafe { *self.contents.offset(i as isize) });
            // }

            // state.serialize_field("contents", &contents)?;

            // state.end()
            let mut seq = serializer.serialize_seq(Some(self.size as usize))?;
            for i in 0..self.size {
                seq.serialize_element(&unsafe { *self.contents.offset(i as isize) })?;
            }
            seq.end()
        }
    }

    impl<'de, T> Deserialize<'de> for TSArray<T>
    where
        T: Deserialize<'de >,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {

            struct TSArrayVisitor<T> { _unused: std::marker::PhantomData<T> }

            impl<'de, T> Visitor<'de> for TSArrayVisitor<T>
            where
                T: Deserialize<'de>
            {
                type Value = TSArray<T>;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a TSArray")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let mut contents = Vec::new();
                    while let Some(elem) = seq.next_element()? {
                        contents.push(elem);
                    }

                    let size = contents.len() as u32;

                    Ok(TSArray {
                        contents: contents.leak().as_ptr(),
                        size,
                        capacity: size,
                    })
                }
            } 
            deserializer.deserialize_seq(TSArrayVisitor{ _unused: std::marker::PhantomData{}})
        }
    }


    #[derive(Serialize, Deserialize)]
    #[repr(C)]
    struct SymbolTable {
        characters: TSArray<u8>,
        slices: TSArray<Slice>,
    }

    const MAX_STEP_CAPTURE_COUNT: usize = 3;

    #[derive(Serialize, Deserialize, Clone, Copy)]
    #[repr(C)]
    pub struct QueryStep {
        symbol: TSSymbol,
        supertype_symbol: TSSymbol,
        field: TSFieldId,
        capture_ids: [u16; MAX_STEP_CAPTURE_COUNT],
        depth: u16,
        alternative_index: u16,
        negated_field_list_id: u16,
        _flags_1: u8,
        _flags_2: u8,
    }
    #[derive(Serialize, Deserialize, Clone, Copy)]
    #[repr(C)]
    struct CaptureQuantifiers(TSArray<u8>);
    #[derive(Serialize, Deserialize, Clone, Copy)]
    #[repr(C)]
    pub struct PatternEntry {
        step_index: u16,
        pattern_index: u16,
        is_rooted: bool,
    }
    #[derive(Serialize, Deserialize, Clone, Copy)]
    #[repr(C)]
    struct QueryPattern {
        steps: Slice,
        predicate_steps: Slice,
        start_byte: u32,
    }
    #[derive(Serialize, Deserialize, Clone, Copy)]
    #[repr(C)]
    struct StepOffset {
        byte_offset: u32,
        step_index: u16,
    }

    #[derive(Serialize, Deserialize, Clone, Copy)]
    #[repr(C)]
    enum TSQueryPredicateStepType {
        TSQueryPredicateStepTypeDone,
        TSQueryPredicateStepTypeCapture,
        TSQueryPredicateStepTypeString,
    }
    #[derive(Serialize, Deserialize, Clone, Copy)]
    #[repr(C)]
    struct TSQueryPredicateStep {
        type_: TSQueryPredicateStepType,
        value_id: u32,
    }

    #[derive(Serialize, Deserialize)]
    #[repr(C)]
    pub struct TSQuery {
        captures: SymbolTable,
        capture_quantifiers: TSArray<CaptureQuantifiers>,
        predicate_values: SymbolTable,
        pub steps: TSArray<QueryStep>,
        pub pattern_map: TSArray<PatternEntry>,
        predicate_steps: TSArray<TSQueryPredicateStep>,
        patterns: TSArray<QueryPattern>,
        step_offsets: TSArray<StepOffset>,
        negated_fields: TSArray<TSFieldId>,
        string_buffer: TSArray<char>,
        pub language: usize, //*const TSLanguage,
        pub wildcard_root_pattern_count: u16,
    }
}

#[repr(transparent)]
#[derive(Serialize)]
pub struct Language<'a>(&'a Placeholder);

#[derive(Serialize, Deserialize)]
pub struct RegexPlaceholder {
    ro: Arc<Placeholder>,
    pool: Box<Placeholder>,
}

#[derive(Serialize, Deserialize)]
pub enum CaptureQuantifier {
    Zero,
    ZeroOrOne,
    ZeroOrMore,
    One,
    OneOrMore,
}

#[derive(Serialize, Deserialize)]
pub enum TextPredicate {
    CaptureEqString(u32, String, bool),
    CaptureEqCapture(u32, u32, bool),
    CaptureMatchString(u32, RegexPlaceholder, bool),
}

#[derive(Serialize, Deserialize)]
pub enum QueryPredicateArg {
    Capture(u32),
    String(Box<str>),
}

#[derive(Serialize, Deserialize)]
pub struct QueryPredicate {
    pub operator: Box<str>,
    pub args: Vec<QueryPredicateArg>,
}

#[derive(Serialize, Deserialize)]
pub struct QueryProperty {
    pub key: Box<str>,
    pub value: Option<Box<str>>,
    pub capture_id: Option<usize>,
}

// impl Serialize for *const c_types::TSQuery {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer {
//         todo!()
//     }
// }

#[derive(Serialize, Deserialize)]
pub struct PtrTSQuery {
    _unused: [u8; std::mem::size_of::<*const c_types::TSQuery>()]
}

#[derive(Serialize, Deserialize)]
pub struct Query {
    pub ptr: usize,//,Box<c_types::TSQuery>,//&'a [u8],//c_types::TSQuery, //'static u8, //c_types::TSQuery,//PtrTSQuery,
    capture_names: Vec<String>,
    capture_quantifiers: Vec<Vec<CaptureQuantifier>>,
    pub text_predicates: Vec<Box<[TextPredicate]>>,
    property_settings: Vec<Box<[QueryProperty]>>,
    property_predicates: Vec<Box<[(QueryProperty, bool)]>>,
    general_predicates: Vec<Box<[QueryPredicate]>>,
}

#[derive(Serialize, Deserialize)]
pub struct Highlight(usize);

#[derive(Serialize, Deserialize)]
pub struct HighlightConfiguration {
    pub language: usize, //*const TSLanguage (aliased by transparent Langauge),
    pub query: Query,
    combined_injections_query: Option<Query>,
    locals_pattern_index: usize,
    highlights_pattern_index: usize,
    highlight_indices: Vec<Option<Highlight>>,
    non_local_variable_patterns: Vec<bool>,
    injection_content_capture_index: Option<u32>,
    injection_language_capture_index: Option<u32>,
    local_scope_capture_index: Option<u32>,
    local_def_capture_index: Option<u32>,
    local_def_value_capture_index: Option<u32>,
    local_ref_capture_index: Option<u32>,
}

pub struct Intermediate_HighlightConfiguration {
    pub language: tree_sitter::Language,
    pub query: tree_sitter::Query,
    pub combined_injections_query: Option<tree_sitter::Query>,
    pub locals_pattern_index: usize,
    pub highlights_pattern_index: usize,
    pub highlight_indices: Vec<Option<Highlight>>,
    pub non_local_variable_patterns: Vec<bool>,
    pub injection_content_capture_index: Option<u32>,
    pub injection_language_capture_index: Option<u32>,
    pub local_scope_capture_index: Option<u32>,
    pub local_def_capture_index: Option<u32>,
    pub local_def_value_capture_index: Option<u32>,
    pub local_ref_capture_index: Option<u32>,
}

impl HighlightConfiguration {

    pub unsafe fn convert_to_intermediate(self) -> Intermediate_HighlightConfiguration {
        let combined_injections_query = match self.combined_injections_query {
            Some(query) => Some(std::mem::transmute::<_, _>(query)),
            None => None,
        };
        
        Intermediate_HighlightConfiguration {
            language: std::mem::transmute::<_, _>(self.language),
            query: std::mem::transmute::<_, _>(self.query),
            combined_injections_query,
            locals_pattern_index: self.locals_pattern_index,
            highlights_pattern_index: self.highlights_pattern_index,
            highlight_indices: self.highlight_indices,
            non_local_variable_patterns: self.non_local_variable_patterns,
            injection_content_capture_index: self.injection_content_capture_index,
            injection_language_capture_index: self.injection_language_capture_index,
            local_scope_capture_index: self.local_scope_capture_index,
            local_def_capture_index: self.local_def_capture_index,
            local_def_value_capture_index: self.local_def_value_capture_index,
            local_ref_capture_index: self.local_ref_capture_index,
        }
    }

    pub unsafe fn convert_from_intermediate(other: Intermediate_HighlightConfiguration) -> Self {
        let combined_injections_query = match other.combined_injections_query {
            Some(query) => Some(std::mem::transmute::<_, _>(query)),
            None => None,
        };
        
        HighlightConfiguration {
            language: std::mem::transmute::<_, _>(other.language),
            query: std::mem::transmute::<_, _>(other.query),
            combined_injections_query,
            locals_pattern_index: other.locals_pattern_index,
            highlights_pattern_index: other.highlights_pattern_index,
            highlight_indices: other.highlight_indices,
            non_local_variable_patterns: other.non_local_variable_patterns,
            injection_content_capture_index: other.injection_content_capture_index,
            injection_language_capture_index: other.injection_language_capture_index,
            local_scope_capture_index: other.local_scope_capture_index,
            local_def_capture_index: other.local_def_capture_index,
            local_def_value_capture_index: other.local_def_value_capture_index,
            local_ref_capture_index: other.local_ref_capture_index,
        }
    }

    pub fn get_injections_data(&self) -> Option<Vec<u8>> {
        if let Some(query) = &self.combined_injections_query {
            let c_query = unsafe {
                let ptr = std::mem::transmute::<_, *const c_types::TSQuery>(query.ptr);
                &*ptr
            };
            Some(bincode::serialize(c_query).unwrap())
        } else {
            None
        }
    }

    pub fn get_injections_regex(&self) -> Vec<String> {
        if let Some(query) = &self.combined_injections_query {
            let mut regexes = vec![];
            for predicates in &query.text_predicates {
                for predicate in predicates.iter() {
                    match predicate {
                        TextPredicate::CaptureMatchString(_, re, _) => {
                            unsafe {
                                let regex = std::mem::transmute::<_, &regex::bytes::Regex>(re);
                                regexes.push(regex.as_str().into());
                            }
                        }
                        _ => ()
                    }
                }
            }
            regexes
        } else {
            vec![]
        }
    }

    pub fn add_injections_regex(&mut self, regexes_iter: &mut impl Iterator<Item = String>) {
        if let Some(query) = &mut self.combined_injections_query {
            for predicates in query.text_predicates.iter_mut() {
                for predicate in predicates.iter_mut() {
                    match predicate {
                        TextPredicate::CaptureMatchString(a, _placeholder, b) => {
                            unsafe {
                                let regex = regex::bytes::Regex::new(regexes_iter.next().unwrap().as_str()).unwrap();
    
                                let regex_placeholder =  std::mem::transmute::<_, RegexPlaceholder>(regex);
                                *predicate = TextPredicate::CaptureMatchString(*a, regex_placeholder, *b);
    
                            }
                        }
                        _ => ()
                    }
                }
            }
        }
    }

    pub fn set_injections_data(&mut self, data: &mut c_types::TSQuery) {
        if let Some(query) = &mut self.combined_injections_query {
            query.ptr = unsafe {
                std::mem::transmute::<_, usize>(data)
            };
        }
    }

}