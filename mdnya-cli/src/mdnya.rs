use std::{collections::HashMap, borrow::Cow, io::stdout};

use tree_sitter::{TreeCursor, Node};

use regex::Regex;
use lazy_static::lazy_static;

use crate::highlight;

const INDENT_STR: &str = "    ";

pub struct HtmlHelper {
    pub is_inline: bool,
    pub indent_level: usize,
    pub close_tags: bool,
    pub extra_heading_level: u8,
    pub wrap_sections: Option<String>,
    pub last_heading_level: u8,
    pub last_elem_was_header: bool,
    pub omit_header_id: bool,
    pub inside_section: bool,
}

#[derive(Clone, PartialEq)]
enum TagBehavior {
    Full,
    OptionalClose,
    SelfClose,
    NoTags
}
use TagBehavior::*;

#[derive(Clone)]
struct NodeBehavior {
    inline: bool,
    wrap: TagBehavior,
    skip_children: usize,
    tag_attr_content: fn(&Node, &[u8], &HtmlHelper) -> (Vec<String>, Vec<(&'static str, Option<String>)>, Option<Vec<u8>>),
}

impl NodeBehavior {
    fn new(inline: bool, wrap: TagBehavior, skip_children: usize, tag_attr_content: fn(&Node, &[u8], &HtmlHelper) -> (Vec<String>, Vec<(&'static str, Option<String>)>, Option<Vec<u8>>)) -> Self {
        Self { inline, wrap, skip_children, tag_attr_content: tag_attr_content }
    }
}

impl std::default::Default for NodeBehavior {
    fn default() -> Self {
        NodeBehavior {
            inline: false,
            wrap: Full,
            skip_children: 0,
            tag_attr_content: |n, _, _| (vec![n.kind().to_string()], vec![], None)
        }
    }
}

lazy_static! {
    static ref LANGUAGE_ALIASES: HashMap<&'static str, &'static str> = {
        [
            ("c++", "cpp"),
            ("c#", "c_sharp"),
            ("f#", "fsharp"),
            // ("html", "xml"),
            ("js", "javascript"),
            ("py", "python"),
            ("rb", "ruby"),
            ("sh", "bash"),
            ("ts", "typescript"),
        ].iter().cloned().collect()
    };

    static ref RE_ADMONITION: Regex = Regex::new(r"\{(?P<class>\w+)\}( (?P<title>\w[\w\s]*))?").unwrap();

}

fn decide_list_type(node: &Node, src: &[u8], _: &HtmlHelper) -> (Vec<String>, Vec<(&'static str, Option<String>)>, Option<Vec<u8>>) {
    let markers = (0..node.child_count()).map(|i| node.child(i).unwrap().child(0).unwrap().utf8_text(src).unwrap()).collect::<Vec<_>>();
    let is_bulleted = markers.iter().all(|&m| m == "-" || m == "*");
    let is_numbered_forward = markers.iter().enumerate().all(|(i, &m)| m == &((i+1).to_string() + "."));
    let is_numbered_backward  = markers.iter().enumerate().all(|(i, &m)| m == &((markers.len() - i).to_string() + "."));

    if is_bulleted {
        (vec!["ul".into()], vec![], None)
    } else if is_numbered_forward {
        (vec!["ol".into()], vec![], None)
    } else if is_numbered_backward {
        (vec!["ol".into()], vec![("reversed", None)], None)
    } else {
        todo!("unknown list type {:?}", markers)
    }
}

fn render_code_block(node: &Node, src: &[u8], helper: &HtmlHelper) -> (Vec<String>, Vec<(&'static str, Option<String>)>, Option<Vec<u8>>) {
    let first_child = node.child(0).expect("fenced_code_block are never empty");
    if first_child.kind() == "info_string" {
        let info = first_child.utf8_text(src).unwrap();
        // println!("info: {}", info);
        if  let Some(caps) = RE_ADMONITION.captures(info) {
            // println!("is admonition");
            let indent = INDENT_STR.repeat(helper.indent_level+1);
            let close_p = if helper.close_tags { "</p>" } else { "" };
            let title_elem = format!("{indent}<h3>{}</h3>\n{indent}<p>", 
                match caps.name("title") {
                    Some(titlematch) => to_title_case(titlematch.as_str()),
                    None => {
                        to_title_case(caps.name("class").unwrap().as_str())
                    }
                }
            );
            let mut inner_content: Vec<u8> = 
                title_elem.as_bytes().iter()
                .chain(node_text_safe(&node.child(1).unwrap(), src).as_bytes().iter())
                .chain(close_p.as_bytes().iter())
                .cloned().collect();
            inner_content.push(b'\n');
            (vec!["div".into()], vec![
                ("class", Some(format!("admonition {}", caps.name("class").unwrap().as_str())))
            ], Some(inner_content))
        } else {
            let code_node = node.child(1).unwrap();
            let code_slice = &src[code_node.start_byte()..code_node.end_byte()];
            let start = std::time::Instant::now();
            let ts_name = LANGUAGE_ALIASES.get(info.to_lowercase().as_str()).unwrap_or(&info);
            let hl_code = highlight::highlight_code(code_slice, ts_name).unwrap();
            let hl_code_lines: Option<Vec<u8>> = match hl_code {
                Some(v) => {
                    let mut str = String::from_utf8(v).unwrap();
                    str.truncate(str.len()-1); // remove trailing newline
                    let with_lines: String = str.replace("\n", "</span>\n<span class=\"code-line\">");
                    Some(format!("<span class=\"code-line\">{with_lines}</span>").into())
                }
                None => None
            };

            let inner_content = hl_code_lines.unwrap_or_else(
                || {
                    std::vec::Vec::from(code_slice)
                }
            );

            println!("highlighting took {:?}", start.elapsed());
            (vec!["pre".into(), "code".into()], vec![
                ("data-lang", Some(info.into()))
            ], Some(inner_content))
        }
    } else {
        (vec!["pre".into(), "code".into()], vec![], None)
    }
}

fn to_title_case(s: impl AsRef<str>) -> String {
    let mut c = s.as_ref().chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect()
    }
}

macro_rules! rename_tag {
    ($tag:expr) => {
        NodeBehavior {
            tag_attr_content: |_, _, _| (vec![$tag.into()], vec![], None),
            ..Default::default()
        }
    }
}

macro_rules! tag_name {
    ($tag:expr) => {
        |_, _, _| (vec![$tag.into()], vec![], None)
    }
}

fn no_tag() -> NodeBehavior {
    NodeBehavior {
        wrap: NoTags,
        ..Default::default()
    }
}

fn node_text_safe<'a>(node: &Node, src: &'a [u8]) -> Cow<'a, str> {
    html_escape::encode_safe(node.utf8_text(src).unwrap())
}

fn node_text_raw<'a>(node: &Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap()
}


// fn node_text_attr<'a>(node: &Node, src: &'a [u8]) -> Cow<'a, str> {
//     html_escape::encode_double_quoted_attribute(node.utf8_text(src).unwrap())
// }

impl HtmlHelper {

    fn write_indent(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        for _ in 0..self.indent_level {
            write!(out, "{}", INDENT_STR)?;
        }
        Ok(())
    }

    fn write_tag(&self, out: &mut impl std::io::Write, before: &str, tag: &str, attrs: &[(&str, Option<String>)], after: &str, n: bool) -> std::io::Result<()> {
        if !self.is_inline {
            self.write_indent(out)?;
        }
        write!(out, "{}{tag}", before)?;
        for (k, v) in attrs {
            let k = html_escape::encode_text_minimal(k);
            if let Some(v) = v {
                write!(out, " {k}=\"{}\"", html_escape::encode_quoted_attribute(v))?;
            } else {
                write!(out, " {k}")?;
            }
        }
        write!(out, "{}", after)?;
        if !self.is_inline && n {
            write!(out, "\n")?;
        }
        Ok(())
    }

    pub fn start_tag(&self, out: &mut impl std::io::Write, tag: & impl AsRef<str>, attrs: &[(&str, Option<String>)], n: bool) -> std::io::Result<()> {
        self.write_tag(out, "<", tag.as_ref(), attrs, ">", n)
    }

    fn self_close_tag(&self, out: &mut impl std::io::Write, tag: & impl AsRef<str>, attrs: &[(&str, Option<String>)]) -> std::io::Result<()> {
        self.write_tag(out, "<", tag.as_ref(), attrs, " />", true)
    }

    pub fn end_tag(&self, out: &mut impl std::io::Write, tag: & impl AsRef<str>) -> std::io::Result<()> {
        self.write_tag(out, "</", tag.as_ref(), &[], ">", true)
    }

    pub fn start_section(&mut self, out: &mut impl std::io::Write, tag: & impl AsRef<str>) -> std::io::Result<()> {
        println!("!start section");
        self.start_tag(out, tag, &[], true)?;
        self.indent_level += 1;
        self.inside_section = true;
        Ok(())
    }

    pub fn end_section(&mut self, out: &mut impl std::io::Write, tag: & impl AsRef<str>) -> std::io::Result<()> {
        println!("!end section");
        self.indent_level = self.indent_level.saturating_sub(1);
        self.end_tag(out, tag)?;
        self.inside_section = false;
        Ok(())
    }

}

fn find_header_level(node: &Node) -> u8 {
    u8::from_str_radix(&node.child(0).unwrap().kind()[5..6], 10).unwrap()
}

pub fn render_into(src: &[u8], cursor: &mut TreeCursor, putter: &mut HtmlHelper, out: &mut impl std::io::Write) -> std::io::Result<()>
{
    
    lazy_static!{

       
        static ref NODES_BEHAVE: HashMap<&'static str, NodeBehavior> = [
            ("paragraph",       NodeBehavior::new(true, OptionalClose, 0, tag_name!("p"))),
            ("list_item",       NodeBehavior::new(true, OptionalClose, 1, tag_name!("li"))),
            ("task_list_item",  NodeBehavior::new(true, OptionalClose, 1, tag_name!("li"))),
            
            ("atx_heading",     NodeBehavior::new(true, Full, 1,
                |node, src, helper| { // find hX tag
                    let level = find_header_level(node);
                    let h_str = format!("h{}", level+helper.extra_heading_level-1);
                    let heading_content = node_text_raw(&node.child(1).unwrap(), src).trim_start();
                    let attrs =
                        if helper.omit_header_id {
                            vec![]
                        } else {
                            let id = 
                                if heading_content.starts_with('@')  {
                                    heading_content.to_string()
                                } else {
                                    heading_content.to_lowercase().replace(" ", "-")
                                };
                            vec![("id", Some(id))]
                        };
                    (vec![h_str], attrs, Some(html_escape::encode_text(heading_content).as_bytes().into()))
                })),

            // ("heading_content",     no_tag()),
            ("text",                no_tag()),
            // ("link_text",           no_tag()),
            ("line_break",          no_tag()),
            ("code_fence_content",  no_tag()),

            // ("link_destination", NodeBehavior { wrap: NoTags, skip_children: None, ..Default::default()}),

            ("emphasis",        rename_tag!("em")),
            ("strong_emphasis", rename_tag!("strong")),
            ("strikethrough",   rename_tag!("del")),
            ("code_span",       rename_tag!("code")),
            ("block_quote",     rename_tag!("blockquote")),
            ("uri_autolink",    rename_tag!("what")),

            ("thematic_break",  NodeBehavior::new(false, SelfClose, 0, tag_name!("hr"))),
            ("image", NodeBehavior::new(false, SelfClose, 0,
                |node, src, _| (vec!["img".into()],
                    vec![
                        ("src", Some(node_text_raw(&node.child(1).unwrap(), src).into())),
                        ("alt", Some(node_text_raw(&node.child(0).unwrap(), src).into()))
                    ], None)
                )),
            ("task_list_item_marker", NodeBehavior::new(false, SelfClose, 0,
                |node, src, _| {
                    let is_checked = node.utf8_text(src).unwrap() == "[x]";
                    let mut attrs = vec![
                        ("type", Some("checkbox".into())),
                        ("disabled", None)
                    ];
                    if is_checked { attrs.push(("checked", None)); }
                    (vec!["input".into()], attrs, None)
                })),
            ("link", NodeBehavior::new(false, Full, 0,
                |node, src, _| (vec!["a".into()],
                    vec![ ("href", Some(node.child(1).unwrap().utf8_text(src).unwrap().to_string())) ],
                    Some( // link text
                        node.child(0).unwrap().utf8_text(src).unwrap().to_string().into()
                    ))
                )),
            ("tight_list", NodeBehavior::new(false, Full, 0, decide_list_type)),
            ("loose_list", NodeBehavior::new(false, Full, 0, decide_list_type)),
            ("fenced_code_block", NodeBehavior::new(false, Full, 0, render_code_block)),
            ("indented_code_block", NodeBehavior::new(false, Full, 0, render_code_block)),
        ].iter().cloned().collect();

        // static ref DEFAULT_BEHAVE: NodeBehavior = NodeBehavior::default();
    }

    // let mut inside_section = false;

    loop {
        let node = cursor.node();
        let kind = node.kind();
        let behave = NODES_BEHAVE.get(kind).unwrap_or_else(|| panic!("{}", kind));

        let is_heading = kind == "atx_heading";

        if let Some(n) = node.prev_sibling() {
            if n.kind() == "atx_heading" && !is_heading && !putter.inside_section {
                if let Some(tag) = putter.wrap_sections.clone() {
                    // println!("!start section");
                    // putter.start_tag(out, tag, &[])?;
                    // putter.indent_level += 1;
                    // putter.inside_section = true;
                    putter.start_section(out, &tag)?;
                }
            }
        }

        
        putter.write_indent(&mut stdout())?;
        if behave.inline || putter.is_inline {
            println!("@{kind}~");
        } else {
            putter.write_indent(&mut stdout())?;
            println!("@{kind}");
        }

        // let behave = NODES_BEHAVE.get(kind).unwrap_or_else(|| panic!("{}", kind)); // _or(&DEFAULT_BEHAVE);

        let (tag, attrs, replace_content) = (behave.tag_attr_content)(&node, src, putter);

        let is_inline = putter.is_inline || behave.inline;

        let switched_inline = is_inline && !putter.is_inline;

        let k = &mut HtmlHelper { is_inline, wrap_sections: putter.wrap_sections.clone(), inside_section: false, ..*putter};
        if switched_inline {
            putter.write_indent(out)?;
        }
        match behave.wrap {
            SelfClose => {
                // TODO: type enforce only one tag for self close 
                k.self_close_tag(out, &tag[0], &attrs)?;
            }
            NoTags => (),
            _ => {
                // if not <p> before a <img> (omitted)
                if node.child_count() != 1 || !(kind == "paragraph" && node.child(0).unwrap().kind() == "image") {
                    // if kind == "atx_heading" {
                    //     writeln!(out, "")?;
                    // }
                    let mut tag_iter = tag.iter();
                    let first = tag_iter.next().unwrap();
                    k.start_tag(out, first, &attrs, first != "pre")?;
                    
                    for tag in tag_iter {
                        write!(out, "<{}>", tag)?;
                    }
                    // if tag[0] == "pre" {
                    //     writeln!(out, "")?;
                    // }
                }
            }
        }
        if behave.wrap != SelfClose {
            if let Some(content) = replace_content {
                out.write_all(&content)?;
            } else {
                if cursor.goto_first_child() {
                    for _ in 0..behave.skip_children {
                        cursor.goto_next_sibling();
                    }
                    
                    render_into(src, cursor, &mut HtmlHelper {
                        indent_level: putter.indent_level + 1,
                        wrap_sections: putter.wrap_sections.clone(),
                        last_heading_level: 0,
                        ..*k},
                        
                        out)?;
                    cursor.goto_parent();
                } else {
                    let /*mut*/ text = node.utf8_text(src).unwrap().trim_start();
                    // if node.parent().and_then(|n| n.parent()).map(|n| n.kind()) == Some("atx_heading") {
                    //     text = text.trim();
                    // } 
                    html_escape::encode_text_to_writer(text, out)?;
                }
            }
        }
        match (&behave.wrap, putter.close_tags)  {
            (Full, _) | 
            (OptionalClose, true) => {
                let omit_innner_newline = tag[0] == "pre";
                
                for tag in tag.iter().rev() {
                    if omit_innner_newline {
                        write!(out, "</{}>", tag)?;
                    } else {
                        k.end_tag(out, &tag)?;
                    }
                }
                if omit_innner_newline {
                    writeln!(out, "")?;
                }
            }
            _ => ()
        }
        if switched_inline {
            writeln!(out, "")?;
        }

        let has_next = cursor.goto_next_sibling();

        if (putter.inside_section && !has_next) || (has_next && cursor.node().kind() == "atx_heading") {
            if let Some(tag) = putter.wrap_sections.clone() {
                if !is_heading {
                    putter.end_section(out, &tag)?;
                    // println!("!end section");
                    // putter.indent_level = putter.indent_level.saturating_sub(1);
                    // putter.end_tag(out, tag)?;
                    // putter.inside_section = false;
                }
            }
        }

        if has_next && cursor.node().kind() == "atx_heading" {
            writeln!(out, "")?;
        }

        if !has_next {
            break;
        }
    }

    Ok(())
}