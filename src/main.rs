use std::{path::PathBuf, io::Write, error::Error};

use tree_sitter::{Parser, Language, TreeCursor};
use clap::Parser as clapParser;

extern "C" { fn tree_sitter_markdown() -> Language; }

macro_rules! write_format {
    ($outstream:ident, $text:expr) => {
        $outstream.write(format!($text).as_bytes())?;
    };
}

fn to_html<'a, 'b, 'c>(src: &[u8], cursor: &mut TreeCursor<'b>, depth: usize, out: &mut Box<dyn Write>, opts: &Options) -> Result<(), Box<dyn Error>> {
    let node = cursor.node();
    let indent = " ".repeat(4*depth);

    let recurse_siblings = |cursor: &mut TreeCursor<'b>, out: &mut Box<dyn Write>, depth| {
        loop {
            to_html(src, cursor, depth, &mut *out, opts)?;
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

    match node.kind() {
        "document" => {
            if let Some(tags @ [_, ..]) = opts.wrap_tags.as_deref(){
                for (i, tag) in tags.iter().enumerate() {
                    let indent = " ".repeat(4*i);
                    write_format!(out, "{indent}<{tag}>\n");
                }
                recurse(cursor, out, tags.len())?;
                for (i, tag) in tags.iter().enumerate().rev() {
                    let indent = " ".repeat(4*i);
                    write_format!(out, "{indent}</{tag}>\n");
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
                out.write(b"\n")?;
            }
            write_format!(out, "{indent}<{tag}>");
            assert!(cursor.goto_next_sibling(), "headings' second child is content");
            to_html(src, cursor, depth, out, opts)?;
            write_format!(out, "</{tag}>\n");
            cursor.goto_parent();
        },
        "text" => {
            out.write(node.utf8_text(src).expect("the input source is valid utf8").trim_end().as_bytes())?;
        },
        "paragraph" => {
            // peek to elide if only surrounding one image:
            assert!(cursor.goto_first_child(), "paragraphs are not empty");
            let is_image = cursor.node().kind() == "image";
            let only_child = !cursor.goto_next_sibling();
            cursor.goto_parent();

            if !is_image || !only_child {
                write_format!(out, "{indent}<p>");
                recurse(cursor, out,  0)?;
                out.write(if opts.close_all_tags { b"</p>\n" } else { b"\n" })?;
            }
            else {
                recurse(cursor, out,  0)?;
            }
            
        },
        "link" => {
            cursor.goto_first_child();
            cursor.goto_next_sibling();
            let link_url = cursor.node().utf8_text(src).expect("the input source is valid utf8");
            write_format!(out, " <a href=\"{link_url}\">");
            cursor.goto_parent();
            cursor.goto_first_child();
            recurse(cursor, out,  0)?;
            out.write(b"</a>")?;
            cursor.goto_parent();
        },
        "image" => {
            cursor.goto_first_child();
            let description = cursor.node().utf8_text(src).expect("the input source is valid utf8");
            cursor.goto_next_sibling();
            let image_url = cursor.node().utf8_text(src).expect("the input source is valid utf8");
            write_format!(out, "{indent}<img src=\"{image_url}\" alt=\"{description}\" />\n");
            cursor.goto_parent();
        },
        "thematic_break" => {
            write_format!(out, "{indent}<hr />\n\n");
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
                        format!("unknown list format in list:\n{}", node.utf8_text(src).unwrap().to_string()).into()
                    );
                }
            } else {
                "".into()
            };

            cursor.goto_parent();

            let tag = if is_bulleted {"ul"} else {"ol"};

            write_format!(out, "{indent}<{tag}{attributes}>\n");
            recurse(cursor, out,  1)?;
            write_format!(out, "{indent}</{tag}>\n");
        },
        "list_item" | "task_list_item" => {
            write_format!(out, "{indent}<li>");
            assert!(cursor.goto_first_child(), "list_items' first child is inline and not empty");
            assert!(cursor.goto_next_sibling(), "list_items' second child is a its content");
            if cursor.node().kind() == "paragraph" {
                cursor.goto_first_child();
                to_html(src, cursor, depth, out, opts)?;
                cursor.goto_parent();
            } else {
                to_html(src, cursor, depth, out, opts)?;
            }
            out.write(if opts.close_all_tags { b"</li>\n" } else { b"\n" })?;
            cursor.goto_parent();
        },
        "task_list_item_marker" => {
            let is_checked = node.utf8_text(src).unwrap() == "[x]";
            let checked_attr = if is_checked { " checked" } else { "" } ; 
            write_format!(out, "<input type=\"checkbox\" disabled{checked_attr} />");
            assert!(cursor.goto_next_sibling(), "task_list_item_markers are not the end of a task_list_item");
            recurse_siblings(cursor, out, depth)?;
        },
        "list_marker" => {
            out.write(b" <span class=\"marker\">")?;
            recurse(cursor, out,  0)?;
            out.write(b"</span>")?;
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
                };
            let attributes = if let Some(info_string) = info_string {
                format!(" data-lang=\"{}\"", info_string)
            } else {
                "".into()
            };
            write_format!(out, "{indent}<pre><code{attributes}>\n");
            recurse(cursor, out,  1)?;
            cursor.goto_parent(); // step back out of the code_block_content
            write_format!(out, "\n{indent}</code></pre>\n");
        },
        "block_quote" => {
            write_format!(out, "{indent}<blockquote>\n");
            recurse(cursor, out,  1)?;
            write_format!(out, "{indent}</blockquote>\n");
        },
        "soft_line_break" => {
            write_format!(out, "\n{indent}<br />\n{indent}");
        },
        "line_break" => {
            out.write(b"\n")?;
        },
        "code_span" => {
            out.write(b" <code>")?;
            recurse(cursor, out,  0)?;
            out.write(b"</code>")?;
        },
        "emphasis" => {
            out.write(b" <em>")?;
            recurse(cursor, out,  0)?;
            out.write(b"</em>")?;
        },
        "strong_emphasis" => {
            out.write(b" <strong>")?;
            recurse(cursor, out,  0)?;
            out.write(b"</strong>")?;
        },
        "strikethrough" => {
            out.write(b" <del>")?;
            recurse(cursor, out,  0)?;
            out.write(b"</del>")?;
        },
        unhandled => {
            return Err(
                format!("unhandled node kind '{}':\n{}", unhandled, node.utf8_text(src).unwrap().to_string()).into()
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

    let md_treesit = unsafe { tree_sitter_markdown() };
    parser.set_language(md_treesit).unwrap();
    let tree = parser.parse(source_code.as_slice(), None).unwrap();
    let root_node = tree.root_node();

    to_html(source_code.as_slice(), &mut root_node.walk(), 0, &mut output_writer, &opts)?;

    Ok(())
}
