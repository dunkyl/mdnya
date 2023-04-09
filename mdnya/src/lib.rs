use std::{collections::HashMap, error::Error};

use regex::Regex;
use lazy_static::lazy_static;
use serde::Serialize;

use crate::html::NO_ATTRS;

mod html;
mod starry;
pub use starry::StarryHighlighter;

pub type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Serialize)]
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
pub struct MdnyaOptions {
    close_all_tags: bool,
    wrap_sections: Option<String>,
    heading_level: u8,
    add_header_ids: bool,
    no_code_lines: bool,
    razor: bool,
    highlighter: Box<dyn starry::Highlighter>,
}

struct MdnyaRenderer<'a> {
    options: MdnyaOptions,
    html_writer: html::HTMLWriter<'a>,
    highlighter: Box<dyn starry::Highlighter>,
}

impl<'a> MdnyaRenderer<'a> {
    fn new(output: Box<dyn std::io::Write + 'a>, options: MdnyaOptions, hl: Box<dyn starry::Highlighter>) -> Self {
        Self {
            html_writer: html::HTMLWriter::new(output, 4, options.close_all_tags),
            options,
            highlighter: hl,
        }
    }

    fn render(input: &str) {

    }
}

pub fn render_markdown<'a>(input: impl AsRef<str>, output: &'a mut impl std::io::Write, options: MdnyaOptions, hl: Box<dyn starry::Highlighter>) -> Result<()> {
    let mut renderer = MdnyaRenderer::new(Box::new(output), options, hl);
    Ok(())
}



impl MdnyaOptions {

    pub fn new(close_all_tags: bool, wrap_sections: Option<String>, heading_level: u8, add_header_ids: bool) -> Self {
        Self { 
            close_all_tags,
            wrap_sections,
            heading_level,
            add_header_ids,
            no_code_lines: false,
            razor: false,
            highlighter: Box::new(StarryHighlighter::new(HashMap::new())),
        }
    }

    fn render_root(&mut self, node: markdown::mdast::Node, htmler: &mut html::HTMLWriter) -> DocumentMetaData {
        use markdown::mdast::*;
        let Node::Root(Root { children, ..}) = node else {
            panic!("non-root node passed to render_root")
        };
        let mut children_iter = children.into_iter();
        let first_child = children_iter.next();
        let (frontmatter, skip1) =
            if let Some(Node::Yaml(Yaml{value, ..})) = &first_child {
                use serde_yaml::from_str;
                let fm = (from_str(value).expect("frontmatter is mapping"), true);
                justlogfox::log_debug!("frontmatter: {:?}", (fm.0));
                fm
            } else {
                justlogfox::log_debug!("document has no frontmatter");
                (serde_yaml::Mapping::new(), false)
            };
        let rest_children = if skip1 {
            (None).into_iter().chain(children_iter)
        } else {
            first_child.into_iter().chain(children_iter)
        };
        let mut tags = (None, vec![]);
        let result = self.render_children(rest_children, &mut tags, htmler);
        if let Err(e) = result {
            justlogfox::log_error!("error while rendering: {}", e);
        }
        for tag in &tags.1 {
            justlogfox::log_trace!("collected tag: #{}", tag);
        }
        
        DocumentMetaData {
            title: tags.0,
            tags: tags.1,
            frontmatter
        }
    }

    fn render_table(&mut self, table: markdown::mdast::Table, caption: Option<Vec<markdown::mdast::Node>>, tags: &mut (Option<String>, Vec<String>), htmler: &mut html::HTMLWriter) -> Result<()> {
        use markdown::mdast::*;
        let Table { children, align, .. } = table;
        let align_attrs = align.iter().map(|align| match align {
            AlignKind::None => vec![],
            AlignKind::Left => vec![("style", Some("text-align: left"))],
            AlignKind::Right => vec![("style", Some("text-align: right"))],
            AlignKind::Center => vec![("style", Some("text-align: center"))],
        }).collect::<Vec<_>>();

        let mut col_num = 0;
        let n_cols = align.len();
        htmler.start("table", NO_ATTRS)?;

        if let Some(caption) = caption {
            self.inline_tag("caption", NO_ATTRS, caption, tags, htmler)?;
        }

        htmler.start("thead", NO_ATTRS)?;

        let mut rows = children.into_iter();

        let Node::TableRow(TableRow { children: header_cells, .. }) = rows.next().expect("table with no header row")
            else { panic!("non-row in table") };
        for cell in header_cells {
            let Node::TableCell(TableCell { children, .. }) = cell
                else { panic!("non-cell in table row") };
            let cell_attrs = &align_attrs[col_num];
            self.inline_tag("th", cell_attrs, children, tags, htmler)?;
            col_num = (col_num + 1) % n_cols;
        }
        htmler.end("thead")?;
        htmler.start("tbody", NO_ATTRS)?;
        for row in rows {
            let Node::TableRow(TableRow { children: cells, .. }) = row
                else { panic!("non-row in table") };
            htmler.start("tr", NO_ATTRS)?;
            for cell in cells {
                let Node::TableCell(TableCell { children, .. }) = cell
                    else { panic!("non-cell in table row") };
                let cell_attrs = &align_attrs[col_num];
                htmler.enter_inline()?;
                htmler.start("td", cell_attrs)?;
                self.render_children(children, tags, htmler)?;
                htmler.end("td")?;
                htmler.exit_inline()?;
                col_num = (col_num + 1) % n_cols;
            }
            htmler.end("tr")?;
        }
        htmler.end("tbody")?;
        htmler.end("table")?;
        Ok(())
    } 

    fn render_children(&mut self, children: impl IntoIterator<Item=markdown::mdast::Node>, tags: &mut  (Option<String>, Vec<String>), htmler: &mut html::HTMLWriter) -> Result<()> {
        use markdown::mdast::*;

        // this is for table captions:
        let mut children = children.into_iter().peekable();
        while let Some(node) = children.next() {
            if matches!(node, Node::Table(_)) { // a table
               if let Some(Node::Paragraph(par)) = children.peek() { // followed by a paragraph
                    if let Some(Node::Text(Text { value, .. })) = par.children.first() {
                        if value.starts_with(": ") { // that starts with ": "
                            let Some(Node::Paragraph(Paragraph { children: mut caption_nodes, .. })) = children.next() else { unreachable!() }; // consume the paragraph
                            let Node::Text(mut text) = caption_nodes.remove(0) else { unreachable!() };
                            text.value = text.value[2..].to_string(); // remove ": "
                            caption_nodes.insert(0, Node::Text(text));
                            let caption = Some(caption_nodes); // is a caption.
                            let Node::Table(table) = node else { unreachable!() };
                            self.render_table(table, caption, tags, htmler)?;
                            continue;
                        }
                    }
               }
            }

            // default case:
            self.render_node(node, tags, htmler)?;
        }
        Ok(())
    }

    fn tag(&mut self, tag: &str, attrs: &[(impl AsRef<str>, Option<impl AsRef<str>>)], children: Vec<markdown::mdast::Node>, tags: &mut (Option<String>, Vec<String>), htmler: &mut html::HTMLWriter) -> Result<()> {
        htmler.start(tag, attrs)?;
        self.render_children(children, tags, htmler)?;
        htmler.end(tag)?;
        Ok(())
    }

    fn inline_tag(&mut self, tag: &str, attrs: &[(impl AsRef<str>, Option<impl AsRef<str>>)], children: Vec<markdown::mdast::Node>, tags: &mut (Option<String>, Vec<String>), htmler: &mut html::HTMLWriter) -> Result<()> {
        htmler.enter_inline()?;
        self.tag(tag, attrs, children, tags, htmler)?;
        htmler.exit_inline()?;
        Ok(())
    }

    fn render_node(&mut self, node: markdown::mdast::Node, tags: &mut (Option<String>, Vec<String>), htmler: &mut html::HTMLWriter) -> Result<()> {
        use markdown::mdast::*;
        match node {
            Node::Heading(Heading { children, depth, .. }) => {

                if self.wrap_sections.is_some() {
                    htmler.maybe_exit_section()?;
                }

                lazy_static! {
                    static ref FRAGMENT_REMOVE_RE: Regex = Regex::new(r"[^a-zA-Z0-9-]").unwrap();
                }
                let mut attrs = vec![];
                
                if self.add_header_ids {
                    let fragment = children.iter()
                                   .fold(String::new(), |acc, node| acc + node.to_string().as_str())
                                   .to_ascii_lowercase()
                                   .replace(' ', "-");
                    let fragment = FRAGMENT_REMOVE_RE.replace_all(&fragment, "").to_string();
                    
                    attrs.push(("id", Some(fragment)));
                }

                let tag = format!("h{}", (depth + self.heading_level) as isize - 1);
                
                justlogfox::log_debug!("heading: {} {:?}", tag, attrs);

                // capture title HTML for metadata
                if (tags.0.is_none()) && (tag == "h1") && (htmler.indent_level == 0) {
                    let mut tempbuf: Vec<u8> = vec![];
                    {
                        let tempwriter = Box::new(&mut tempbuf);
                        let mut temphtmler = html::HTMLWriter::new(tempwriter, htmler.indent, htmler.close_all_tags);

                        self.inline_tag(&tag, &attrs, children, tags, &mut temphtmler)?;
                    }

                    let titlehtml = String::from_utf8(tempbuf).unwrap().split_once('>').unwrap().1.replace("</h1>", "").trim().to_string();
                    htmler.write_html(&titlehtml)?;
                    tags.0 = Some(titlehtml);
                }
                else {
                    self.inline_tag(&tag, &attrs, children, tags, htmler)?;
                }


                if let Some(section_tag) = &self.wrap_sections {
                    htmler.enter_section(section_tag)?;
                }
            }
            Node::Text(Text { value, .. }) => {

                lazy_static! {
                    static ref TAG_RE: Regex = Regex::new(r"#[a-zA-Z0-9_-]+").unwrap();
                }

                let new_tags = TAG_RE.find_iter(&value).map(|m| m.as_str()[1..].to_string()).collect::<Vec<_>>();
                for tag in &new_tags {
                    justlogfox::log_debug!("tag: #{}", tag);
                }
                tags.1.extend(new_tags);

                htmler.write_text(value)?;
            }
            Node::Emphasis(Emphasis { children, .. }) => {
                self.tag("em", NO_ATTRS, children, tags, htmler)?;
            }
            Node::Strong(Strong { children, .. }) => {
                self.tag("strong", NO_ATTRS, children, tags, htmler)?;
            }
            Node::Delete(Delete { children, .. }) => {
                self.tag("del", NO_ATTRS, children, tags, htmler)?;
            }
            Node::BlockQuote(BlockQuote { children, .. }) => {
                self.tag("blockquote", NO_ATTRS, children, tags, htmler)?;
            }
            Node::Paragraph(Paragraph { children, .. }) => {

                if children.len() != 1
                   || !matches!(children[0], Node::Image(_)) {
                    
                    self.inline_tag("p", NO_ATTRS, children, tags, htmler)?;
                    
                } else {
                    let mut nodes = children.into_iter();
                    let Node::Image(Image { url, title, alt, .. }) = nodes.next().unwrap()
                        else { unreachable!(); };

                    if title.is_some() { todo!("title was {:?}", title); }
                    htmler.self_close_tag("img", &[("src", Some(&url)), ("alt", Some(&alt))])?;
                }
            },
            Node::Link(Link { children, url, title, .. }) => {
                if title.is_some() { todo!("title was {:?}", title); }
                self.tag("a", &[("href", Some(&url))], children, tags, htmler)?;
            }
            Node::Image(Image { url, title, alt, .. }) => {
                if title.is_some() { todo!("title was {:?}", title); }
                htmler.self_close_tag("img", &[("src", Some(&url)), ("alt", Some(&alt))])?;
            }
            Node::Code(Code { value, lang, meta, .. }) => {
                if meta.is_some() { todo!("meta was {:?}", meta); }
                let lang = lang.as_deref();

                justlogfox::log_trace!("code: {:?}\n{}", lang, value);

                
                if Some("@") == lang && self.razor { // special case for razor code block
                    htmler.write_html("@{\n")?;
                    htmler.write_html(value)?;
                    htmler.write_html("\n}")?;
                    return Ok(());
                }

                let add_code_lines = 
                    if self.no_code_lines {
                        |text: &str| text.trim_end().to_string()
                    } else {
                        |text: &str| {
                            text.trim_end().lines().map(|line| {
                                format!("<span class=\"code-line\">{line}</span>")
                            }).collect::<Vec<_>>().join("\n")
                        }
                    };

                lazy_static! {
                    static ref RE_ADMONITION: Regex = Regex::new(r"\{(?P<class>\w+)\}\w*((?P<title>\w[\w\s]*))?").unwrap();
                }
                let mut attrs = vec![];
                let code =
                    if let Some(info) = lang {
                        let adm_match = RE_ADMONITION.captures(info);
                        if let Some(captures) = adm_match {
                            let class = captures.name("class").unwrap().as_str();
                            let title = captures.name("title")
                                                .map(|m| m.as_str())
                                                .map(ToString::to_string)
                                                .unwrap_or_else(|| to_title_case(class));
                            let class_attr = format!("admonition {class}");
                            htmler.start("div", &[("class", Some(&class_attr))])?;
                            htmler.enter_inline()?;
                            htmler.start("h3", NO_ATTRS)?;
                            htmler.write_text(title)?;
                            htmler.end("h3")?;
                            htmler.exit_inline()?;
                            htmler.enter_inline()?;
                            htmler.start("p", NO_ATTRS)?;
                            htmler.write_text(value)?;
                            htmler.end("p")?;
                            htmler.exit_inline()?;
                            htmler.end("div")?;
                            return Ok(());
                        }

                        attrs.push(("data-lang", Some(info)));
                        
                        self.highlighter.highlight(info, &value)?


                    } else {
                        html_escape::encode_text(&value).to_string()
                    };
                

                let code = add_code_lines(&code);

                htmler.enter_inline()?;
                htmler.start("pre", NO_ATTRS)?;
                htmler.start("code", &attrs)?;
                htmler.write_html(code)?;
                htmler.end("code")?;
                htmler.end("pre")?;
                htmler.exit_inline()?;
            }
            Node::ThematicBreak(ThematicBreak {..}) => {
                htmler.self_close_tag("hr", NO_ATTRS)?;
            }
            Node::List(List { children, start: Some(start_i), .. }) => {
                let start_str = start_i.to_string();
                let attrs =
                    if start_i != 1 {
                        vec![("start", Some(start_str.as_str()))]
                    } else {
                        vec![]
                    };
                htmler.start("ol", &attrs)?;
                self.render_children(children, tags, htmler)?;
                htmler.end("ol")?;
            }
            Node::List(List { children, start: None, .. }) => {
                self.tag("ul", NO_ATTRS, children, tags, htmler)?;
            }
            Node::ListItem(ListItem { children, checked, .. }) => {

                htmler.enter_inline()?;
                htmler.start("li", NO_ATTRS)?;

                if let Some(is_checked) = checked {
                    let mut attrs = vec![("type", Some("checkbox")), ("disabled", None)];
                    if is_checked { attrs.push(("checked", None)) }

                    htmler.self_close_tag("input", &attrs)?;
                }

                if children.len() != 1
                   || !matches!(children[0], Node::Paragraph(_)) {
                    
                    self.render_children(children, tags, htmler)?;
                    
                } else {
                    let mut nodes = children.into_iter();
                    let Node::Paragraph(Paragraph {children, .. }) = nodes.next().unwrap()
                        else { unreachable!(); };

                    self.render_children(children, tags, htmler)?;
                }
                
                htmler.end("li")?;
                htmler.exit_inline()?;
            }
            Node::InlineCode(InlineCode { value, .. }) => {
                htmler.start("code", NO_ATTRS)?;
                htmler.write_text(value)?;
                htmler.end("code")?;
            }
            Node::Table(table) => { // table has no caption
                self.render_table(table, None, tags, htmler)?;
            }
            Node::Html(Html { value, .. }) => {
                htmler.write_html("\n")?;
                htmler.write_html(value)?;
                htmler.write_html("\n")?;
            }



            Node::TableRow(_) |
            Node::TableCell(_) => {
                panic!("table row or cell passed to render_node");
            }

            Node::Yaml(_) | Node::Root(_) => {
                panic!("root or yaml node passed to render_node");
            }
            _ => {
                let s = format!("{:?}", node);
                let y = s.split_ascii_whitespace().next().unwrap();
                todo!("render_node: {}", y);
            }
        };
        Ok(())
    }

    pub fn render(&mut self, md_source: &str, out: Box<dyn std::io::Write>) -> Result<DocumentMetaData> {
        justlogfox::log_trace!("rendering markdown source, {} bytes", (md_source.len()));

        let mut options = markdown::Options::gfm();
        options.parse.constructs.frontmatter = true;
        let ast = markdown::to_mdast(md_source, &options.parse).unwrap();

        let mut html_writer = html::HTMLWriter::new(out, 4, self.close_all_tags);

        let meta = self.render_root(ast, &mut html_writer);

        Ok(meta)
    }
}
