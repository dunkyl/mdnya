use std::{path::PathBuf, io::Write};

use mdnya::MdnyaOptions;
use clap::Parser as clapParser;

#[derive(clapParser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Options {
    /// Markdown file to convert
    #[clap(name="input")]
    input_file: PathBuf,

    /// HTML file to write to (default: <input>.html).
    /// Can be 'stdout' to write to stdout.
    #[clap(short, long="output")]
    output_file: Option<PathBuf>,

    /// Output JSON file for metadata. If passed without a value, the default is <input>.json
    #[clap(short, long="meta")]
    metadata_file: Option<Option<PathBuf>>,

    /// Include closing tags for <p> and <li> elements
    #[clap(short, long="close-all-tags")]
    close_all_tags: bool,

    /// Surround document in tags, such as 'html,body' or article. Comma separated
    #[clap(long="doc-tags", value_parser,  value_delimiter = ',')]
    document_tags: Option<Vec<String>>,

    /// Surround text after each heading in a tag
    #[clap(long="section-tags")]
    section_tags: Option<String>,

    /// Show extra information
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
}

fn convert_one(input: &PathBuf, output: &mut Box<dyn Write>, meta_output: Option<Box<dyn Write>>, options: &MdnyaOptions) -> mdnya::Result<()> {
    let source_code = std::fs::read_to_string(input)?;
    let meta = mdnya::render_markdown(source_code, output, options.clone())?;
    if let Some(mut meta_output) = meta_output {
        let json = serde_json::to_string_pretty(&meta)?;
        write!(meta_output, "{}", json)?;
    }
    Ok(())
}

fn open_write(path: &PathBuf) -> Box<dyn Write> {
    Box::new(std::io::BufWriter::new(std::fs::File::create(path).unwrap())) as Box<dyn Write>
}

fn main() -> mdnya::Result<()> {

    let opts = Options::parse();

    if opts.verbose {
        justlogfox::set_log_level(justlogfox::LogLevel::Debug);
    } else {
        justlogfox::set_log_level(justlogfox::LogLevel::Warn);
    }

    justlogfox::set_crate_color!(justlogfox::CssColors::Pink);

    justlogfox::log_trace!("Close all tags: {}", (opts.close_all_tags));

    let input_files =
        if opts.input_file.is_dir() {
            justlogfox::log_trace!("input is directory {:?}", opts.input_file);
            let files = std::fs::read_dir(&opts.input_file)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
                .filter(|entry| entry.path().extension() == Some("md".as_ref()))
                .map(|entry| entry.path())
                .collect::<Vec<_>>();
            if files.is_empty() {
                justlogfox::log_warn!("No markdown files found in {:?}", opts.input_file);
            }
            files
        } else {
            justlogfox::log_trace!("input is file {:?}", opts.input_file);
            vec![opts.input_file.clone()]
        };

    let ext = opts.output_ext.unwrap_or("html".to_owned());

    let outputs =
        match &opts.output_file {
            Some(path) if path == &PathBuf::from("stdout") => {
                justlogfox::log_trace!("output to stdout");
                vec![()].repeat(input_files.len())
                .iter().map(
                    |_| Box::new(std::io::stdout()) as Box<dyn Write>)
                .collect()
            }
            Some(path) if path.is_dir() => {
                std::fs::create_dir_all(path).unwrap();
                justlogfox::log_trace!("output to directory {:?}", path);
                input_files.iter().map(|input_file| {
                    let out_filename = input_file.file_name().unwrap();
                    let mut out_path = path.join(out_filename);
                    out_path.set_extension(&ext);
                    open_write(&out_path)
                }).collect()
            }
            Some(path) if input_files.len() == 1 => {
                justlogfox::log_trace!("output to one file {:?}", path);
                vec![open_write(path)]
            }
            Some(path) => {
                justlogfox::log_error!("multiple input files, but output is not a directory: {:?}", path);
                println!("Try specifying a directory instead, or omit --output");
                std::process::exit(1)
            }
            None => {
                justlogfox::log_trace!("output to renamed  in .");
                input_files.iter().map(|input_file| {
                    let out_filename = input_file.file_name().unwrap();
                    let mut out_path = input_file.parent().unwrap().join(out_filename);
                    out_path.set_extension(&ext);
                    open_write(&out_path)
                }).collect()
            }
        };

    let meta_outputs =
        match &opts.metadata_file {
            Some(Some(path)) if path == &PathBuf::from("stdout") => {
                justlogfox::log_trace!("metadata to stdout");
                vec![()].repeat(input_files.len())
                .iter().map(
                    |_| Some(Box::new(std::io::stdout()) as Box<dyn Write>))
                .collect()
            }
            Some(Some(path)) if path.is_dir() => {
                std::fs::create_dir_all(path).unwrap();
                justlogfox::log_trace!("metadata to directory {:?}", path);
                input_files.iter().map(|input_file| {
                    let out_filename = input_file.file_name().unwrap();
                    let mut out_path = path.join(out_filename);
                    out_path.set_extension("json");
                    Some(open_write(&out_path))
                }).collect()
            }
            Some(Some(path)) if input_files.len() == 1 => {
                justlogfox::log_trace!("metadata to one file {:?}", path);
                vec![Some(open_write(path))]
            }
            Some(Some(path)) => {
                justlogfox::log_error!("multiple input files, but meta is not a directory: {:?}", path);
                println!("Try specifying a directory instead, or omit the argument to --meta");
                std::process::exit(1)
            }
            Some(None) => {
                justlogfox::log_trace!("metadata to .");
                input_files.iter().map(|input_file| {
                    let out_filename = input_file.file_name().unwrap();
                    let mut out_path = input_file.parent().unwrap().join(out_filename);
                    out_path.set_extension("json");
                    Some(open_write(&out_path))
                }).collect()
            }
            None => {
                justlogfox::log_trace!("no metadata output");
                input_files.iter().map(|_| None).collect()
            }
        };
    
    let load_start = std::time::Instant::now();

    let options = 
        MdnyaOptions::new(opts.close_all_tags, opts.section_tags, opts.document_tags, opts.heading_level, !opts.no_ids)
        .with_starry_night();

    justlogfox::log_debug!("setup took {:?}", (load_start.elapsed()));

    for ((input, mut output), meta) in input_files.iter().zip(outputs).zip(meta_outputs) {
        justlogfox::log_debug!("rendering {:?}", input);
        let render_start = std::time::Instant::now();

        convert_one(input, &mut output, meta, &options)?;

        justlogfox::log_debug!("mdnya render() took {:?}", (render_start.elapsed()));
    }

    Ok(())
}
