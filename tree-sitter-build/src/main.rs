use clap::Parser;
use std::{path::PathBuf, os::windows::process::CommandExt, process::exit};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Options {
    /// path to the Tree Sitter language
    #[clap(name="input-dir")]
    input_dir: PathBuf,
}

fn main() {

    let opts = Options::parse();

    let lang_name_dashes = opts.input_dir.file_name().unwrap().to_str().unwrap().splitn(3, '-').collect::<Vec<&str>>()[2];
    let lang_name_underscores = lang_name_dashes.replace("-", "_");
    let lang_name = lang_name_dashes.replace("-", "");

    let src = opts.input_dir.join("src");
    let output_dir = PathBuf::from(".").join("tree-sitter-builds");
    let output = output_dir.join(format!("tree-sitter-{}.dll", lang_name));

    let scanner = 
        if src.join("scanner.c").exists() {
            src.join("scanner.c")
        } else {
            src.join("scanner.cc")
        };

    let cl_args = [
        "cl".into(),
        format!("/I{}", src.to_string_lossy()),
        scanner.to_string_lossy().into(),
        src.join("parser.c").to_string_lossy().into(),
        "/LD".into(),
        format!("/Fe:{}", output.to_string_lossy()),
        "/link".into()
    ];
    let cl_cmd = cl_args.iter().cloned().collect::<Vec<_>>().join(" ");

    println!("cl_cmd: {cl_cmd}");

    let vcvars = r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat";

    let build_result = std::process::Command::new("cmd")
        .raw_arg("/c")
        .raw_arg(format!(r#""{vcvars}" && {cl_cmd}"#))
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output();

    match build_result {
        Ok(output) if output.status.success() => {
            println!("Build succeeded");
        },
        Ok(output) => exit(output.status.code().unwrap_or(1)),
        Err(_) => exit(1)
    }

    let hash_path = output_dir.join(format!("tree-sitter-{}.sha256", lang_name));
    let hash = sha256::try_digest(output.as_ref()).unwrap();
    std::fs::write(hash_path, hash).unwrap();

    let library = unsafe {
        libloading::Library::new(output).unwrap()
    };

    let language =
        unsafe {
            let lang_fn_name = format!("tree_sitter_{lang_name_underscores}");
            println!("lang_fn_name: {lang_fn_name}");
            let get_language = library.get
                ::<unsafe extern "C" fn() -> tree_sitter::Language>(lang_fn_name.as_bytes()).unwrap_or_else(|_| {
                    panic!("Could not find function '{}' in built library!", lang_fn_name)
                });
            get_language()
        };
    
    println!("{}", language.field_count());

    
    use mdnya_hl::{configure_tshlc, generate_hlconfig};

    let hl_query_path = opts.input_dir.join("queries").join("highlights.scm");
    let hl_query = std::fs::read_to_string(&hl_query_path).unwrap_or_else(|_|
        panic!("Could not read query file at {:?}", hl_query_path)
    );

    let config = configure_tshlc(language, &hl_query).unwrap_or_else(|_| 
        panic!("Could not configure tree-sitter-hl-config for language '{}'", lang_name_dashes)
    );

    let pregen = generate_hlconfig(config);

    bincode::serialize_into(
        std::fs::File::create(
            output_dir.join(format!("{lang_name}.hlconfig"))
        ).unwrap(), &pregen
    ).unwrap();

}