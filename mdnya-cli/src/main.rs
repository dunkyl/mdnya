use std::{path::PathBuf, io::Write, error::Error};

use mdnya::MDNya;
use clap::Parser as clapParser;

#[derive(clapParser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Options {
    /// Markdown file to convert
    #[clap(name="input")]
    input_file: PathBuf,

    /// HTML file to write to (default: <input>.html)
    #[clap(short, long="output")]
    output_file: Option<PathBuf>,

    /// Include closing tags for <p> and <li> elements
    #[clap(short, long="close-all-tags")]
    close_all_tags: bool,

    /// Surround document in tags, such as 'html,body' or article. Comma separated
    #[clap(long="wrap-tags", value_parser,  value_delimiter = ',')]
    wrap_document: Option<Vec<String>>,

    /// Surround text after each heading in a tag
    #[clap(long="wrap-sections")]
    wrap_sections: Option<String>,

    // #[clap(short, long, name="enclose-sections", value_parser,  value_delimiter = ',')]
    // enclose_sections: Option<String>,

    /// Show times
    #[clap(short, long)]
    verbose: bool,

    /// Increase base heading level to this number
    #[clap(short='l', long="heading-level", default_value="1")]
    heading_level: u8,

    /// Change to this extension for default output. 
    #[clap(long="ext")]
    output_ext: Option<String>,

    /// Don't add id attributes to headings
    #[clap(long="no-ids")]
    no_ids: bool,

    // TODO: Add option for yielding tags (#blah) present in the document
    //  ^ Like in Obsidian
    /// Don't add id attributes to headings
    #[clap(long="detect-tags")]
    detect_tags: bool,
}

fn main() -> Result<(), Box<dyn Error>> {

    let opts = Options::parse();

    let source_code = std::fs::read(&opts.input_file).unwrap();

    let output = 
        if let Some(ref path) = opts.output_file {
            if path == &PathBuf::from("stdout") {
                Box::new(std::io::stdout()) as Box<dyn Write>
            }
            else {
                Box::new(std::fs::File::create(path)?) as Box<dyn Write>
            }
        }
        else {
            let stem_opt = opts.input_file.file_stem();
            let out_dir = opts.input_file.as_path().parent().unwrap_or(std::path::Path::new("."));
            if let Some(stem) = stem_opt {
                let stem = stem.to_str().unwrap().to_owned();
                let ext = match opts.output_ext {
                    Some(ref ext) => ext,
                    None => ".html",
                };
                let output_path = out_dir.join(stem + ext);
                Box::new(std::fs::File::create(output_path)?) as Box<dyn Write>
            }
            else {
                return Err("default output file (replace .md with .html) expects a filename with a stem".into())
            }
        };
    
    let time_write_start = std::time::Instant::now();

    let mut mdnya = MDNya::new(false, Some("section".into()), 1, false);
    mdnya.add_highlighter(mdnya_hl_rust::hl_static());
    mdnya.render(&source_code, output)?;

    let write_elapsed = time_write_start.elapsed();
    if opts.verbose {
        println!("write time: {:?}", write_elapsed);
    }

    Ok(())
}
