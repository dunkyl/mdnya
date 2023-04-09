use std::{process::{Child, ChildStdin, ChildStdout, Stdio}, io::Write, io::Read, path::PathBuf, collections::HashMap, error::Error};

use regex::Regex;
use lazy_static::lazy_static;

use crate::html::NO_ATTRS;

mod html;

type MdResult = core::result::Result<(), Box<dyn std::error::Error>>;

fn to_title_case(s: impl AsRef<str>) -> String {
    let mut c = s.as_ref().chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect()
    }
}
pub struct MDNya {
    close_all_tags: bool,
    wrap_sections: Option<String>,
    heading_level: u8,
    add_header_ids: bool,
    no_code_lines: bool,
    hl_node_proc: InOutProc,
    hl_ready: bool,
    razor: bool,
    rename_langs: std::collections::HashMap<String, String>,
}

use serde::Serialize;

#[derive(Serialize)]
pub struct DocumentMetaData {
    title: Option<String>,
    tags: Vec<String>,
    frontmatter: serde_yaml::Mapping,
}

impl Drop for MDNya {
    fn drop(&mut self) {
        self.hl_node_proc.proc.kill().unwrap();
    }
}


struct InOutProc {
    proc: Child,
    in_: ChildStdin,
    out: ChildStdout,
}

impl InOutProc {
    fn new(mut proc: Child) -> Self {
        let in_ = proc.stdin.take().unwrap();
        let out = proc.stdout.take().unwrap();
        Self { proc, in_, out }
    }
}

const INDEXJS_SRC: &str = include_str!("../../dist/bundle.cjs");

fn ensure_indexjs() -> std::io::Result<PathBuf> {
    let indexjs = dirs::data_local_dir().unwrap().join(".mdnya").join("bundle.cjs");
    if !indexjs.exists() {
        std::fs::create_dir_all(indexjs.parent().unwrap())?;
        std::fs::write(&indexjs, INDEXJS_SRC)?;
    }
    Ok(indexjs)
}




impl MDNya {

    fn wait_for_starry(&mut self) {
        if self.hl_ready { return; }
        let start = std::time::Instant::now();
        justlogfox::log_info!("waiting for starry night");
        {
            let mut buf = [0u8; 6];
            self.hl_node_proc.out.read_exact(&mut buf).unwrap();
            assert_eq!(&buf, b"ready\n");
        }
        let elapsed = start.elapsed();
        justlogfox::log_info!("starry night loaded :D\ntook: {}ms", (elapsed.as_millis()));
        self.hl_ready = true;
    }

    pub fn new(close_all_tags: bool, wrap_sections: Option<String>, heading_level: u8, add_header_ids: bool) -> Self {
        let indexjs = ensure_indexjs();
        let Ok(indexjs) = indexjs else {
            justlogfox::log_error!("failed to setup index.js");
            std::process::exit(1);
        };
        justlogfox::log_info!("index.js bundled at {:?}", indexjs);
        justlogfox::log_info!("starting node");
        let node = std::process::Command::new("node")
            .arg(indexjs)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn().expect("node not found");
        let proc = InOutProc::new(node);
        let rename_langs = HashMap::from([
            ("md".to_string(), "markdown".to_string()),
            ("sh".to_string(), "bash".to_string())
            ]);
        Self { 
            close_all_tags,
            wrap_sections,
            heading_level,
            add_header_ids,
            no_code_lines: false,
            hl_node_proc: proc,
            hl_ready: false,
            razor: false,
            rename_langs
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
        let meta = DocumentMetaData {
            title: tags.0,
            tags: tags.1,
            frontmatter
        };
        meta
    }

    fn render_table(&mut self, table: markdown::mdast::Table, caption: Option<Vec<markdown::mdast::Node>>, tags: &mut (Option<String>, Vec<String>), htmler: &mut html::HTMLWriter) -> MdResult {
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
        let attrs: &[(&str, Option<&str>)] = &[];
        htmler.start("table", attrs)?;

        if let Some(caption) = caption {
            self.simple_inline_tag("caption", caption, tags, htmler)?;
        }

        htmler.start("thead", attrs)?;

        let mut rows = children.into_iter();

        let Node::TableRow(TableRow { children: header_cells, .. }) = rows.next().expect("table with no header row")
            else { panic!("non-row in table") };
        for cell in header_cells {
            let Node::TableCell(TableCell { children, .. }) = cell
                else { panic!("non-cell in table row") };
            let cell_attrs = &align_attrs[col_num];
            htmler.enter_inline()?;
            htmler.start("th", cell_attrs)?;
            self.render_children(children, tags, htmler)?;
            htmler.end("th")?;
            htmler.exit_inline()?;
            col_num = (col_num + 1) % n_cols;
        }
        htmler.end("thead")?;
        htmler.start("tbody", attrs)?;
        for row in rows {
            let Node::TableRow(TableRow { children: cells, .. }) = row
                else { panic!("non-row in table") };
            htmler.start("tr", attrs)?;
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

    fn render_children(&mut self, children: impl IntoIterator<Item=markdown::mdast::Node>, tags: &mut  (Option<String>, Vec<String>), htmler: &mut html::HTMLWriter) -> MdResult {
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

    fn tag(&mut self, tag: &str, attrs: Vec<(&str, Option<impl AsRef<str>>)>, children: Vec<markdown::mdast::Node>, tags: &mut (Option<String>, Vec<String>), htmler: &mut html::HTMLWriter) -> MdResult {
        htmler.start(tag, attrs.as_slice())?;
        self.render_children(children, tags, htmler)?;
        htmler.end(tag)?;
        Ok(())
    }

    fn simple_tag(&mut self, tag: &str, children: Vec<markdown::mdast::Node>, tags: &mut (Option<String>, Vec<String>), htmler: &mut html::HTMLWriter) -> MdResult {
        let attrs: Vec<(&str, Option<&str>)> = vec![];
        self.tag(tag, attrs, children, tags, htmler)?;
        Ok(())
    }

    fn simple_inline_tag(&mut self, tag: &str, children: Vec<markdown::mdast::Node>, tags: &mut (Option<String>, Vec<String>), htmler: &mut html::HTMLWriter) -> MdResult {
        let attrs: Vec<(&str, Option<&str>)> = vec![];
        self.inline_tag(tag, attrs, children, tags, htmler)?;
        Ok(())
    }

    fn inline_tag(&mut self, tag: &str, attrs: Vec<(&str, Option<impl AsRef<str>>)>, children: Vec<markdown::mdast::Node>, tags: &mut (Option<String>, Vec<String>), htmler: &mut html::HTMLWriter) -> MdResult {
        htmler.enter_inline()?;
        self.tag(tag, attrs, children, tags, htmler)?;
        htmler.exit_inline()?;
        Ok(())
    }

    fn render_node(&mut self, node: markdown::mdast::Node, tags: &mut (Option<String>, Vec<String>), htmler: &mut html::HTMLWriter) -> MdResult {
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
                                   .replace(" ", "-");
                    let fragment = FRAGMENT_REMOVE_RE.replace_all(&fragment, "").to_string();
                    
                    attrs.push(("id", Some(fragment)));
                }

                let tag = format!("h{}", (depth + self.heading_level) as isize - 1);
                
                justlogfox::log_debug!("heading: {} {:?}", tag, attrs);

                if (tags.0.is_none()) && (tag == "h1") {
                    let mut tempbuf: Vec<u8> = vec![];
                    {
                        let tempwriter = Box::new(&mut tempbuf);
                        let mut temphtmler = html::HTMLWriter {
                            is_inline: false,
                            indent: htmler.indent,
                            indent_level: htmler.indent_level,
                            close_all_tags: htmler.close_all_tags,
                            section: None,
                            writer: tempwriter,
                        };

                        self.inline_tag(&tag, attrs, children, tags, &mut temphtmler)?;
                    }

                    let titlehtml = String::from_utf8(tempbuf).unwrap().splitn(2, ">").skip(1).next().unwrap().replace("</h1>", "").trim().to_string();
                    htmler.write_html(&titlehtml)?;
                    tags.0 = Some(titlehtml);
                }
                else {
                    self.inline_tag(&tag, attrs, children, tags, htmler)?;
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
                self.simple_tag("em", children, tags, htmler)?;
            }
            Node::Strong(Strong { children, .. }) => {
                self.simple_tag("strong", children, tags, htmler)?;
            }
            Node::Delete(Delete { children, .. }) => {
                self.simple_tag("del", children, tags, htmler)?;
            }
            Node::BlockQuote(BlockQuote { children, .. }) => {
                self.simple_tag("blockquote", children, tags, htmler)?;
            }
            Node::Paragraph(Paragraph { children, .. }) => {

                if children.len() != 1
                   || !matches!(children[0], Node::Image(_)) {
                    
                    self.simple_inline_tag("p", children, tags, htmler)?;
                    
                } else {
                    let mut nodes = children.into_iter();
                    let Node::Image(Image { url, title, alt, .. }) = nodes.next().unwrap()
                        else { unreachable!(); };

                    if title != None { todo!("title was {:?}", title); }
                    htmler.self_close_tag("img", &[("src", Some(&url)), ("alt", Some(&alt))])?;
                }
            },
            Node::Link(Link { children, url, title, .. }) => {
                if title != None { todo!("title was {:?}", title); }
                htmler.start("a", &[("href", Some(&url))])?;
                self.render_children(children, tags, htmler)?;
                htmler.end("a")?;
            }
            Node::Image(Image { url, title, alt, .. }) => {
                if title != None { todo!("title was {:?}", title); }
                htmler.self_close_tag("img", &[("src", Some(&url)), ("alt", Some(&alt))])?;
            }
            Node::Code(Code { value, lang, meta, .. }) => {
                if meta != None { todo!("meta was {:?}", meta); }
                let lang = lang.as_deref();

                justlogfox::log_debug!("code: {:?}\n{}", lang, value);

                let mut attrs = vec![];
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
                            text.trim_end().split('\n').map(|line| {
                                format!("<span class=\"code-line\">{line}</span>")
                            }).collect::<Vec<_>>().join("\n")
                        }
                    };

                lazy_static! {
                    static ref RE_ADMONITION: Regex = Regex::new(r"\{(?P<class>\w+)\}\w*((?P<title>\w[\w\s]*))?").unwrap();
                }
                
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
                            htmler.start(&"div", &[("class", Some(&class_attr))])?;
                            htmler.enter_inline()?;
                            htmler.start(&"h3", NO_ATTRS)?;
                            htmler.write_text(title)?;
                            htmler.end(&"h3")?;
                            htmler.exit_inline()?;
                            htmler.enter_inline()?;
                            htmler.start(&"p", NO_ATTRS)?;
                            htmler.write_text(value)?;
                            htmler.end(&"p")?;
                            htmler.exit_inline()?;
                            htmler.end(&"div")?;
                            return Ok(());
                        }



                        attrs.push(("data-lang", Some(info)));
                        let lang_name = self.rename_langs.get(info).map(String::as_str).unwrap_or(&info).to_string();
                        justlogfox::log_debug!("try highlight language: {} ", lang_name);
                        self.wait_for_starry();

                        let nodein = &mut self.hl_node_proc.in_;
                        let nodeout = &mut self.hl_node_proc.out;
                        writeln!(nodein, "{}", lang_name)?;
                        for line in value.lines() {
                            writeln!(nodein, "\t{}", line)?;
                        }
                        writeln!(nodein, "")?;
                        let mut hl = String::new();
                        loop {
                            let mut buf = [0u8; 1024];
                            let mut n = nodeout.take(1024).read(&mut buf)?;
                            justlogfox::log_trace!("read {} bytes from node", n);
                            if n == 0 { break; }
                            let mut end_text = false;
                            if n >= 2 && buf[n-2] == 0x04 { // EOT
                                end_text = true;
                                n -= 2;
                            } 
                            hl.push_str(std::str::from_utf8(&buf[..n]).unwrap());
                            if end_text { break; }
                        }
                        hl
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
                self.simple_tag("ul", children, tags, htmler)?;
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

    pub fn render(&mut self, md_source: &str, out: Box<dyn std::io::Write>) -> Result<DocumentMetaData, Box<dyn Error>> {
        justlogfox::log_trace!("rendering markdown source, {} bytes", (md_source.len()));

        let mut options = markdown::Options::gfm();
        options.parse.constructs.frontmatter = true;
        let ast = markdown::to_mdast(md_source, &options.parse).unwrap();

        let mut html_writer = html::HTMLWriter {
            is_inline: false,
            close_all_tags: self.close_all_tags,
            indent: 4,
            indent_level: 0,
            writer: out,
            section: None
        };

        let meta = self.render_root(ast, &mut html_writer);

        Ok(meta)
    }
}
