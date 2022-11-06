use std::path::{Path, PathBuf};
use std::io::Write;

// define a function which at runtime configures the parsers for each language
fn code_gen(lang_names: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut lang_rs = std::fs::File::create(PathBuf::from(std::env::var("OUT_DIR")?).join("lang.rs"))?;
    writeln!(lang_rs, "use tree_sitter::Language;\n")?;
    writeln!(lang_rs, "use std::collections::HashMap;\n")?;
    let mut highlight_queries = vec![];
    for lang in &lang_names {
        let lang_up = lang.to_uppercase();
        writeln!(lang_rs, "extern \"C\" {{ fn tree_sitter_{lang}() -> Language; }}")?;
        writeln!(lang_rs, "pub fn language_{lang}() -> Language {{ unsafe {{ tree_sitter_{lang}() }} }}")?;
        // let highlight_query_path = Path::new("langs").join(format!("tree-sitter-{lang}")).join("queries").join("highlights.scm");
        let highlight_query_path = ["..", "langs", format!("tree-sitter-{lang}").as_str(), "queries", "highlights.scm"].iter().collect::<PathBuf>();
        if highlight_query_path.exists() {
            // println!("{:?}", );
            let path_string = PathBuf::from(std::env::current_dir()?).join(highlight_query_path);
            // let path_string = Path::new("..").join("..").join("..").join("..").join("..").join(highlight_query_path);
            let path_string = path_string.to_str().unwrap().replace('\\', "\\\\");
            writeln!(lang_rs, "pub const HIGHLIGHT_QUERY_{lang_up}: &str = include_str!(\"{path_string}\");")?;
            highlight_queries.push(lang);
        }
    }

    writeln!(lang_rs, "
pub const HL_NAMES: &[&str] = &[
    \"attribute\",
    \"constant\",
    \"function.builtin\",
    \"function\",
    \"keyword\",
    \"operator\",
    \"property\",
    \"punctuation\",
    \"punctuation.bracket\",
    \"punctuation.delimiter\",
    \"string\",
    \"string.special\",
    \"tag\",
    \"type\",
    \"type.builtin\",
    \"variable\",
    \"variable.builtin\",
    \"variable.parameter\",
];
    ")?;


    writeln!(lang_rs, "pub fn initialize_configs() -> HashMap<&'static str, tree_sitter_highlight::HighlightConfiguration> {{")?;
    writeln!(lang_rs, "  let mut configs = HashMap::new();")?;
    for lang in highlight_queries {
        let lang_up = lang.to_uppercase();
        writeln!(lang_rs, "  let mut config_{lang} = tree_sitter_highlight::HighlightConfiguration::new(
            language_{lang}(),
            HIGHLIGHT_QUERY_{lang_up},
            \"\",
            \"\"
        ).unwrap();")?;
        writeln!(lang_rs, "  config_{lang}.configure(HL_NAMES);")?;
        writeln!(lang_rs, "  configs.insert(\"{lang}\", config_{lang});")?;
    }
    writeln!(lang_rs, "  configs")?;
    writeln!(lang_rs, "}}")?;
    Ok(())
}

fn main() {

    let lang_paths = Path::new("../langs").read_dir().unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.is_dir()).collect::<Vec<_>>();
    
    for lang in &lang_paths {
        let lang_name = lang.file_name().unwrap().to_str().unwrap();
        let lang_src = ["..", "langs", lang_name, "src"].iter().collect::<PathBuf>();
        println!("cargo:rerun-if-changed={}", lang_src.display());
        let mut build = cc::Build::new();
        build.include(&lang_src);
        build.file(lang_src.join("parser.c"));
        if lang_src.join("scanner.cc").exists() {
            build.file(lang_src.join("scanner.cc"));
        } else {
            build.file(lang_src.join("scanner.c"));
        }
        build.compile(lang_name);
    }

    // tree_sitter_<blank>
    let langs = lang_paths.iter().map(|p| { p.as_os_str().to_string_lossy().split('-').last().unwrap().to_owned() }).collect::<Vec<_>>();
    code_gen(langs).unwrap();
}