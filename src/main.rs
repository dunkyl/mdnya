use std::{path::PathBuf, io::Write, error::Error};

use tree_sitter::{Parser, TreeCursor};
use clap::Parser as clapParser;

use regex::Regex;
use lazy_static::lazy_static;

mod generated_lang;

fn to_title_case(s: impl AsRef<str>) -> String {
    let mut c = s.as_ref().chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect()
    }
}

const HL_NAMES: &[&str] = &[
    "attribute",
    "constant",
    "function.builtin",
    "function",
    "keyword",
    "operator",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "string",
    "string.special",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
];

fn highlight_code(source: &[u8], lang_name: &str, hl_classes: &[&str]) -> Result<Option<String>, Box<dyn Error>>{
    let mut tshl = tree_sitter_highlight::Highlighter::new();
    let lang = generated_lang::language_by_name(lang_name);
    let highlightscm = generated_lang::highlight_query_by_name(lang_name);
    if let (Some(lang), Some(query)) = (lang, highlightscm) {
        let mut config = tree_sitter_highlight::HighlightConfiguration::new(
            lang,
            query,
            "",
            ""
        )?;
        config.configure(HL_NAMES);
        let hl = tshl.highlight(&config, source, None, |_| None)?;
        let mut renderer = tree_sitter_highlight::HtmlRenderer::new();
        renderer.render(hl, source, &|hl| hl_classes[hl.0].as_bytes())?;
        Ok(Some(String::from_utf8(renderer.html)?))
    } else {
        Ok(None)
    }
}

fn to_html<'a, 'b, 'c>(src: &[u8], cursor: &mut TreeCursor<'b>, depth: usize, out: &mut Box<dyn Write>, opts: &Options, hl_classes: &[&str]) -> Result<(), Box<dyn Error>> {
    let node = cursor.node();
    let indent = " ".repeat(4*depth);

    let recurse_siblings = |cursor: &mut TreeCursor<'b>, out: &mut Box<dyn Write>, depth| {
        loop {
            to_html(src, cursor, depth, &mut *out, opts, hl_classes)?;
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        Ok::<(), Box<dyn Error>>(())
    };

    let recurse = |cursor: &mut TreeCursor<'b>, out: &mut Box<dyn Write>, inc_depth| {
        let new_depth = depth + inc_depth;
        if cursor.goto_first_child() {
            recurse_siblings(cursor, out, new_depth)?;
            cursor.goto_parent();
        }
        Ok::<(), Box<dyn Error>>(())
    };

    lazy_static! {
        static ref RE_ADMONITION: Regex = Regex::new(r"\{(?P<class>\w+)\}( (?P<title>\w[\w\s]*))?").unwrap();
    }

    match node.kind() {
        "document" => {
            if let Some(tags @ [_, ..]) = opts.wrap_tags.as_deref(){
                for (i, tag) in tags.iter().enumerate() {
                    let indent = " ".repeat(4*i);
                    writeln!(out, "{indent}<{tag}>")?;
                }
                recurse(cursor, out, tags.len())?;
                for (i, tag) in tags.iter().enumerate().rev() {
                    let indent = " ".repeat(4*i);
                    writeln!(out, "{indent}</{tag}>")?;
                }
            } else {
                recurse(cursor, out, 0)?;
            }
        },
        "heading_content" => {
            recurse(cursor, out, 0)?;
        },
        "atx_heading" => {
            assert!(cursor.goto_first_child(), "headings' first child is a atx heading marker");
            let tag = &cursor.node().kind()[4..6];
            if node.parent().unwrap().kind() != "document" {
                writeln!(out)?;
            }
            write!(out, "{indent}<{tag}>")?;
            assert!(cursor.goto_next_sibling(), "headings' second child is content");
            to_html(src, cursor, depth, out, opts, hl_classes)?;
            writeln!(out, "</{tag}>")?;
            cursor.goto_parent();
        },
        "text" => {
            write!(out, "{}", node.utf8_text(src).expect("the input source is valid utf8").trim_end())?;
        },
        "paragraph" => {
            // peek to elide if only surrounding one image:
            assert!(cursor.goto_first_child(), "paragraphs are not empty");
            let is_image = cursor.node().kind() == "image";
            let only_child = !cursor.goto_next_sibling();
            cursor.goto_parent();

            if !is_image || !only_child {
                write!(out, "{indent}<p>")?;
                recurse(cursor, out,  0)?;
                write!(out, "{}", if opts.close_all_tags { "</p>\n" } else { "\n" })?;
            }
            else {
                recurse(cursor, out,  0)?;
            }
            
        },
        "link" => {
            cursor.goto_first_child();
            cursor.goto_next_sibling();
            let link_url = cursor.node().utf8_text(src).expect("the input source is valid utf8");
            write!(out, " <a href=\"{link_url}\">")?;
            cursor.goto_parent();
            cursor.goto_first_child();
            recurse(cursor, out,  0)?;
            write!(out, "</a>")?;
            cursor.goto_parent();
        },
        "image" => {
            cursor.goto_first_child();
            let description = cursor.node().utf8_text(src).expect("the input source is valid utf8");
            cursor.goto_next_sibling();
            let image_url = cursor.node().utf8_text(src).expect("the input source is valid utf8");
            writeln!(out, "{indent}<img src=\"{image_url}\" alt=\"{description}\" />")?;
            cursor.goto_parent();
        },
        "thematic_break" => {
            write!(out, "{indent}<hr />\n\n")?;
        },
        "tight_list" | "loose_list" => {
            // peek to find out kind of list
            cursor.goto_first_child();
            
            let mut markers = std::vec::Vec::new();
            loop {
                cursor.goto_first_child();
                markers.push(cursor.node().utf8_text(src).unwrap());
                cursor.goto_parent();
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            let is_bulleted = markers.iter().all(|&m| m == "-" || m == "*");
            let is_numbered_forward = markers.iter().enumerate().all(|(i, &m)| m == (i+1).to_string() + ".");
            let is_numbered_backward  = markers.iter().enumerate().all(|(i, &m)| m == (markers.len() - i).to_string() + ".");

            let attributes: String = if !is_bulleted {
                if is_numbered_backward  {
                    " reversed".into()
                } else if is_numbered_forward {
                    "".into()
                } else {
                    return Err(
                        format!("unknown list format in list:\n{}", node.utf8_text(src).unwrap()).into()
                    );
                }
            } else {
                "".into()
            };

            cursor.goto_parent();

            let tag = if is_bulleted {"ul"} else {"ol"};

            writeln!(out, "{indent}<{tag}{attributes}>")?;
            recurse(cursor, out,  1)?;
            writeln!(out, "{indent}</{tag}>")?;
        },
        "list_item" | "task_list_item" => {
            write!(out, "{indent}<li>")?;
            assert!(cursor.goto_first_child(), "list_items' first child is inline and not empty");
            assert!(cursor.goto_next_sibling(), "list_items' second child is a its content");
            if cursor.node().kind() == "paragraph" {
                cursor.goto_first_child();
                to_html(src, cursor, depth, out, opts, hl_classes)?;
                cursor.goto_parent();
            } else {
                to_html(src, cursor, depth, out, opts, hl_classes)?;
            }
            write!(out, "{}", if opts.close_all_tags { "</li>\n" } else { "\n" })?;
            cursor.goto_parent();
        },
        "task_list_item_marker" => {
            let is_checked = node.utf8_text(src).unwrap() == "[x]";
            let checked_attr = if is_checked { " checked" } else { "" } ; 
            write!(out, "<input type=\"checkbox\" disabled{checked_attr} />")?;
            assert!(cursor.goto_next_sibling(), "task_list_item_markers are not the end of a task_list_item");
            recurse_siblings(cursor, out, depth)?;
        },
        "fenced_code_block" => {
            // peek for info string
            assert!(cursor.goto_first_child(), "fenced_code_blocks' is not empty");
            let first_child = cursor.node();
            let info_string =
                if first_child.kind() == "info_string" {
                    cursor.goto_next_sibling();
                    Some(first_child.utf8_text(src).expect("the input source is valid utf8"))
                } else {
                    None
                }.unwrap_or("");
            if let Some(caps) = RE_ADMONITION.captures(info_string) {
                let admonition_class = caps.get(1).unwrap().as_str();
                let default_title = to_title_case(admonition_class);
                let admonition_title = caps.get(2).map(|m| m.as_str()).unwrap_or(default_title.as_str());
                write!(out, "{indent}<div class=\"admonition {admonition_class}\">" )?;
                write!(out, "{indent}{indent}<div class=\"admonition-title\">{admonition_title}</div>" )?;
                recurse(cursor, out,  2)?;
                cursor.goto_parent();
                write!(out, "{indent}</div>" )?;
            } else {
                let attributes = if info_string.is_empty() {
                    "".into()
                } else {
                    format!(" data-lang=\"{}\"", info_string)
                };
                writeln!(out, "{indent}<pre><code{attributes}>")?;
                if let Some(highlighted_code) = highlight_code(&src[cursor.node().start_byte()..cursor.node().end_byte()], info_string, hl_classes).unwrap() {
                    write!(out, "{}", highlighted_code)?;
                } else {
                    println!("no code highlighter found for language: {:?}", info_string);
                    recurse(cursor, out,  1)?;
                }
                cursor.goto_parent(); // step back out of the code_block_content
                write!(out, "\n{indent}</code></pre>\n")?;
            }
        },
        "block_quote" => {
            writeln!(out, "{indent}<blockquote>")?;
            recurse(cursor, out,  1)?;
            writeln!(out, "{indent}</blockquote>")?;
        },
        "soft_line_break" => {
            write!(out, "\n{indent}<br />\n{indent}")?;
        },
        "line_break" => {
            writeln!(out)?;
        },
        "code_span" => {
            write!(out, " <code>")?;
            recurse(cursor, out,  0)?;
            write!(out, "</code>")?;
        },
        "emphasis" => {
            write!(out, " <em>")?;
            recurse(cursor, out,  0)?;
            write!(out, "</em>")?;
        },
        "strong_emphasis" => {
            write!(out, " <strong>")?;
            recurse(cursor, out,  0)?;
            write!(out, "</strong>")?;
        },
        "strikethrough" => {
            write!(out, " <del>")?;
            recurse(cursor, out,  0)?;
            write!(out, "</del>")?;
        },
        unhandled => {
            return Err(
                format!("unhandled node kind '{}':\n{}", unhandled, node.utf8_text(src).unwrap()).into()
            );
        }
    };
    Ok(())
}

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
    #[clap(short, long, name="wrap-tags", value_parser,  value_delimiter = ',')]
    wrap_tags: Option<Vec<String>>,

    /// Show times
    #[clap(short, long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
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
            if let Some(stem) = stem_opt {
                let stem = stem.to_str().unwrap().to_owned();
                Box::new(std::fs::File::create(stem + ".html")?) as Box<dyn Write>
            }
            else {
                return Err("default output file (replace .md with .html) expects a filename with a stem".into())
            }
        };

    parser.set_language(generated_lang::language_markdown()).unwrap();

    let time_parse_start = std::time::Instant::now();
    let tree = parser.parse(source_code.as_slice(), None).unwrap();
    let root_node = tree.root_node();
    let parse_elapsed = time_parse_start.elapsed();
    if opts.verbose {
        println!("parse time: {:?}", parse_elapsed);
    }

    let hl_classes = HL_NAMES.iter().map(|s| format!("mdnya-hl-{}", s.replace('.', "-"))).collect::<Vec<_>>();
    let hl_classes_ref = hl_classes.iter().map(|s| s.as_str()).collect::<Vec<_>>();

    let time_write_start = std::time::Instant::now();
    to_html(source_code.as_slice(), &mut root_node.walk(), 0, &mut output_writer, &opts, &hl_classes_ref)?;
    let write_elapsed = time_write_start.elapsed();
    if opts.verbose {
        println!("write time: {:?}", write_elapsed);
    }
    Ok(())
}
