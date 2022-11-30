use std::{collections::HashMap};

use phf::phf_map;
use tree_sitter::{Node, TreeCursor};

use mdnya_hl::{TSHLang, CodeHighlighter};

mod html;

extern "C" { fn tree_sitter_markdown() -> tree_sitter::Language; }

pub struct MDNya {
    highlighters: HashMap<String, TSHLang>,
    highlighters_aliases: HashMap<String, String>,
    close_all_tags: bool,
    wrap_sections: Option<String>,
    heading_level: u8,
    no_ids: bool,
}

#[derive(Clone, PartialEq)]
enum TagBehavior {
    Full(&'static str),
    OptionalClose(&'static str),
    SelfClose(&'static str),
    NoTags
}
use TagBehavior::*;

// #[derive(Clone)]
// struct SimpleBehavior {
    
// }

#[derive(Clone)]
enum NodeTransform {
    Simple {
        tag: TagBehavior,
        inline: bool,
    },
    Custom(fn(&MDNya, &mut TreeCursor, &[u8], &mut Box<&mut dyn std::io::Write>, &mut html::HTMLWriter) -> std::io::Result<()>),
}
use NodeTransform::*;

fn heading_transform(m: &MDNya, cur: &mut TreeCursor, src: &[u8],  out: &mut Box<&mut dyn std::io::Write>, writer: &mut html::HTMLWriter) -> std::io::Result<()> {
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
    writer.start_tag(out, &tag, attrs.as_slice())?;
    writer.is_inline = true;
    m.render_elem(src, cur, out, writer)?;
    writer.end_tag(out, &tag)?;
    writer.is_inline = false;
    cur.goto_parent();
    Ok(())
}

fn text_transform(m: &MDNya, cur: &mut TreeCursor, src: &[u8],  out: &mut Box<&mut dyn std::io::Write>, writer: &mut html::HTMLWriter) -> std::io::Result<()> {
    let text = cur.node().utf8_text(src).unwrap().trim_start();
    html_escape::encode_text_to_writer(text, out)
}

static MD_TRANSFORMERS: phf::Map<&'static str, NodeTransform> = phf_map! {
    "document" => Simple { tag: NoTags, inline: false },
    "atx_heading" => Custom(heading_transform),
    "heading_content" => Simple { tag: NoTags, inline: true },
    "text" => Custom(text_transform),
    "paragraph" => Simple {tag: OptionalClose("p"), inline: true},
    "emphasis" => Simple {tag: Full("em"), inline: true},
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

    fn add_highlighter(&mut self, lang: TSHLang) {
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

    fn render_elem(&self, src: &[u8], cur: &mut TreeCursor, out: &mut impl std::io::Write, helper: &mut html::HTMLWriter) -> std::io::Result<()> {
        let node = cur.node();
        let kind = node.kind();
        let behave = MD_TRANSFORMERS.get(kind).unwrap_or_else(|| {
            println!("\n--- s-expr:\n{}\n---\n", node.to_sexp());
            panic!("{}", kind)
        });
        match behave {
            Simple {tag, inline} => {
                match tag {
                    Full(t) | OptionalClose(t) => helper.start_tag(out, t, &[])?,
                    SelfClose(t) => helper.self_close_tag(out, t, &[])?,
                    NoTags => ()
                }
                helper.is_inline = *inline;
                if cur.goto_first_child() {
                    loop { // for each child
                        self.render_elem(src, cur, out, helper)?;
                        if !cur.goto_next_sibling() {
                            break;
                        }
                    }
                    cur.goto_parent();
                }
                match tag {
                    Full(t) | OptionalClose(t) => helper.end_tag(out, t)?,
                    SelfClose(_) | NoTags => {}
                }
            },
            Custom(f) => f(&self, cur, src, &mut Box::new(out as &mut dyn std::io::Write), helper)?,
        }

        Ok(())
    }

    pub fn render(&self, md_source: &[u8], out: &mut impl std::io::Write) -> std::io::Result<()> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(unsafe { tree_sitter_markdown() }).unwrap();
        let tree = parser.parse(md_source, None).unwrap();
        let mut cur = tree.root_node().walk();
        let mut helper = html::HTMLWriter {
            is_inline: false,
            close_all_tags: self.close_all_tags,
            indent: "    ".into(),
            indent_level: 0,
        };
        
        self.render_elem(md_source, &mut cur, out, &mut helper)?;
        Ok(())
    }
}