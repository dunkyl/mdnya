use std::collections::HashMap;

use phf::phf_map;
use regex::Regex;
use lazy_static::lazy_static;
use tree_sitter::TreeCursor;

use mdnya_hl::{TSHLang, CodeHighlighter};

mod html;

extern "C" { fn tree_sitter_markdown() -> tree_sitter::Language; }

type MdResult = core::result::Result<(), Box<dyn std::error::Error>>;

lazy_static! {
    static ref RE_ADMONITION: Regex = Regex::new(r"\{(?P<class>\w+)\}\w*((?P<title>\w[\w\s]*))?").unwrap();
}

fn to_title_case(s: impl AsRef<str>) -> String {
    let mut c = s.as_ref().chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect()
    }
}
pub struct MDNya {
    highlighters: HashMap<String, TSHLang>,
    highlighters_aliases: HashMap<String, String>,
    close_all_tags: bool,
    wrap_sections: Option<String>,
    heading_level: u8,
    no_ids: bool,
}

struct MDNyaState {
    inside_section: bool,
}

#[derive(Clone, PartialEq)]
enum TagBehavior {
    Full(&'static str),
    OptionalClose(&'static str),
    SelfClose(&'static str),
    NoTags,
}
use TagBehavior::*;

#[derive(Clone)]
enum NodeTransform {
    Simple {
        tag: TagBehavior,
        inline: bool,
        attrs: &'static [(&'static str, Option<&'static str>)],
    },
    Custom(fn(&MDNya, &mut TreeCursor, &[u8], &mut html::HTMLWriter, &mut MDNyaState) -> MdResult),
    Skip
}
use NodeTransform::*;

fn heading_transform(m: &MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    if state.inside_section {
        if let Some(section_tag) = &m.wrap_sections {
            helper.end_tag(section_tag)?;
            state.inside_section = false;
        }
    }
    cur.goto_first_child();
    let level = u8::from_str_radix(&cur.node().kind()[5..6], 10).unwrap();
    let tag = format!("h{}", level+m.heading_level-1);
    cur.goto_next_sibling();
    let heading_content = cur.node().utf8_text(src).unwrap().trim_start();
    let attrs =
        if m.no_ids {
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
    if cur.node().parent().unwrap().prev_sibling().is_some() {
        helper.write_html("\n")?;
        helper.enter_inline()?;
    } else {
        helper.enter_inline_s()?;
    }
    
    helper.start_tag(&tag, &attrs)?;
    m.render_elem(src, cur, helper, state)?;
    helper.end_tag(&tag)?;
    helper.exit_inline()?;
    cur.goto_parent();
    
    if !state.inside_section {
        if let Some(section_tag) = &m.wrap_sections {
            helper.start_tag(section_tag, &[])?;
            state.inside_section = true;
        }
    }
    Ok(())
}

fn link_transform(m: &MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    let link_destination = cur.node().child(1).unwrap().utf8_text(src).unwrap();
    helper.start_tag(&"a", &[("href", Some(link_destination.into()))])?;
    cur.goto_first_child();
    cur.goto_first_child();
    m.render_elem(src, cur, helper, state)?;
    cur.goto_parent();
    cur.goto_parent();
    helper.end_tag(&"a")?;
    Ok(())
}

fn image_transform(m: &MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    cur.goto_first_child();
    let alt_text = cur.node().utf8_text(src).unwrap();
    cur.goto_next_sibling();
    let src_url = cur.node().utf8_text(src).unwrap();
    helper.self_close_tag(&"img", &[
        ("src", Some(src_url.into())),
        ("alt", Some(alt_text.into()))
    ])?;
    cur.goto_parent();
    Ok(())
}

fn text_transform(m: &MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    let text = cur.node().utf8_text(src).unwrap().trim_start();
    helper.write_text(text)?;
    Ok(())
}

fn list_transform(m: &MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    let node = cur.node();
    let markers = (0..node.child_count()).map(|i| node.child(i).unwrap().child(0).unwrap().utf8_text(src).unwrap()).collect::<Vec<_>>();
    let is_bulleted = markers.iter().all(|&m| m == "-" || m == "*");
    let is_numbered_forward = markers.iter().enumerate().all(|(i, &m)| m == &((i+1).to_string() + "."));
    let is_numbered_backward  = markers.iter().enumerate().all(|(i, &m)| m == &((markers.len() - i).to_string() + "."));

    let (tag, attrs): (_, &[(_, Option<String>)]) =
        if is_bulleted {
            ("ul", &[])
        } else if is_numbered_forward {
            ("ol", &[])
        } else if is_numbered_backward {
            ("ol", &[("reversed", None)])
        } else {
            todo!("unknown list type {:?}", markers)
        };
    
    m.render_elem_seq(helper, false, &Full(&tag), cur, src, attrs, state)
}

fn list_item_transform(m: &MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    cur.goto_first_child();
    cur.goto_next_sibling(); // skip list marker and p
    m.render_elem_seq(helper, true, &OptionalClose("li"), cur, src, &[], state)?;
    cur.goto_parent();
    Ok(())
}

fn checkbox_transform(m: &MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    let node = cur.node();
    let is_checked = node.utf8_text(src).unwrap() == "[x]";
    let mut attrs = vec![
        ("type", Some("checkbox".into())),
        ("disabled", None)
    ];
    if is_checked { attrs.push(("checked", None)); }
    m.render_elem_seq(helper, false, &SelfClose("input"), cur, src, &attrs, state)
}

fn codeblock_transform(m: &MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    cur.goto_first_child();

    if cur.node().kind() == "info_string" {
        let info = cur.node().utf8_text(src).unwrap();
        cur.goto_next_sibling();
        let content = cur.node().utf8_text(src).unwrap().trim_end();
        if let Some(caps) = RE_ADMONITION.captures(info) { // admonition
            let class = caps.name("class").unwrap().as_str();
            let title = match caps.name("title") {
                Some(titlematch) => to_title_case(titlematch.as_str()),
                None => {
                    to_title_case(class)
                }
            };
            helper.start_tag(&"div", &[("class", Some(format!("admonion {class}")))])?;
            helper.push_elem(&["h3"], title)?;
            helper.push_elem(&["p"], content)?;
            helper.end_tag(&"div")?;

        } else { // possibly-highlighted code block
            helper.enter_inline()?;
            helper.start_tag(&"pre", &[("data-lang", Some(info.into()))])?;
            helper.start_tag(&"code", &[])?;

            let highligher = m.try_get_highlighter(info);

            if let Some(hl) = highligher {
                helper.write_html(hl.highlight(content.as_bytes())?)?;
            } else {
                helper.write_text(content)?;
            };
            
            helper.end_tag(&"code")?;
            helper.end_tag(&"pre")?;
            helper.exit_inline()?;
        }
    } else { // no info, plain code block
        let content = cur.node().utf8_text(src).unwrap().trim_end();

        helper.push_elem(&["pre", "code"], content)?;
    }


    cur.goto_parent();
    Ok(())
}

fn table_cell_transform(m: &MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    let node = cur.node();
    let is_header = node.parent().unwrap().kind() == "table_header_row";
    let tag = if is_header { "th" } else { "td" };
    m.render_elem_seq(helper, true, &Full(&tag), cur, src, &[], state)
}

fn slb_transform(m: &MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    helper.write_text("\n")?;
    Ok(())
}

static MD_TRANSFORMERS: phf::Map<&'static str, NodeTransform> = phf_map! {
    "document" => Simple { tag: NoTags, inline: false, attrs: &[] },
    "atx_heading" => Custom(heading_transform),
    "heading_content" => Simple { tag: NoTags, inline: true, attrs: &[] },
    "text" => Custom(text_transform),
    "paragraph" => Simple {tag: OptionalClose("p"), inline: true, attrs: &[] },
    "emphasis" => Simple {tag: Full("em"), inline: true, attrs: &[] },
    "link" => Custom(link_transform),
    "image" => Custom(image_transform),
    "thematic_break" => Simple {tag: SelfClose("hr"), inline: false, attrs: &[] },
    "strong_emphasis" => Simple {tag: Full("strong"), inline: true, attrs: &[] },
    "strikethrough" => Simple {tag: Full("s"), inline: true, attrs: &[] },
    "code_span" => Simple {tag: Full("code"), inline: true, attrs: &[] },
    "block_quote" => Simple {tag: Full("blockquote"), inline: false, attrs: &[] },
    "tight_list" => Custom(list_transform),
    "loose_list" => Custom(list_transform),
    "list_item" =>  Custom(list_item_transform),
    "task_list_item" => Custom(list_item_transform),
    "task_list_item_marker" => Custom(checkbox_transform),
    "fenced_code_block" => Custom(codeblock_transform),

    // TODO: like obsidian
    "indented_code_block" => Simple {tag: Full("aside"), inline: true, attrs: &[] },

    "table" => Simple {tag: Full("table"), inline: false, attrs: &[] },
    "table_header_row" => Simple {tag: Full("tr"), inline: false, attrs: &[("class", Some("header"))] },
    "table_data_row" => Simple {tag: Full("tr"), inline: false, attrs: &[] },
    "table_cell" => Custom(table_cell_transform),

    "table_delimiter_row" => Skip,

    "line_break" => Simple {tag: SelfClose("br"), inline: false, attrs: &[] },
    "soft_line_break" => Custom(slb_transform),

    // skipped by custom transforms:
    // list_marker, atx_hX_marker,link_dest, link_text, image_dest, image_text
};

impl MDNya {
    pub fn new(close_all_tags: bool, wrap_sections: Option<String>, heading_level: u8, no_ids: bool,) -> Self {
        Self { 
            highlighters: HashMap::new(),
            highlighters_aliases: HashMap::new(),
            close_all_tags,
            wrap_sections,
            heading_level,
            no_ids
        }
    }

    pub fn add_highlighter(&mut self, lang: TSHLang) {
        let name = lang.name().to_string();
        let aliases: Vec<String> = lang.aliases().iter().cloned().map(|s| s.to_string()).collect();
        self.highlighters.insert(name.clone(), lang);
        for alias in aliases {
            self.highlighters_aliases.insert(alias, name.clone());
        }
    }

    fn try_get_highlighter(&self, name: impl AsRef<str>) -> Option<&TSHLang> {
        let name_str = name.as_ref().to_string();
        let name_str = self.highlighters_aliases.get(&name_str).unwrap_or(&name_str);
        self.highlighters.get(name_str)
    }

    fn render_elem(&self, src: &[u8], cur: &mut TreeCursor, helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
        let node = cur.node();
        let kind = node.kind();
        let behave = MD_TRANSFORMERS.get(kind).unwrap_or_else(|| {
            println!("\n--- s-expr:\n{}\n---\n", node.parent().unwrap_or(node).to_sexp());
            panic!("{}", kind)
        });
        match behave {
            Simple {tag, inline, attrs} => {
                let attrs: Vec<_> = attrs.iter().map(|(k, v)| (*k, v.as_ref().map(|s| s.to_string()))).collect();
                self.render_elem_seq(helper, *inline, tag, cur, src, &attrs, state)?;
            },
            Custom(f) => f(&self, cur, src, helper, state)?,
            Skip => ()
        }

        Ok(())
    }

    fn render_elem_seq(&self, helper: &mut html::HTMLWriter, inline: bool, tag: &TagBehavior, cur: &mut TreeCursor, src: &[u8], attrs: &[(&str, Option<String>)], state: &mut MDNyaState) -> MdResult {
        let switched_inline = !helper.is_inline && inline;
        if switched_inline {
            helper.enter_inline()?;
        }
        match tag {
            Full(t) | OptionalClose(t) => helper.start_tag(&t, attrs)?,
            SelfClose(t) => helper.self_close_tag(&t, attrs)?,
            NoTags => ()
        }
        if cur.goto_first_child() {
            loop { // for each child
                let node = cur.node();
                let mut skipped_p = false;
                if node.kind() == "paragraph" && node.child_count() == 1 {
                    if node.child(0).unwrap().kind() == "image" {
                        cur.goto_first_child();
                        skipped_p = true;
                    }
                }
                self.render_elem(src, cur, helper, state)?;
                if skipped_p  {
                    cur.goto_parent();
                }
                if !cur.goto_next_sibling() {
                    break;
                }
            }
            cur.goto_parent();
        }
        match tag {
            Full(t) | OptionalClose(t) => helper.end_tag(&t)?,
            SelfClose(_) | NoTags => (),
        }
        Ok(if switched_inline {
            helper.exit_inline()?;
            if let Some(next) = cur.node().next_sibling() {
                if cur.node().kind() == "paragraph" && next.kind() != "atx_heading"{
                    helper.write_html("\n")?;
                }
            }
        })
    }

    pub fn render(&self, md_source: &[u8], out: Box<dyn std::io::Write>) -> MdResult {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(unsafe { tree_sitter_markdown() }).unwrap();
        let tree = parser.parse(md_source, None).unwrap();
        let mut cur = tree.root_node().walk();
        let mut helper = html::HTMLWriter {
            is_inline: false,
            close_all_tags: self.close_all_tags,
            indent: 4,
            indent_level: 0,
            writer: out,
        };
        let mut state = MDNyaState { inside_section: false };
        self.render_elem(md_source, &mut cur, &mut helper, &mut state)?;
        if state.inside_section {
            if let Some(section_tag) = &self.wrap_sections {
                helper.end_tag(section_tag)?;
                state.inside_section = false;
            }
        }
        Ok(())
    }
}