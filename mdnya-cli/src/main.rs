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

    justlogfox::verbose_verbose_verbose();

    justlogfox::set_crate_color!(justlogfox::CssColors::Pink);

    justlogfox::log_trace!("Close all tags: {}", (opts.close_all_tags));

    let source_code = std::fs::read(&opts.input_file).unwrap();

    let output = 
        if let Some(ref path) = opts.output_file {
            
            if path == &PathBuf::from("stdout") {
                justlogfox::log_trace!("output to stdout");
                Box::new(std::io::BufWriter::new(std::io::stdout())) as Box<dyn Write>
            }
            else {
                justlogfox::log_trace!("output to file {:?}", path);
                Box::new(std::io::BufWriter::new(std::fs::File::create(path)?)) as Box<dyn Write>
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
                justlogfox::log_trace!("output to renamed default {:?}", output_path);
                Box::new(std::io::BufWriter::new(std::fs::File::create(output_path)?)) as Box<dyn Write>
            }
            else {
                return Err("default output file (replace .md with .html) expects a filename with a stem".into())
            }
        };
    
    let time_write_start = std::time::Instant::now();

    let mut mdnya = MDNya::new(opts.close_all_tags, opts.wrap_sections, opts.heading_level, opts.no_ids);
    
    // let source_code = std::str::from_utf8(&source_code).unwrap();

    // mdnya.render(&source_code, output)?;

    let write_elapsed = time_write_start.elapsed();
    justlogfox::log_debug!("mdnya new() took {:?}", write_elapsed);

    let time_write_start2 = std::time::Instant::now();
    
    let source_str = std::str::from_utf8(&source_code).unwrap();

    mdnya.render(source_str, output)?;

    // pulldown_cmark::html::write_html(output, parser).unwrap();

    let write_elapsed2 = time_write_start2.elapsed();
    justlogfox::log_debug!("mdnya render() took {:?}", write_elapsed2);

    Ok(())
}
