//! A small XML tree reader for XMP packets: elements, attributes, text, entities, comments,
//! CDATA, self-closing tags, and processing instructions.

use anyhow::{bail, ensure, Context, Result};

/// Maximum nesting depth for elements and for `collect_text` recursion.
const MAX_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Element(Element),
    Text(String),
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn children_named<'a>(&'a self, name: &str) -> impl Iterator<Item = &'a Element> + 'a {
        let name = name.to_owned();
        self.children.iter().filter_map(move |n| match n {
            Node::Element(e) if e.name == name => Some(e),
            _ => None,
        })
    }

    pub fn child_named(&self, name: &str) -> Option<&Element> {
        self.children.iter().find_map(|n| match n {
            Node::Element(e) if e.name == name => Some(e),
            _ => None,
        })
    }

    pub fn text_content(&self) -> String {
        let mut out = String::new();
        let _ = collect_text(self, &mut out, 0);
        out
    }

    pub fn walk<'a>(&'a self) -> Walk<'a> {
        Walk {
            stack: vec![self.children.iter()],
        }
    }
}

pub struct Walk<'a> {
    stack: Vec<std::slice::Iter<'a, Node>>,
}

impl<'a> Iterator for Walk<'a> {
    type Item = &'a Element;
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(iter) = self.stack.last_mut() {
            match iter.next() {
                Some(Node::Element(e)) => {
                    self.stack.push(e.children.iter());
                    return Some(e);
                }
                Some(Node::Text(_)) => {}
                None => {
                    self.stack.pop();
                }
            }
        }
        None
    }
}

fn collect_text(el: &Element, out: &mut String, depth: usize) -> Result<()> {
    ensure!(depth < MAX_DEPTH, "XML nesting exceeds {MAX_DEPTH} levels");
    for child in &el.children {
        match child {
            Node::Text(t) => out.push_str(t),
            Node::Element(e) => collect_text(e, out, depth + 1)?,
        }
    }
    Ok(())
}

/// Parses one root element from `text`, skipping a leading BOM, XML declaration, and
/// processing instructions / comments before the root.
pub fn parse_document(text: &str) -> Result<Element> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut p = Parser {
        bytes: text.as_bytes(),
        pos: 0,
    };
    p.skip_prolog()?;
    p.parse_element(0)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn remaining(&self) -> &'a [u8] {
        &self.bytes[self.pos..]
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Result<u8> {
        let b = self.peek().context("Unexpected end of XML")?;
        self.pos += 1;
        Ok(b)
    }

    fn starts_with(&self, s: &[u8]) -> bool {
        self.remaining().starts_with(s)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn skip_prolog(&mut self) -> Result<()> {
        loop {
            self.skip_ws();
            if self.starts_with(b"<?") {
                self.skip_pi()?;
            } else if self.starts_with(b"<!--") {
                self.skip_comment()?;
            } else if self.starts_with(b"<!DOCTYPE") || self.starts_with(b"<!doctype") {
                self.skip_until(b">")?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn skip_pi(&mut self) -> Result<()> {
        ensure!(self.starts_with(b"<?"), "Expected processing instruction");
        self.pos += 2;
        while !self.starts_with(b"?>") {
            ensure!(
                self.pos < self.bytes.len(),
                "Unclosed processing instruction"
            );
            self.pos += 1;
        }
        self.pos += 2;
        Ok(())
    }

    fn skip_comment(&mut self) -> Result<()> {
        ensure!(self.starts_with(b"<!--"), "Expected comment");
        self.pos += 4;
        while !self.starts_with(b"-->") {
            ensure!(self.pos < self.bytes.len(), "Unclosed comment");
            self.pos += 1;
        }
        self.pos += 3;
        Ok(())
    }

    fn skip_cdata(&mut self) -> Result<String> {
        ensure!(self.starts_with(b"<![CDATA["), "Expected CDATA");
        self.pos += 9;
        let start = self.pos;
        while !self.starts_with(b"]]>") {
            ensure!(self.pos < self.bytes.len(), "Unclosed CDATA");
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .context("CDATA is not UTF-8")?
            .to_owned();
        self.pos += 3;
        Ok(text)
    }

    fn skip_until(&mut self, end: &[u8]) -> Result<()> {
        while !self.starts_with(end) {
            ensure!(self.pos < self.bytes.len(), "Unclosed XML markup");
            self.pos += 1;
        }
        self.pos += end.len();
        Ok(())
    }

    fn parse_name(&mut self) -> Result<String> {
        let start = self.pos;
        let first = self.bump()?;
        ensure!(
            first.is_ascii_alphabetic() || first == b'_' || first == b':',
            "Expected a name"
        );
        while matches!(
            self.peek(),
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b':' | b'-' | b'.')
        ) {
            self.pos += 1;
        }
        Ok(std::str::from_utf8(&self.bytes[start..self.pos])
            .unwrap()
            .to_owned())
    }

    fn parse_attrs(&mut self) -> Result<(Vec<(String, String)>, bool)> {
        let mut attrs = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'/') => {
                    self.pos += 1;
                    self.skip_ws();
                    ensure!(self.bump()? == b'>', "Expected />");
                    return Ok((attrs, true));
                }
                Some(b'>') => {
                    self.pos += 1;
                    return Ok((attrs, false));
                }
                Some(_) => {
                    let name = self.parse_name()?;
                    self.skip_ws();
                    ensure!(self.bump()? == b'=', "Expected = after attribute name");
                    self.skip_ws();
                    let quote = self.bump()?;
                    ensure!(
                        quote == b'"' || quote == b'\'',
                        "Expected a quoted attribute"
                    );
                    let start = self.pos;
                    while self.peek() != Some(quote) {
                        ensure!(self.pos < self.bytes.len(), "Unclosed attribute value");
                        self.pos += 1;
                    }
                    let raw = std::str::from_utf8(&self.bytes[start..self.pos])
                        .context("Attribute value is not UTF-8")?;
                    self.pos += 1;
                    attrs.push((name, decode_entities(raw)?));
                }
                None => bail!("Unexpected end of XML in attributes"),
            }
        }
    }

    fn parse_element(&mut self, depth: usize) -> Result<Element> {
        ensure!(depth < MAX_DEPTH, "XML nesting exceeds {MAX_DEPTH} levels");
        self.skip_ws();
        ensure!(self.bump()? == b'<', "Expected '<'");
        ensure!(
            !matches!(self.peek(), Some(b'/' | b'!' | b'?')),
            "Expected an element"
        );
        let name = self.parse_name()?;
        let (attrs, empty) = self.parse_attrs()?;
        if empty {
            return Ok(Element {
                name,
                attrs,
                children: Vec::new(),
            });
        }
        let mut children = Vec::new();
        loop {
            self.skip_ws();
            if self.starts_with(b"</") {
                let saved = self.pos;
                self.pos += 2;
                self.skip_ws();
                let end_name = self.parse_name()?;
                self.skip_ws();
                if end_name == name && self.peek() == Some(b'>') {
                    self.pos += 1;
                    break;
                }
                self.pos = saved;
                bail!("Unexpected closing tag inside <{name}>");
            }
            if self.starts_with(b"<!--") {
                self.skip_comment()?;
                continue;
            }
            if self.starts_with(b"<?") {
                self.skip_pi()?;
                continue;
            }
            if self.starts_with(b"<![CDATA[") {
                children.push(Node::Text(self.skip_cdata()?));
                continue;
            }
            if self.starts_with(b"<") {
                children.push(Node::Element(self.parse_element(depth + 1)?));
                continue;
            }
            // Text until the next '<'.
            let start = self.pos;
            while !matches!(self.peek(), Some(b'<') | None) {
                self.pos += 1;
            }
            let raw = std::str::from_utf8(&self.bytes[start..self.pos])
                .context("Text node is not UTF-8")?;
            if !raw.is_empty() {
                children.push(Node::Text(decode_entities(raw)?));
            }
            if self.peek().is_none() {
                bail!("Unclosed </{name}>");
            }
        }
        Ok(Element {
            name,
            attrs,
            children,
        })
    }
}

pub fn decode_entities(text: &str) -> Result<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest.find(';').context("Unclosed XML entity")?;
        let entity = &rest[1..end];
        let ch = match entity {
            "lt" => '<',
            "gt" => '>',
            "amp" => '&',
            "quot" => '"',
            "apos" => '\'',
            _ if entity.starts_with('#') => {
                let num = &entity[1..];
                let code = if let Some(hex) = num.strip_prefix(['x', 'X']) {
                    u32::from_str_radix(hex, 16)
                } else {
                    num.parse::<u32>()
                }
                .with_context(|| format!("Bad numeric entity &{entity};"))?;
                char::from_u32(code).context("Invalid character entity")?
            }
            _ => bail!("Unsupported XML entity &{entity};"),
        };
        out.push(ch);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_tree_with_entities_cdata_comment_and_pi() {
        let xml = r#"<?xml version="1.0"?>
<!-- note -->
<root a="1" b="&lt;2&gt;">
  text <![CDATA[raw <&>]]>
  <child/>
</root>"#;
        let root = parse_document(xml).unwrap();
        assert_eq!(root.name, "root");
        assert_eq!(root.attr("a"), Some("1"));
        assert_eq!(root.attr("b"), Some("<2>"));
        assert!(root.text_content().contains("raw <&>"));
        assert!(root.child_named("child").is_some());
    }

    #[test]
    fn accepts_whitespace_before_closing_angle() {
        let root = parse_document(r#"<root><rdf:li >hi</rdf:li ></root>"#).unwrap();
        let li = root.child_named("rdf:li").unwrap();
        assert_eq!(li.text_content(), "hi");
    }

    #[test]
    fn rejects_nesting_deeper_than_64() {
        let mut xml = String::new();
        for i in 0..70 {
            xml.push_str(&format!("<e{i}>"));
        }
        xml.push('x');
        for i in (0..70).rev() {
            xml.push_str(&format!("</e{i}>"));
        }
        let err = parse_document(&xml).unwrap_err().to_string();
        assert!(err.contains("64"), "{err}");
    }
}
