use std::{collections::HashMap, borrow::Cow};

use tree_sitter::{TreeCursor, Node};

use regex::Regex;
use lazy_static::lazy_static;

use crate::highlight;

pub struct HtmlHelper {
    pub is_inline: bool,
    pub indent_level: usize,
    pub close_tags: bool
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
    tag_attr_content: fn(&Node, &[u8]) -> (String, Vec<(&'static str, Option<String>)>, Option<Vec<u8>>),
}

impl NodeBehavior {
    fn new(inline: bool, wrap: TagBehavior, skip_children: usize, tag_attr_content: fn(&Node, &[u8]) -> (String, Vec<(&'static str, Option<String>)>, Option<Vec<u8>>)) -> Self {
        Self { inline, wrap, skip_children, tag_attr_content: tag_attr_content }
    }
}

impl std::default::Default for NodeBehavior {
    fn default() -> Self {
        NodeBehavior {
            inline: false,
            wrap: Full,
            skip_children: 0,
            tag_attr_content: |n, _| (n.kind().to_string(), vec![], None)
        }
    }
}

fn decide_list_type(node: &Node, src: &[u8]) -> (String, Vec<(&'static str, Option<String>)>, Option<Vec<u8>>) {
    let markers = (0..node.child_count()).map(|i| node.child(i).unwrap().child(0).unwrap().utf8_text(src).unwrap()).collect::<Vec<_>>();
    let is_bulleted = markers.iter().all(|&m| m == "-" || m == "*");
    let is_numbered_forward = markers.iter().enumerate().all(|(i, &m)| m == &((i+1).to_string() + "."));
    let is_numbered_backward  = markers.iter().enumerate().all(|(i, &m)| m == &((markers.len() - i).to_string() + "."));

    if is_bulleted {
        ("ul".into(), vec![], None)
    } else if is_numbered_forward {
        ("ol".into(), vec![], None)
    } else if is_numbered_backward {
        ("ol".into(), vec![("reversed", None)], None)
    } else {
        todo!("unknown list type {:?}", markers)
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
            tag_attr_content: |_, _| ($tag.into(), vec![], None),
            ..Default::default()
        }
    }
}

macro_rules! tag_name {
    ($tag:expr) => {
        |_, _| ($tag.into(), vec![], None)
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

impl HtmlHelper {

    fn write_indent(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        for _ in 0..self.indent_level {
            write!(out, "    ")?;
        }
        Ok(())
    }

    fn write_tag(&self, out: &mut impl std::io::Write, before: &str, tag: &str, attrs: &[(&str, Option<String>)], after: &str) -> std::io::Result<()> {
        if !self.is_inline {
            self.write_indent(out)?;
        }
        write!(out, "{}{tag}", before)?;
        for (k, v) in attrs {
            if let Some(v) = v {
                write!(out, " {k}=\"{}\"", html_escape::encode_quoted_attribute(v))?;
            } else {
                write!(out, " {k}")?;
            }
        }
        write!(out, "{}", after)?;
        if !self.is_inline {
            write!(out, "\n")?;
        }
        Ok(())
    }

    pub fn start_tag(&self, out: &mut impl std::io::Write, tag: & impl AsRef<str>, attrs: &[(&str, Option<String>)]) -> std::io::Result<()> {
        self.write_tag(out, "<", tag.as_ref(), attrs, ">")
    }

    fn self_close_tag(&self, out: &mut impl std::io::Write, tag: & impl AsRef<str>, attrs: &[(&str, Option<String>)]) -> std::io::Result<()> {
        self.write_tag(out, "<", tag.as_ref(), attrs, " />")
    }

    pub fn end_tag(&self, out: &mut impl std::io::Write, tag: & impl AsRef<str>) -> std::io::Result<()> {
        self.write_tag(out, "</", tag.as_ref(), &[], ">")
    }
}

pub fn render_into(src: &[u8], cursor: &mut TreeCursor, putter: &mut HtmlHelper, out: &mut impl std::io::Write) -> std::io::Result<()>
{

    lazy_static!{

        static ref RE_ADMONITION: Regex = Regex::new(r"\{(?P<class>\w+)\}( (?P<title>\w[\w\s]*))?").unwrap();

        static ref NODES_BEHAVE: HashMap<&'static str, NodeBehavior> = [
            ("paragraph",       NodeBehavior::new(true, OptionalClose, 0, tag_name!("p"))),
            ("list_item",       NodeBehavior::new(true, OptionalClose, 1, tag_name!("li"))),
            ("task_list_item",  NodeBehavior::new(true, OptionalClose, 1, tag_name!("li"))),
            
            ("atx_heading",     NodeBehavior::new(true, Full, 1,
                |node, _| { // find hX tag
                    (node.child(0).unwrap().kind()[4..6].into(), vec![], None)
                })),

            ("heading_content",     no_tag()),
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

            ("thematic_break",  NodeBehavior::new(false, SelfClose, 0, tag_name!("hr"))),
            ("image", NodeBehavior::new(false, SelfClose, 0,
                |node, src| ("img".into(),
                    vec![
                        ("src", Some(node_text_safe(&node.child(1).unwrap(), src).into())),
                        ("alt", Some(node_text_safe(&node.child(0).unwrap(), src).into()))
                    ], None)
                )),
            ("task_list_item_marker", NodeBehavior::new(false, SelfClose, 0,
                |node, src| {
                    let is_checked = node.utf8_text(src).unwrap() == "[x]";
                    let mut attrs = vec![
                        ("type", Some("checkbox".into())),
                        ("disabled", None)
                    ];
                    if is_checked { attrs.push(("checked", None)); }
                    ("input".into(), attrs, None)
                })),
            ("link", NodeBehavior::new(false, Full, 0,
                |node, src| ("a".into(),
                    vec![ ("href", Some(node.child(1).unwrap().utf8_text(src).unwrap().to_string())) ],
                    Some( // link text
                        node.child(1).unwrap().utf8_text(src).unwrap().to_string().into()
                    ))
                )),
            ("tight_list", NodeBehavior::new(false, Full, 0, decide_list_type)),
            ("loose_list", NodeBehavior::new(false, Full, 0, decide_list_type)),
            ("fenced_code_block", NodeBehavior::new(false, Full, 0, 
                |node, src| {
                    let first_child = node.child(0).expect("fenced_code_block are never empty");
                    if first_child.kind() == "info_string" {
                        let info = first_child.utf8_text(src).unwrap();
                        // println!("info: {}", info);
                        if  let Some(caps) = RE_ADMONITION.captures(info) {
                            // println!("is admonition");
                            let title_elem = caps.name("title").map(
                                |title| {
                                    format!("<div class=\"admonition-title\">{}</div>", to_title_case(title.as_str()))
                                }
                            );
                            let inner_content: Vec<u8> = 
                                title_elem.unwrap_or("".into()).as_bytes().iter()
                                .chain(node_text_safe(&node.child(1).unwrap(), src).as_bytes().iter())
                                .cloned().collect();
                            ("div".into(), vec![
                                ("class", Some(format!("admonition {}", caps.name("class").unwrap().as_str())))
                            ], Some(inner_content))
                        } else {
                            let code_node = node.child(1).unwrap();
                            let code_slice = &src[code_node.start_byte()..code_node.end_byte()];
                            let hl_code = highlight::highlight_code(code_slice, info).unwrap();
                            ("pre".into(), vec![
                                ("data-lang", Some(info.into()))
                            ], hl_code)
                        }
                    } else {
                        ("pre".into(), vec![], None)
                    }
                })),
        ].iter().cloned().collect();

        // static ref DEFAULT_BEHAVE: NodeBehavior = NodeBehavior::default();
    }

    loop {
        let node = cursor.node();
        let kind = node.kind();

        let behave = NODES_BEHAVE.get(kind).unwrap_or_else(|| panic!("{}", kind)); // _or(&DEFAULT_BEHAVE);

        let (tag, attrs, replace_content) = (behave.tag_attr_content)(&node, src);

        let is_inline = putter.is_inline || behave.inline;

        let switched_inline = is_inline && !putter.is_inline;

        let k = &mut HtmlHelper { is_inline, ..*putter};
        if switched_inline {
            putter.write_indent(out)?;
        }
        match behave.wrap {
            SelfClose => {
                k.self_close_tag(out, &tag, &attrs)?;
            }
            NoTags => (),
            _ => {
                // omit <p> before a <img>
                if node.child_count() != 1 || !(kind == "paragraph" && node.child(0).unwrap().kind() == "image") {
                    k.start_tag(out, &tag, &attrs)?;
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
                        indent_level: putter.indent_level + 1, ..*k},
                        out)?;
                    cursor.goto_parent();
                } else {
                    let mut text = node.utf8_text(src).unwrap();
                    if node.parent().and_then(|n| n.parent()).map(|n| n.kind()) == Some("atx_heading") {
                        text = text.trim();
                    } 
                    html_escape::encode_text_to_writer(text, out)?;
                }
            }
        }
        match (&behave.wrap, putter.close_tags)  {
            (Full, _) | 
            (OptionalClose, true) => {
                k.end_tag(out, &tag)?;
            }
            _ => ()
        }
        if switched_inline {
            writeln!(out, "")?;
        }
        
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    Ok(())
}