use std::{path::PathBuf, io::Write, error::Error, collections::HashMap};

use mdnya::MdnyaOptions;
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

    /// JSON file for metadata
    #[clap(short, long="metadata")]
    metadata_file: Option<PathBuf>,

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

fn main() -> mdnya::Result<()> {

    let opts = Options::parse();

    justlogfox::set_log_level(justlogfox::LogLevel::Debug);

    justlogfox::set_crate_color!(justlogfox::CssColors::Pink);

    justlogfox::log_trace!("Close all tags: {}", (opts.close_all_tags));

    let mut output =
        match opts.output_file {
            Some(ref path) if path == &PathBuf::from("stdout") => {
                justlogfox::log_trace!("output to stdout");
                Box::new(std::io::BufWriter::new(std::io::stdout())) as Box<dyn Write>
            }
            Some(ref path) => {
                justlogfox::log_trace!("output to file {:?}", path);
                Box::new(std::io::BufWriter::new(std::fs::File::create(path)?)) 
            }
            None => {
                let renamed = opts.input_file.with_extension(
                                opts.output_ext.unwrap_or("html".to_owned()));
                justlogfox::log_trace!("output to renamed default {:?}", renamed);
                Box::new(std::io::BufWriter::new(std::fs::File::create(renamed)?))
            }
        };
    
    let load_start = std::time::Instant::now();

    let source_code = std::fs::read_to_string(&opts.input_file)?;
    let highligher = Box::new(mdnya::StarryHighlighter::new(HashMap::new()));
    let options = MdnyaOptions::new(opts.close_all_tags, opts.wrap_sections, opts.heading_level, !opts.no_ids);

    justlogfox::log_debug!("setup took {:?}", (load_start.elapsed()));

    let render_start = std::time::Instant::now();

    let meta = mdnya::render_markdown(source_code, &mut output, options, highligher)?;

    justlogfox::log_debug!("mdnya render() took {:?}", (render_start.elapsed()));

    if let Some(path) = &opts.metadata_file {
        let json = serde_json::to_string_pretty(&meta)?;
        std::fs::write(path, json)?;
    }

    Ok(())
}
