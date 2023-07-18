use std::{sync::Arc, collections::HashMap};

use html::Attributes;
use regex::Regex;
use lazy_static::lazy_static;
use serde::Serialize;

use crate::html::NO_ATTRS;

mod html;
mod starry;

pub type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Serialize, Clone)]
pub struct DocumentMetaData {
    title: Option<String>,
    tags: Vec<String>,
    frontmatter: serde_yaml::Mapping,
}

fn to_title_case(s: impl AsRef<str>) -> String {
    let mut c = s.as_ref().chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect()
    }
}

#[derive(Clone)]
pub struct MdnyaOptions {
    close_all_tags: bool,
    wrap_sections: Option<String>,
    wrap_document: Option<Vec<String>>,
    heading_level: u8,
    add_header_ids: bool,
    no_code_lines: bool,
    highlighter: Option<Arc<dyn starry::Highlighter>>,
    razor: bool,
}

struct MdnyaRenderer<'a> {
    options: MdnyaOptions,
    html: html::HTMLWriter<'a>,
    meta: DocumentMetaData,
}

impl<'a> MdnyaRenderer<'a> {
    fn new(output: Box<dyn std::io::Write + 'a>, options: MdnyaOptions) -> Self {
        Self {
            html: html::HTMLWriter::new(output, 4, options.close_all_tags),
            options,
            meta: DocumentMetaData {
                title: None,
                tags: Vec::new(),
                frontmatter: serde_yaml::Mapping::new(),
            }
        }
    }
}

pub fn render_markdown(input: impl AsRef<str>, output: & mut impl std::io::Write, options: MdnyaOptions) -> Result<DocumentMetaData> {
    let renderer = MdnyaRenderer::new(Box::new(output), options);
    renderer.render_document(input.as_ref())
}



impl MdnyaOptions {

    pub fn new(close_all_tags: bool, wrap_sections: Option<String>, wrap_document: Option<Vec<String>>, heading_level: u8, add_header_ids: bool) -> Self {
        Self { 
            close_all_tags,
            wrap_sections,
            wrap_document,
            heading_level,
            add_header_ids,
            no_code_lines: false,
            highlighter: None,
            razor: true,
        }
    }

    pub fn with_starry_night(self) -> Self {
        static mut STARRY: Option<Arc<starry::StarryHighlighter>> = None;
        static STARRY_INIT: std::sync::Once = std::sync::Once::new();

        STARRY_INIT.call_once(|| {
            unsafe {
                STARRY = Some(Arc::new(starry::StarryHighlighter::new(HashMap::new())));
            }
        });

        Self {
            highlighter: Some(unsafe {STARRY.clone().unwrap()}),
            .. self
        }
    }

}

use markdown::mdast::*;

impl<'a> MdnyaRenderer<'a> {

    // write to the output and collect metadata
    fn render_document(mut self, input: &str) -> Result<DocumentMetaData> {
        justlogfox::log_trace!("rendering {} bytesof  markdown", (input.len()));

        let mut options = markdown::Options::gfm();
        options.parse.constructs.frontmatter = true;
        let ast = markdown::to_mdast(input, &options.parse).unwrap();
        let Node::Root(Root { children: mut root_nodes, ..}) = ast else { unreachable!() };

        if let Some(tags) = &self.options.wrap_document {
            for tag in tags {
                self.html.start(tag, NO_ATTRS)?;
            }
        }

        let Some(first_node) = root_nodes.first() else {
            justlogfox::log_warn!("document has no content");
            return Ok(self.meta);
        }; 

        // if the first node is a yaml node, it's the frontmatter
        if let Node::Yaml(Yaml{value, ..}) = first_node {
            let fm = serde_yaml::from_str(value).expect("frontmatter is yml map");
            justlogfox::log_debug!("frontmatter: {:?}", fm);
            self.meta.frontmatter = fm;
            root_nodes.remove(0); // skip when rendering HTML
        } else {
            justlogfox::log_debug!("no frontmatter");
        };

        self.render_seq(root_nodes.iter())?;

        if let Some(tags) = &self.options.wrap_document {
            for tag in tags {
                self.html.end(tag)?;
            }
        }

        Ok(self.meta)
    }

    fn render_seq<'node>(&mut self, nodes: impl Iterator<Item=&'node Node>) -> Result<()> {
        // this is for table captions:
        let mut nodes = nodes.peekable();
        while let Some(node) = nodes.next() {
            // a table
            if let Node::Table(table) = node { 
                // followed by a paragraph
                if let Some(Node::Paragraph(Paragraph { children: par_nodes, .. } )) = nodes.peek() { 
                    // with text
                    if let Some( Node::Text(text_node @ Text { value, .. })) = par_nodes.first() {
                        // that starts with ": "
                        if let Some(clean) = value.strip_prefix(": ") { 
                            let new_text_node = [Node::Text( // text node without ": "
                                Text { value: clean.to_string(), .. text_node.clone() })];
                            
                            let caption = new_text_node.iter().chain(par_nodes.iter().skip(1));

                            self.render_table(table, Some(caption))?;
                            nodes.next(); // skip the paragraph
                            continue;
                        }
                    }
               }
            }

            // default case:
            self.render_node(node)?;
        }
        Ok(())
    }

    fn render_list(&mut self, node: &List) -> Result<()> {
        let (tag, attrs) = match node.start {
            Some(1) => ("ol", vec![]),
            Some(i) => ("ol", vec![("start", Some(i.to_string()))]),
            _ => ("ul", vec![]),
        };
        self.html.start(tag, &attrs)?;
        for li in &node.children {
            let Node::ListItem(ListItem { children: li_nodes, checked, .. }) = li 
                else { panic!("non-li in list") };
            self.html.enter_inline()?;
            self.html.start("li", NO_ATTRS)?;

            // add checkbox
            if let Some(is_checked) = checked {
                let mut attrs = vec![("type", Some("checkbox")), ("disabled", None)];
                if *is_checked { attrs.push(("checked", None)) }

                self.html.void_tag("input", &attrs, false)?;
            }

            if li_nodes.len() == 1 {
                let only = li_nodes.first().unwrap();
                if let Node::Paragraph(Paragraph { children: par_nodes, .. }) = only {
                    self.render_seq(par_nodes.iter())?;
                } else {
                    self.render_node(only)?;
                }
            } else {
                self.render_seq(li_nodes.iter())?;
            }
            
            self.html.end("li")?;
            self.html.exit_inline()?;
        }
        self.html.end(tag)?;
        Ok(())
    }

    fn render_text(&mut self, text: &str) -> Result<()> {
        lazy_static! {
            static ref TAG_RE: Regex = Regex::new(r"#[a-zA-Z0-9_-]+").unwrap();
        }

        let escaped = html_escape::encode_text(text);

        let tagged_text = TAG_RE.replace(&escaped, |caps: &regex::Captures| {
            let tag = caps[0][1..].to_string();
            let tagged_tag = format!("<span class=\"tag\">{}</span>", tag);
            self.meta.tags.push(tag);
            tagged_tag
        });

        self.html.write_html(tagged_text)?;
        Ok(())
    }

    fn render_paragraph(&mut self, node: &Paragraph) -> Result<()> {
        lazy_static!(
            static ref RAZOR_STATEMENT_RE: Regex = Regex::new(r"^\s*@\w+").unwrap();
        );

        let first = node.children.first().unwrap();

        // TODO: when x && let Pattern stabilized, here
        if self.options.razor {
            if let Node::Text(Text { value, .. }) = first {
                if RAZOR_STATEMENT_RE.is_match(value) {
                    self.html.write_html(value)?;
                    self.html.write_html("\n")?;
                    return Ok(());
                }
            }
        }
        // if it's just an image, don't wrap it in a p tag
        if node.children.len() == 1 { 
            if let Node::Image(_) = first { 
                self.render_node(first)?;
                return Ok(());
            }
        }
        self.tag_wrap_inline("p", NO_ATTRS, node.children.iter())?;
        Ok(())
        
    }

    fn render_header(&mut self, node: &Heading) -> Result<()> {
        if self.options.wrap_sections.is_some() {
            self.html.maybe_exit_section()?;
        }

        lazy_static! {
            static ref FRAGMENT_REMOVE_RE: Regex = Regex::new(r"[^a-zA-Z0-9-]").unwrap();
        }
        let mut attrs = vec![];

        let Heading { children, depth, .. } = node;
        
        if self.options.add_header_ids {
            let fragment = children.iter()
                           .fold(String::new(), |acc, node| acc + node.to_string().as_str())
                           .to_ascii_lowercase()
                           .replace(' ', "-");
            let fragment = FRAGMENT_REMOVE_RE.replace_all(&fragment, "").to_string();
            
            attrs.push(("id", Some(fragment)));
        }

        let tag = format!("h{}", (depth + self.options.heading_level) as isize - 1);
        
        justlogfox::log_debug!("heading: {} {:?}", tag, attrs);

        // capture title HTML for metadata
        if (self.meta.title.is_none()) && (tag == "h1") && (self.html.indent_level == 0) {
            let mut tempbuf: Vec<u8> = vec![];
            {   
                let mut html = html::HTMLWriter::new(Box::new(&mut tempbuf), 0, true);
                html.is_inline = true;
                let mut temp_renderer = MdnyaRenderer {
                    html,
                    meta: self.meta.clone(),
                    options: self.options.clone(),
                };

                temp_renderer.render_seq(children.iter())?;
            }
            let title_html = String::from_utf8(tempbuf).unwrap();
            justlogfox::log_debug!("captured title html: {}", title_html);

            self.html.enter_inline()?;
            self.html.start(&tag, &attrs)?;
            self.html.write_html(&title_html)?;
            self.html.end(&tag)?;
            self.html.exit_inline()?;
            self.meta.title = Some(title_html);
        }
        else {
            self.tag_wrap_inline(&tag, &attrs, children.iter())?;
        }


        if let Some(section_tag) = &self.options.wrap_sections {
            self.html.enter_section(section_tag)?;
        }
        Ok(())
    }

    fn render_codeblock(&mut self, node: &Code) -> Result<()> {
        let Code { value, meta, lang, .. } = node;
        let lang = lang.as_deref();

        justlogfox::log_trace!("code: {:?}\n{}", lang, value);

        // special case for razor code block
        if Some("@") == lang && self.options.razor { 
            self.html.write_html("@{\n")?;
            self.html.write_html(value)?;
            self.html.write_html("\n}\n\n")?;
            return Ok(());
        }

        let add_code_lines = 
            if self.options.no_code_lines {
                |text: &str| text.trim_end().to_string()
            } else {
                |text: &str| {
                    text.trim_end().lines().map(|line| {
                        format!("<span class=\"code-line\">{line}</span>")
                    }).collect::<Vec<_>>().join("\n")
                }
            };

        lazy_static! {
            static ref RE_ADMONITION: Regex = Regex::new(r"\{(\w+)\}").unwrap();
        }
        let mut attrs = vec![];
        let code =
            if let Some(info) = lang {
                let adm_match = RE_ADMONITION.captures(info);
                if let Some(captures) = adm_match {
                    let class = &captures[1];
                    let title = meta.clone().unwrap_or_else(|| to_title_case(class));
                    let class_attr = format!("admonition {class}");
                    self.html.start("div", &[("class", Some(&class_attr))])?;
                    self.tag_wrap_text_inline("div", &[("class", Some("admonition-title"))], &title)?;
                    self.tag_wrap_text_inline("p", NO_ATTRS, value)?;
                    self.html.end("div")?;
                    return Ok(());
                }

                attrs.push(("data-lang", Some(info)));
                
                if let Some(highlighter) = &self.options.highlighter {
                    highlighter.highlight(info, value)?
                } else {
                    html_escape::encode_text(&value).to_string()
                }


            } else {
                html_escape::encode_text(&value).to_string()
            };
        

        let code = add_code_lines(&code);

        self.html.enter_inline()?;
        self.html.start("pre", &attrs)?;
        self.html.start("code", NO_ATTRS)?;
        self.html.write_html(code)?;
        self.html.end("code")?;
        self.html.end("pre")?;
        self.html.exit_inline()?;
        Ok(())
    }

    fn render_node(&mut self, node: &Node) -> Result<()> {
        match node {
            // terminal
            Node::Break(_) => self.html.void_tag("br", NO_ATTRS, true)?,
            Node::ThematicBreak(_) => self.html.void_tag("hr", NO_ATTRS, true)?,
            Node::Html(Html { value, .. }) => 
                self.html.write_html(format!("\n{value}\n"))?,
            Node::Image(Image { url, alt, .. }) => 
                self.html.void_tag("img", &[("src", Some(&url)), ("alt", Some(&alt))], false)?,
            

            // simple
            Node::BlockQuote(BlockQuote { children, .. }) => 
                self.tag_wrap("blockquote", NO_ATTRS, children.iter())?,
            Node::Emphasis(Emphasis { children, .. }) => 
                self.tag_wrap("em", NO_ATTRS, children.iter())?,
            Node::Strong(Strong { children, .. }) => 
                self.tag_wrap("strong", NO_ATTRS, children.iter())?,
            Node::Delete(Delete { children, .. }) => 
                self.tag_wrap("del", NO_ATTRS, children.iter())?,
            Node::InlineCode(InlineCode { value, .. }) => 
                self.tag_wrap_text("code", NO_ATTRS, value)?,
            Node::Link(Link { url, children, .. }) => 
                self.tag_wrap("a", &[("href", Some(&url))], children.iter())?,

            // specialized
            Node::Text(Text { value, .. }) => self.render_text(value)?,
            Node::List(list) => self.render_list(list)?,
            Node::Paragraph(par) => self.render_paragraph(par)?,
            Node::Heading(heading) => self.render_header(heading)?,
            Node::Code(codeblock) => self.render_codeblock(codeblock)?,

            // Should be handled by other cases
            Node::Root(_) |
            Node::Yaml(_) |
            Node::ListItem(_) |
            Node::Table(_) |
            Node::TableRow(_) |
            Node::TableCell(_)
                => panic!("unexpected node"),

            // TODO
            Node::FootnoteDefinition(_) |
            Node::FootnoteReference(_) |
            Node::Definition(_) |
            Node::ImageReference(_) |
            Node::LinkReference(_) |
            Node::Toml(_) 
                => todo!("{:?}", node),

            // not enabled
            Node::MdxJsxFlowElement(_) |
            Node::MdxJsxTextElement(_) |
            Node::MdxFlowExpression(_) |
            Node::MdxTextExpression(_) |
            Node::MdxjsEsm(_) |
            Node::InlineMath(_) |
            Node::Math(_)
                => unreachable!()
        };
        Ok(())
    }

    fn tag_wrap<'n, Nodes>(&mut self, tag: &str, attrs: impl Attributes, nodes: Nodes) -> Result<()> 
        where Nodes: Iterator<Item=&'n Node>,
    {
        self.html.start(tag, attrs)?;
        self.render_seq(nodes)?;
        self.html.end(tag)?;
        Ok(())
    }

    fn tag_wrap_text(&mut self, tag: &str, attrs: impl Attributes, text: &str) -> Result<()> {
        self.html.start(tag, attrs)?;
        self.html.write_text(text)?;
        self.html.end(tag)?;
        Ok(())
    }

    fn tag_wrap_text_inline(&mut self, tag: &str, attrs: impl Attributes, text: &str) -> Result<()> {
        self.html.enter_inline()?;
        self.tag_wrap_text(tag, attrs, text)?;
        self.html.exit_inline()?;
        Ok(())
    }

    fn tag_wrap_inline<'n, Nodes>(&mut self, tag: &str, attrs: impl Attributes, nodes: Nodes) -> Result<()> 
        where Nodes: Iterator<Item=&'n Node>,
    {
        self.html.enter_inline()?;
        self.tag_wrap(tag, attrs, nodes)?;
        self.html.exit_inline()?;
        Ok(())
    }

    fn render_table<'node>(&mut self, table: &Table, caption: Option<impl Iterator<Item=&'node Node>>) -> Result<()> {
        let Table { children: rows, align, .. } = table;
        let align_attrs = align.iter().map(|align| match align {
            AlignKind::None => vec![],
            AlignKind::Left => vec![("style", Some("text-align: left"))],
            AlignKind::Right => vec![("style", Some("text-align: right"))],
            AlignKind::Center => vec![("style", Some("text-align: center"))],
        }).cycle();

        self.html.start("table", NO_ATTRS)?;

        if let Some(caption) = caption {
            self.tag_wrap_inline("caption", NO_ATTRS, caption)?;
        }

        // flatten rows into cells
        let cells = rows.iter().flat_map(|row| match row { 
            Node::TableRow(TableRow { children: cells, .. }) => cells,
            _ => panic!("non-row in table"),
        }).map(|cell| match cell {
            Node::TableCell(TableCell { children, .. }) => children,
            _ => panic!("non-cell in table row"),
        }).zip(align_attrs).collect::<Vec<_>>();
        let mut cells = cells.chunks(align.len());

        let header_cells = cells.next().unwrap();

        self.html.start("thead", NO_ATTRS)?;
        for (nodes, attrs) in header_cells {
            self.tag_wrap_inline("th", attrs, nodes.iter())?;
        }
        self.html.end("thead")?;

        self.html.start("tbody", NO_ATTRS)?;
        for row in cells {
            self.html.start("tr", NO_ATTRS)?;
            for (nodes, attrs) in row {
                self.tag_wrap_inline("td", attrs, nodes.iter())?;
            }
            self.html.end("tr")?;
        }
        self.html.end("tbody")?;

        self.html.end("table")?;
        Ok(())
    }
}