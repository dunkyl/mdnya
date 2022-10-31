use std::path::{Path, PathBuf};

fn main() {

    let lang_paths = Path::new("langs").read_dir().unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.is_dir()).collect::<Vec<_>>();
    
    for lang in lang_paths {
        let lang_name = lang.file_name().unwrap().to_str().unwrap();
        let lang_src = ["langs", lang_name, "src"].iter().collect::<PathBuf>();
        println!("cargo:rerun-if-changed={}", lang_src.display());
        cc::Build::new()
            .include(&lang_src)
            .file(lang_src.join("parser.c"))
            .file(lang_src.join("scanner.cc"))
            .compile(lang_name);
    }
}