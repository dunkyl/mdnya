pub struct HTMLWriter {
    pub is_inline: bool,
    pub indent: String,
    pub indent_level: usize,
    pub close_all_tags: bool,
}

impl HTMLWriter {
    fn write_indent(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        for _ in 0..self.indent_level {
            write!(out, "{}", self.indent)?;
        }
        Ok(())
    }

    fn write_tag(&self, out: &mut impl std::io::Write, before: &str, tag: &str, attrs: &[(&str, Option<String>)], after: &str) -> std::io::Result<()> {
        if !self.is_inline {
            self.write_indent(out)?;
        }
        write!(out, "{}{tag}", before)?;
        for (k, v) in attrs {
            let k = html_escape::encode_text_minimal(k);
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

    pub fn start_tag(&mut self, out: &mut impl std::io::Write, tag: & impl AsRef<str>, attrs: &[(&str, Option<String>)]) -> std::io::Result<()> {
        self.write_tag(out, "<", tag.as_ref(), attrs, ">")?;
        if !self.is_inline {
            self.indent_level += 1;
        }
        Ok(())
    }

    pub fn self_close_tag(&self, out: &mut impl std::io::Write, tag: & impl AsRef<str>, attrs: &[(&str, Option<String>)]) -> std::io::Result<()> {
        self.write_tag(out, "<", tag.as_ref(), attrs, " />")
    }

    pub fn end_tag(&mut self, out: &mut impl std::io::Write, tag: & impl AsRef<str>) -> std::io::Result<()> {
        if !self.is_inline {
            self.indent_level -= 1;
        }
        let tag = tag.as_ref();
        if !self.close_all_tags && ["p", "li"].contains(&tag) {
            Ok(())
        } else {
            self.write_tag(out, "</", tag, &[], ">")
        }
    }

    // pub fn start_section(&mut self, out: &mut impl std::io::Write, tag: & impl AsRef<str>) -> std::io::Result<()> {
    //     println!("!start section");
    //     self.start_tag(out, tag, &[], true)?;
    //     self.indent_level += 1;
    //     self.inside_section = true;
    //     Ok(())
    // }

    // pub fn end_section(&mut self, out: &mut impl std::io::Write, tag: & impl AsRef<str>) -> std::io::Result<()> {
    //     println!("!end section");
    //     self.indent_level = self.indent_level.saturating_sub(1);
    //     self.end_tag(out, tag)?;
    //     self.inside_section = false;
    //     Ok(())
    // }
}