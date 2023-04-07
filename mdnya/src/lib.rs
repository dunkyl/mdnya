use std::{process::{Child, ChildStdin, ChildStdout, Stdio}, io::Write, io::Read, path::PathBuf};

use phf::phf_map;
use regex::Regex;
use lazy_static::lazy_static;
use tree_sitter::TreeCursor;

mod html;

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
    close_all_tags: bool,
    wrap_sections: Option<String>,
    heading_level: u8,
    no_ids: bool,
    no_code_lines: bool,
    hl_node_proc: InOutProc,
    hl_ready: bool,
    razor: bool
}

impl Drop for MDNya {
    fn drop(&mut self) {
        self.hl_node_proc.proc.kill().unwrap();
    }
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
    Custom(fn(&mut MDNya, &mut TreeCursor, &[u8], &mut html::HTMLWriter, &mut MDNyaState) -> MdResult),
    Skip
}
use NodeTransform::*;

fn heading_transform(m: &mut MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    if state.inside_section {
        if let Some(section_tag) = &m.wrap_sections {
            helper.end(section_tag)?;
            state.inside_section = false;
        }
    }
    cur.goto_first_child();
    let level = u8::from_str_radix(&cur.node().kind()[5..6], 10).unwrap();
    let tag = format!("h{}", level+m.heading_level-1);
    cur.goto_next_sibling();
    let heading_content = cur.node().utf8_text(src).unwrap().trim_start();
    let id =
        if m.no_ids {
            None
        } else {
            let id = 
                if heading_content.starts_with('@')  {
                    heading_content.to_string()
                } else {
                    heading_content.to_lowercase().replace(" ", "-").replace("?", "")
                };
            Some(id)
        };
    if cur.node().parent().unwrap().prev_sibling().is_some() {
        helper.write_html("\n")?;
        helper.enter_inline()?;
    } else {
        helper.enter_inline_s()?;
    }
    
    helper.start(&tag, &[("id", id.as_ref().map(|x| &**x))])?;
    m.render_elem(src, cur, helper, state)?;
    helper.end(&tag)?;
    helper.exit_inline()?;
    cur.goto_parent();
    
    if !state.inside_section {
        if let Some(section_tag) = &m.wrap_sections {
            helper.start(section_tag, &[])?;
            state.inside_section = true;
        }
    }
    Ok(())
}

fn link_transform(m: &mut MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    let link_destination = cur.node().child(1).map(|c| c.utf8_text(src).unwrap().into());
    helper.start(&"a", &[("href", link_destination)])?;
    cur.goto_first_child();
    cur.goto_first_child();
    m.render_elem(src, cur, helper, state)?;
    cur.goto_parent();
    cur.goto_parent();
    helper.end(&"a")?;

    // cur.node().child_by_field_name(field_name)
    Ok(())
}

fn image_transform(m: &mut MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
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

fn text_transform(m: &mut MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    let text = cur.node().utf8_text(src).unwrap();
    helper.write_text(text)?;
    Ok(())
}

fn list_transform(m: &mut MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    let node = cur.node();
    let markers = (0..node.child_count()).map(|i| node.child(i).unwrap().child(0).unwrap().utf8_text(src).unwrap()).collect::<Vec<_>>();
    let is_bulleted = markers.iter().all(|&m| m == "-" || m == "*");
    let is_numbered_forward = markers.iter().enumerate().all(|(i, &m)| m == &((i+1).to_string() + "."));
    let is_numbered_backward  = markers.iter().enumerate().all(|(i, &m)| m == &((markers.len() - i).to_string() + "."));

    let (tag, attrs): (_, &[(_, Option<&str>)]) =
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

fn list_item_transform(m: &mut MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    cur.goto_first_child();
    cur.goto_next_sibling(); // skip list marker and p
    m.render_elem_seq(helper, true, &OptionalClose("li"), cur, src, &[], state)?;
    cur.goto_parent();
    Ok(())
}

fn checkbox_transform(m: &mut MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    let node = cur.node();
    let is_checked = node.utf8_text(src).unwrap() == "[x]";
    let mut attrs = vec![
        ("type", Some("checkbox".into())),
        ("disabled", None)
    ];
    if is_checked { attrs.push(("checked", None)); }
    m.render_elem_seq(helper, false, &SelfClose("input"), cur, src, &attrs, state)
}


static RENAME_LANGS: phf::Map<&'static str, &'static str> = phf_map! {
    "md" => "markdown",
    "sh" => "bash",
};

fn codeblock_transform(m: &mut MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
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
            let class = format!("admonition {class}");
            helper.start(&"div", &[("class", Some(&class))])?;
            helper.push_elem(&["h3"], title)?;
            helper.push_elem(&["p"], content)?;
            helper.end(&"div")?;

        } else { // possibly-highlighted code block
            helper.enter_inline()?;
            helper.start(&"pre", &[("data-lang", Some(info.into()))])?;
            helper.start(&"code", &[])?;

            // let highligher = m.try_get_highlighter(info);
            let add_code_lines = 
                if m.no_code_lines {
                    |text: &str| text.trim_end().to_string()
                } else {
                    |text: &str| {
                        text.trim_end().split('\n').map(|line| {
                            format!("<span class=\"code-line\">{line}</span>")
                        }).collect::<Vec<_>>().join("\n")
                    }
                };

            let langname = RENAME_LANGS.get(info).unwrap_or(&info);

            m.wait_for_starry();

            let nodein = &mut m.hl_node_proc.in_;
            let nodeout = &mut m.hl_node_proc.out;
            writeln!(nodein, "{}", langname)?;
            for line in content.lines() {
                writeln!(nodein, "\t{}", line)?;
            }
            writeln!(nodein, "")?;
            let mut hl = String::new();
            loop {
                let mut buf = [0u8; 1024];
                let mut n = nodeout.take(1024).read(&mut buf)?;
                println!("read {} bytes", n);
                if n == 0 { break; }
                let mut end_text = false;
                if n >= 2 && buf[n-2] == 0x04 { // EOT
                    end_text = true;
                    n -= 2;
                } 
                hl.push_str(std::str::from_utf8(&buf[..n]).unwrap());
                if end_text { break; }
            }
            helper.write_html(add_code_lines(&hl))?;
            
            helper.end(&"code")?;
            helper.end(&"pre")?;
            helper.exit_inline()?;
        }
    } else { // no info, plain code block
        let content = cur.node().utf8_text(src).unwrap().trim_end();

        helper.push_elem(&["pre", "code"], content)?;
    }


    cur.goto_parent();
    Ok(())
}

fn table_cell_transform(m: &mut MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
    let node = cur.node();
    let is_header = node.parent().unwrap().kind() == "table_header_row";
    let tag = if is_header { "th" } else { "td" };
    m.render_elem_seq(helper, true, &Full(&tag), cur, src, &[], state)
}

fn slb_transform(m: &mut MDNya, cur: &mut TreeCursor, src: &[u8], helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
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

    "indented_code_block" => Simple {tag: Full("pre"), inline: false, attrs: &[] },

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

    pub fn new(close_all_tags: bool, wrap_sections: Option<String>, heading_level: u8, no_ids: bool,) -> Self {
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
        Self { 
            close_all_tags,
            wrap_sections,
            heading_level,
            no_ids,
            no_code_lines: false,
            hl_node_proc: proc,
            hl_ready: false,
            razor: false
        }
    }

    fn render_elem(&mut self, src: &[u8], cur: &mut TreeCursor, helper: &mut html::HTMLWriter, state: &mut MDNyaState) -> MdResult {
        let node = cur.node();
        let kind = node.kind();
        let behave = MD_TRANSFORMERS.get(kind).unwrap_or_else(|| {
            println!("\n--- s-expr:\n{}\n---\n", node.parent().unwrap_or(node).to_sexp());
            panic!("{}", kind)
        });
        match behave {
            Simple {tag, inline, attrs} => {
                self.render_elem_seq(helper, *inline, tag, cur, src, &attrs, state)?;
            },
            Custom(f) => f(self, cur, src, helper, state)?,
            Skip => ()
        }

        Ok(())
    }

    fn render_elem_seq(&mut self, helper: &mut html::HTMLWriter, inline: bool, tag: &TagBehavior, cur: &mut TreeCursor, src: &[u8], attrs: &[(&str, Option<&str>)], state: &mut MDNyaState) -> MdResult {
        let switched_inline = !helper.is_inline && inline;
        if switched_inline {
            helper.enter_inline()?;
        }
        match tag {
            Full(t) | OptionalClose(t) => helper.start(&t, attrs)?,
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
            Full(t) | OptionalClose(t) => helper.end(&t)?,
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

    fn render_root(&mut self, node: markdown::mdast::Node, htmler: &mut html::HTMLWriter) -> (serde_yaml::Value, Vec<String>) {
        use markdown::mdast::*;
        let Node::Root(Root { children, ..}) = node else {
            panic!("non-root node passed to render_root")
        };
        let mut children_iter = children.into_iter();
        let first_child = children_iter.next();
        let (frontmatter, skip1) =
            if let Some(Node::Yaml(Yaml{value, ..})) = &first_child {
                use serde_yaml::from_str;
                let fm = (from_str(value).unwrap(), true);
                justlogfox::log_debug!("frontmatter: {:?}", (fm.0));
                fm
            } else {
                justlogfox::log_debug!("document has no frontmatter");
                (serde_yaml::Value::Null, false)
            };
        let rest_children = if skip1 {
            (None).into_iter().chain(children_iter)
        } else {
            first_child.into_iter().chain(children_iter)
        };
        let mut tags = vec![];
        let result = self.render_children(rest_children, &mut tags, htmler);
        if let Err(e) = result {
            justlogfox::log_error!("error while rendering: {}", e);
        }
        (frontmatter, tags)
    }

    fn render_table(&mut self, table: markdown::mdast::Table, caption: Option<Vec<markdown::mdast::Node>>, tags: &mut  Vec<String>, htmler: &mut html::HTMLWriter) -> MdResult {
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
        htmler.start("table", &[])?;

        if let Some(caption) = caption {
            self.simple_inline_tag("caption", caption, tags, htmler)?;
        }

        htmler.start("thead", &[])?;

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
        htmler.start("tbody", &[])?;
        for row in rows {
            let Node::TableRow(TableRow { children: cells, .. }) = row
                else { panic!("non-row in table") };
            htmler.start("tr", &[])?;
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

    fn render_children(&mut self, children: impl IntoIterator<Item=markdown::mdast::Node>, tags: &mut  Vec<String>, htmler: &mut html::HTMLWriter) -> MdResult {
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
            self.render_node(node, 0, tags, htmler)?;
        }
        Ok(())
    }

    fn simple_tag(&mut self, tag: &str, children: Vec<markdown::mdast::Node>, tags: &mut  Vec<String>, htmler: &mut html::HTMLWriter) -> MdResult {
        htmler.start(tag, &[])?;
        self.render_children(children, tags, htmler)?;
        htmler.end(tag)?;
        Ok(())
    }

    fn simple_inline_tag(&mut self, tag: &str, children: Vec<markdown::mdast::Node>, tags: &mut  Vec<String>, htmler: &mut html::HTMLWriter) -> MdResult {
        htmler.enter_inline()?;
        self.simple_tag(tag, children, tags, htmler)?;
        htmler.exit_inline()?;
        Ok(())
    }

    fn render_node(&mut self, node: markdown::mdast::Node, depth: usize, tags: &mut  Vec<String>, htmler: &mut html::HTMLWriter) -> MdResult {
        use markdown::mdast::*;
        match node {
            Node::Heading(Heading { children, depth, .. }) => {

                lazy_static! {
                    static ref FRAGMENT_REMOVE_RE: Regex = Regex::new(r"[^a-zA-Z0-9-]").unwrap();
                }

                let tag = format!("h{}", (depth + self.heading_level) as isize - 1);
                let fragment =
                    children.iter().fold(String::new(), |acc, node| acc + node.to_string().as_str())
                    .to_ascii_lowercase().replace(" ", "-");
                let fragment = FRAGMENT_REMOVE_RE.replace_all(&fragment, "");
                justlogfox::log_debug!("heading: {} {}", tag, fragment);
                
                self.simple_inline_tag(&tag, children, tags, htmler)?;
            }
            Node::Text(Text { value, .. }) => {

                lazy_static! {
                    static ref TAG_RE: Regex = Regex::new(r"#[a-zA-Z0-9_-]+").unwrap();
                }

                let new_tags = TAG_RE.find_iter(&value).map(|m| m.as_str()[1..].to_string()).collect::<Vec<_>>();
                for tag in &new_tags {
                    justlogfox::log_debug!("tag: #{}", tag);
                }
                tags.extend(new_tags);

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
                            htmler.start(&"h3", &[])?;
                            htmler.write_text(title)?;
                            htmler.end(&"h3")?;
                            htmler.exit_inline()?;
                            htmler.enter_inline()?;
                            htmler.start(&"p", &[])?;
                            htmler.write_text(value)?;
                            htmler.end(&"p")?;
                            htmler.exit_inline()?;
                            htmler.end(&"div")?;
                            return Ok(());
                        }



                        attrs.push(("data-lang", Some(info)));
                        let lang_name = RENAME_LANGS.get(info).unwrap_or(&info);
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
                htmler.start("pre", &[])?;
                htmler.start("code", &attrs)?;
                htmler.write_html(code)?;
                htmler.end("code")?;
                htmler.end("pre")?;
                htmler.exit_inline()?;
            }
            Node::ThematicBreak(ThematicBreak {..}) => {
                htmler.self_close_tag("hr", &[])?;
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
                htmler.start("li", &[])?;

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
                htmler.start("code", &[])?;
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

    pub fn render(&mut self, md_source: &str, mut out: Box<dyn std::io::Write>) -> MdResult {
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
        };

        let (frontmatter, tags) = self.render_root(ast, &mut html_writer);

        for tag in tags {
            justlogfox::log_trace!("collected tag: #{}", tag);
        }

        Ok(())
    }
}
