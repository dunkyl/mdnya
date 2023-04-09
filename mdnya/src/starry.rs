use std::{
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Stdio}, 
    io::{Read, Write, Result, BufRead, BufReader}
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
    fn highlight(&mut self, lang: &str, code: &str) -> Result<String>;
}

pub struct StarryHighlighter<'a> {
    node: Child,
    node_stdin: ChildStdin,
    node_stdout: BufReader<ChildStdout>,
    init: std::sync::Once,
    rename_langs: std::collections::HashMap<&'a str, &'a str>,

}

impl<'a> StarryHighlighter<'a> {

    fn wait_for_starry(&mut self) {
        self.init.call_once(|| {
            let start = std::time::Instant::now();
            justlogfox::log_info!("waiting for starry night");
            {
                let mut buf = [0u8; 6];
                self.node_stdout.read_exact(&mut buf).unwrap();
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
        let mut node = std::process::Command::new("node")
            .arg(indexjs)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn().expect("node not found");
        let node_stdin = node.stdin.take().unwrap();
        let node_stdout = BufReader::new(node.stdout.take().unwrap());
        let mut rename_langs = language_aliases.into();
        rename_langs.insert("md", "markdown");
        rename_langs.insert("sh", "bash");
        Self {
            node,
            node_stdin,
            node_stdout,
            rename_langs,
            init: std::sync::Once::new(),
        }
    }

}

impl<'a> Highlighter for StarryHighlighter<'a> {
    fn highlight(&mut self, lang: &str, code: &str) -> Result<String> {
        justlogfox::log_trace!("try highlight language: {} ", lang);
        self.wait_for_starry();

        let lang = self.rename_langs.get(lang).unwrap_or(&lang);

        writeln!(self.node_stdin, "{}", lang)?; 
        for line in code.lines() { // write code to highlight
            writeln!(self.node_stdin, "\t{}", line)?;
        }
        writeln!(self.node_stdin)?;

        let mut hl = String::new();
        loop { // read back highlighted code
            let mut line = String::new();
            let n = self.node_stdout.read_line(&mut line)?;
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
        self.node.kill().unwrap();
    }
}

