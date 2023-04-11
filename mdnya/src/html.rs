pub struct HTMLWriter<'a> {
    pub is_inline: bool,
    pub indent: usize,
    pub indent_level: usize,
    pub close_all_tags: bool,
    pub writer: Box<dyn std::io::Write + 'a>,
    pub section: Option<String>,
    pub is_first_tag: bool
}

pub const NO_ATTRS : &[(&str, Option<&str>)] = &[];

pub trait Attributes {
    fn write_attrs(&self, writer: &mut impl std::io::Write) -> std::io::Result<()>;
}

impl<K, V> Attributes for &[(K, Option<V>)]
    where K: AsRef<str>,
          V: AsRef<str> {
    fn write_attrs(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        for (k, v) in self.iter() {
            let k = html_escape::encode_text_minimal(k.as_ref());
            if let Some(v) = v {
                write!(writer, " {k}=\"{}\"", html_escape::encode_quoted_attribute(v.as_ref()))?;
            } else {
                write!(writer, " {k}")?;
            }
        }
        Ok(())
    }
}

impl<K, V> Attributes for &Vec<(K, Option<V>)>
    where K: AsRef<str>,
          V: AsRef<str> {
    fn write_attrs(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.as_slice().write_attrs(writer)
    }
}

impl<K, V, const N: usize> Attributes for &[(K, Option<V>); N]
    where K: AsRef<str>,
          V: AsRef<str> {
    fn write_attrs(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.as_slice().write_attrs(writer)
    }
}

impl<'a> HTMLWriter<'a> {

    pub fn new(writer: Box<dyn std::io::Write + 'a>, indent: usize, close_all_tags: bool) -> Self {
        Self {
            is_inline: false,
            indent,
            indent_level: 0,
            close_all_tags,
            writer,
            section: None,
            is_first_tag: true
        }
    }

    fn write_indent(&mut self) -> std::io::Result<()> {
        write!(self.writer, "{:amount$}", "", amount = self.indent_level*self.indent)
    }

    fn write_tag(&mut self, before: &str, tag: &str, attrs: impl Attributes, after: &str) -> std::io::Result<()> {
        if !self.is_inline && !self.is_first_tag {
            writeln!(self.writer)?;
            self.write_indent()?;
            
        }

        self.is_first_tag = false;

        write!(self.writer, "{}{tag}", before)?;
        attrs.write_attrs(&mut self.writer)?;
        write!(self.writer, "{}", after)?;
        Ok(())
    }

    pub fn start(&mut self, tag: impl AsRef<str>, attrs: impl Attributes) -> std::io::Result<()> {
        // if !self.is_inline && self.indent_level == 0 {
        //     writeln!(self.writer, "")?;
        // }
        self.write_tag("<", tag.as_ref(), attrs, ">")?;
        if !self.is_inline {
            self.indent_level += 1;
        }
        Ok(())
    }

    pub fn void_tag(&mut self, tag: impl AsRef<str>, attrs: impl Attributes, not_inline: bool) -> std::io::Result<()> {
        let inline_before = self.is_inline;
        if not_inline {
            self.is_inline = false;
        }
        self.write_tag("<", tag.as_ref(), attrs, " />")?;
        if !self.is_inline {
            writeln!(self.writer)?;
        }
        self.is_inline = inline_before;
        Ok(())
    }

    pub fn end(&mut self, tag: impl AsRef<str>) -> std::io::Result<()> {
        if !self.is_inline && self.indent_level > 0 {
            self.indent_level -= 1;
        }
        let tag = tag.as_ref();
        if self.close_all_tags || !["p", "li", "br"].contains(&tag) {
            self.write_tag("</", tag, NO_ATTRS, ">")?;
        }
        if !self.is_inline && self.indent_level == 0 {
            writeln!(self.writer)?;
        }
        Ok(())
    }

    pub fn enter_inline(&mut self) -> std::io::Result<()> {
        self.is_inline = true;
        writeln!(self.writer)?;
        self.write_indent()
    }

    pub fn exit_inline(&mut self) -> std::io::Result<()> {
        self.is_inline = false;
        if self.indent_level == 0 {
            writeln!(self.writer)?;
        }
        Ok(())
    }

    pub fn enter_section(&mut self, tag: impl ToString) -> std::io::Result<()> {
        if let Some(tag) = self.section.take() {
            self.end(&tag)?;
        }
        let tag = tag.to_string();
        self.start(&tag, NO_ATTRS)?;
        self.section = Some(tag);
        Ok(())
    }

    pub fn maybe_exit_section(&mut self) -> std::io::Result<()> {
        if let Some(tag) = self.section.take() {
            self.end(&tag)
        } else {
            Ok(())
        }
    }

    pub fn write_html(&mut self, raw: impl AsRef<str>) -> std::io::Result<()> {
        write!(self.writer, "{}", raw.as_ref())
    }

    pub fn write_text(&mut self, text: impl AsRef<str>) -> std::io::Result<()> {
        write!(self.writer, "{}", html_escape::encode_text(text.as_ref()))
    }
}