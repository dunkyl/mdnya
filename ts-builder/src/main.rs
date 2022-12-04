use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Options {
    /// path to the Tree Sitter language
    #[clap(name="input-dir")]
    input_dir: PathBuf,
}

fn main() {

    let opts = Options::parse();

    let lang_name = opts.input_dir.file_name().unwrap().to_str().unwrap().splitn(3, '-').collect::<Vec<&str>>()[2].replace("-", "");

    let src = opts.input_dir.join("src");
    let output = PathBuf::from(".").join(format!("tree-sitter-builds/tree-sitter-{}.dll", lang_name));

    let scanner = 
        if src.join("scanner.c").exists() {
            src.join("scanner.c")
        } else {
            src.join("scanner.cc")
        };

    let res = std::process::Command::new("cl")
        .arg(format!("/I{}", src.to_string_lossy()))
        .arg(scanner)
        .arg(src.join("parser.c"))
        .arg("/LD")
        .arg(format!("/Fe:{}", output.to_string_lossy()))
        .arg("/link")
        .output();

    println!("res: {:?}", res);

}