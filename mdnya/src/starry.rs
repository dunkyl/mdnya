use std::{
    path::PathBuf,
    process::{Child, Stdio}, 
    io::{Read, Write, Result, BufRead, BufReader},
    sync::Mutex
};

const INDEXJS_SRC: &str = include_str!("../../dist/bundle.cjs");

fn ensure_indexjs() -> Result<PathBuf> {
    let indexjs = dirs::data_local_dir().unwrap().join(".mdnya").join("bundle.cjs");
    if !indexjs.exists() {
        std::fs::create_dir_all(indexjs.parent().unwrap())?;
        std::fs::write(&indexjs, INDEXJS_SRC)?;
    }
    Ok(indexjs)
}

pub trait Highlighter {
    fn highlight(&self, lang: &str, code: &str) -> Result<String>;
}

pub struct StarryHighlighter<'a> {
    node: Mutex<Child>,
    init: std::sync::Once,
    rename_langs: std::collections::HashMap<&'a str, &'a str>,

}

impl<'a> StarryHighlighter<'a> {

    fn wait_for_starry(&self) {
        self.init.call_once(|| {
            let start = std::time::Instant::now();
            justlogfox::log_info!("waiting for starry night");
            {
                let mut buf = [0u8; 6];
                let mut node = self.node.lock().unwrap();
                node.stdout.as_mut().unwrap().read_exact(&mut buf).unwrap();
                assert_eq!(&buf, b"ready\n");
            }
            let elapsed = start.elapsed();
            justlogfox::log_info!("starry night loaded :D\ntook: {}ms", (elapsed.as_millis()));
        });
    }

    pub fn new(language_aliases: impl Into<std::collections::HashMap<&'a str, &'a str>>) -> Self {
        let indexjs = ensure_indexjs();
        let Ok(indexjs) = indexjs else {
            justlogfox::log_error!("failed to setup starry night js file");
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
        let mut rename_langs = language_aliases.into();
        rename_langs.insert("md", "markdown");
        rename_langs.insert("sh", "bash");
        Self {
            node: Mutex::new(node),
            rename_langs,
            init: std::sync::Once::new(),
        }
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
    "number",
    "comment",
];

const HL_CLASSES: &[&str] = &[
    "attribute",
    "constant",
    "function-builtin",
    "function",
    "keyword",
    "operator",
    "property",
    "punctuation",
    "punctuation-bracket",
    "punctuation-delimiter",
    "string",
    "string-special",
    "tag",
    "type",
    "type-builtin",
    "variable",
    "variable-builtin",
    "variable-parameter",
    "number",
    "comment",
];

impl<'a> Highlighter for StarryHighlighter<'a> {
    fn highlight(&self, lang: &str, code: &str) -> Result<String> {

        if ["C#", "csharp", "cs"].contains(&lang) {
            use tree_sitter_highlight::*;
            use tree_sitter_c_sharp::*;

            let cs = language();
            let mut cfg = HighlightConfiguration::new(
                        cs, HIGHLIGHT_QUERY, "", "")
                      .expect("upstream query is valid");
            cfg.configure(HL_NAMES);
            let mut hl = Highlighter::new();
            let events = hl.highlight(&cfg, code.as_bytes(), None, |_| None).unwrap();
            let mut render = HtmlRenderer::new();
            render.render(events, code.as_bytes(), &|hl| HL_CLASSES[hl.0].as_bytes()).unwrap();
            return Ok(String::from_utf8(render.html).unwrap());
        }




        justlogfox::log_trace!("try highlight language: {} ", lang);
        self.wait_for_starry();

        let lang = self.rename_langs.get(lang).unwrap_or(&lang);

        let mut node = self.node.lock().unwrap();
        {
            let node_stdin = node.stdin.as_mut().unwrap();

            writeln!(node_stdin, "{}", lang)?; 
            for line in code.lines() { // write code to highlight
                writeln!(node_stdin, "\t{}", line)?;
            }
            writeln!(node_stdin)?;
        }

        let node_stdout = node.stdout.as_mut().unwrap();
        let mut node_stdout_buf = BufReader::new(node_stdout);
        let mut hl = String::new();
        loop { // read back highlighted code
            let mut line = String::new();
            let n = node_stdout_buf.read_line(&mut line)?;
            justlogfox::log_trace!("read {} bytes from node\n{:?}", n, (&line));
            
            if n == 0 || line == "\x04\n" { // EOT or EOF
                break;
            } 
            hl = hl + &line;
        }
        Ok(hl)
    }
}

impl<'a> Drop for StarryHighlighter<'a> {
    fn drop(&mut self) {
        self.node.lock().unwrap().kill().unwrap();
    }
}

