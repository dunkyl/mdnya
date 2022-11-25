use std::{path::PathBuf, io::Write, error::Error};

use tree_sitter::Parser;
use clap::Parser as clapParser;

// use hlconfig_pregen::generated_lang;

// extern "C" { fn tree_sitter_markdown() -> tree_sitter::Language; }

mod mdnya;
mod highlight;

#[derive(clapParser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Options {
    /// Markdown file to convert
    #[clap(name="input")]
    input_file: PathBuf,

    /// HTML file to write to (default: <input>.html)
    #[clap(short, long, name="output")]
    output_file: Option<PathBuf>,

    /// Include closing tags for <p> and <li> elements
    #[clap(short, long, name="close-all-tags")]
    close_all_tags: bool,

    /// Surround document in tags, such as 'html,body' or article. Comma separated
    #[clap(long, name="wrap-tags", value_parser,  value_delimiter = ',')]
    wrap_document: Option<Vec<String>>,

    /// Surround text after each heading in a tag
    #[clap(long, name="wrap-sections")]
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
    no_ids: bool
}

fn main() -> Result<(), Box<dyn Error>> {

    // std::env::set_var("RUST_BACKTRACE", "1");

    let mut parser = Parser::new();

    let opts = Options::parse();

    let source_code = std::fs::read(&opts.input_file).unwrap();

    let mut output_writer = 
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

    parser.set_language(::mdnya::language_markdown()).unwrap();

    let time_parse_start = std::time::Instant::now();
    let tree = parser.parse(source_code.as_slice(), None).unwrap();
    let root_node = tree.root_node();
    let parse_elapsed = time_parse_start.elapsed();
    if opts.verbose {
        println!("parse time: {:?}", parse_elapsed);
    }

    let time_write_start = std::time::Instant::now();
    
    let mut cur = root_node.walk();
    let mut putter = mdnya::HtmlHelper {
        is_inline: false,
        indent_level: 0,
        close_tags: opts.close_all_tags,
        extra_heading_level: opts.heading_level,
        wrap_sections: opts.wrap_sections,
        last_heading_level: 0,
        last_elem_was_header: false,
        omit_header_id: opts.no_ids,
        inside_section: false,
    };
    if let Some(tags) = &opts.wrap_document {
        for tag in tags {
            putter.start_tag(&mut output_writer, tag, &[], true)?;
            putter.indent_level += 1;
        }
    }

    println!("{}", root_node.to_sexp());

    cur.goto_first_child();
    mdnya::render_into(
        source_code.as_slice(),
        &mut cur, 
        &mut putter,
        &mut output_writer
    )?;
    if let Some(tags) = &opts.wrap_document {
        for tag in tags {
            putter.end_tag(&mut output_writer, tag)?;
            putter.indent_level -= 1;
        }
    }

    let write_elapsed = time_write_start.elapsed();
    if opts.verbose {
        println!("write time: {:?}", write_elapsed);
    }

    Ok(())
}
