pub struct HTMLWriter {
    pub is_inline: bool,
    pub indent: usize,
    pub indent_level: usize,
    pub close_all_tags: bool,
    pub writer: Box<dyn std::io::Write>,
}

impl HTMLWriter {
    fn write_indent(&mut self) -> std::io::Result<()> {
        for _ in 0..self.indent_level {
            for _ in 0..self.indent {
                write!(self.writer, " ")?;
            }
        }
        Ok(())
    }

    fn write_tag(&mut self, before: &str, tag: &str, attrs: &[(&str, Option<&str>)], after: &str) -> std::io::Result<()> {
        if !self.is_inline {
            writeln!(self.writer, "")?;
            self.write_indent()?;
        }
        write!(self.writer, "{}{tag}", before)?;
        for (k, v) in attrs {
            let k = html_escape::encode_text_minimal(k);
            if let Some(v) = v {
                write!(self.writer, " {k}=\"{}\"", html_escape::encode_quoted_attribute(v))?;
            } else {
                write!(self.writer, " {k}")?;
            }
        }
        write!(self.writer, "{}", after)?;
        Ok(())
    }

    pub fn start(&mut self, tag: impl AsRef<str>, attrs: &[(&str, Option<&str>)]) -> std::io::Result<()> {
        // if !self.is_inline && self.indent_level == 0 {
        //     writeln!(self.writer, "")?;
        // }
        self.write_tag("<", tag.as_ref(), attrs, ">")?;
        if !self.is_inline {
            self.indent_level += 1;
        }
        Ok(())
    }

    pub fn self_close_tag(&mut self, tag: impl AsRef<str>, attrs: &[(&str, Option<&str>)]) -> std::io::Result<()> {
        self.write_tag("<", tag.as_ref(), attrs, " />\n")
    }

    pub fn end(&mut self, tag: impl AsRef<str>) -> std::io::Result<()> {
        if !self.is_inline && self.indent_level > 0 {
            self.indent_level -= 1;
        }
        let tag = tag.as_ref();
        if self.close_all_tags || !["p", "li"].contains(&tag) {
            self.write_tag("</", tag, &[], ">")?;
        }
        if !self.is_inline && self.indent_level == 0 {
            writeln!(self.writer, "")?;
        }
        Ok(())
    }

    pub fn end_implicit(&mut self) {
        if !self.is_inline && self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    pub fn enter_inline(&mut self) -> std::io::Result<()> {
        self.is_inline = true;
        writeln!(self.writer, "")?;
        self.write_indent()
    }

    pub fn enter_inline_s(&mut self) -> std::io::Result<()> {
        self.is_inline = true;
        Ok(())
    }

    pub fn exit_inline(&mut self) -> std::io::Result<()> {
        self.is_inline = false;
        if self.indent_level == 0 {
            writeln!(self.writer, "")?;
        }
        Ok(())
    }

    pub fn write_html(&mut self, raw: impl AsRef<str>) -> std::io::Result<()> {
        write!(self.writer, "{}", raw.as_ref())
    }

    pub fn write_text(&mut self, text: impl AsRef<str>) -> std::io::Result<()> {
        write!(self.writer, "{}", html_escape::encode_text(text.as_ref()))
    }

    pub fn push_elem(&mut self, tags: &[&str], text: impl AsRef<str>) -> std::io::Result<()> {
        self.enter_inline()?;
        for tag in tags {
            self.start(tag, &[])?;
        }
        self.write_text(text)?;
        for tag in tags {
            self.end(tag)?;
        }
        self.exit_inline()
    }
    
    // pub fn start_section(&mut self, tag: & impl AsRef<str>) -> std::io::Result<()> {
    //     println!("!start section");
    //     self.start_tag(self.writer, tag, &[], true)?;
    //     self.indent_level += 1;
    //     self.inside_section = true;
    //     Ok(())
    // }

    // pub fn end_section(&mut self, tag: & impl AsRef<str>) -> std::io::Result<()> {
    //     println!("!end section");
    //     self.indent_level = self.indent_level.saturating_sub(1);
    //     self.end_tag(self.writer, tag)?;
    //     self.inside_section = false;
    //     Ok(())
    // }
}