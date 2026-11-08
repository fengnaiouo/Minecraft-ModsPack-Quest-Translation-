//! A small lossless SNBT (stringified NBT) parser.
//!
//! FTB Quests stores chapters as SNBT. We do not need to re-serialize the whole
//! document: every string keeps the byte span of its literal in the original
//! text, so translations can be spliced in without touching formatting.

use anyhow::{bail, Result};
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct Node {
    pub kind: Kind,
    pub span: Range<usize>,
    /// Only meaningful for `Kind::Str`: whether the literal was quoted.
    pub quoted: bool,
    /// Quote character used by the original literal (`"` or `'`).
    pub quote: u8,
}

#[derive(Debug, Clone)]
pub enum Kind {
    Compound(Vec<(String, Node)>),
    List(Vec<Node>),
    Str(String),
    Literal(String),
}

impl Node {
    pub fn get(&self, key: &str) -> Option<&Node> {
        match &self.kind {
            Kind::Compound(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn entries(&self) -> &[(String, Node)] {
        match &self.kind {
            Kind::Compound(entries) => entries,
            _ => &[],
        }
    }

    pub fn items(&self) -> &[Node] {
        match &self.kind {
            Kind::List(items) => items,
            _ => &[],
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match &self.kind {
            Kind::Str(s) => Some(s),
            Kind::Literal(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self.kind, Kind::Str(_))
    }

    pub fn quote_char(&self) -> char {
        if self.quote == b'\'' {
            '\''
        } else {
            '"'
        }
    }
}

pub fn parse(input: &str) -> Result<Node> {
    let mut parser = Parser {
        input,
        src: input.as_bytes(),
        pos: 0,
    };
    parser.skip_trivia();
    let node = parser.value()?;
    parser.skip_trivia();
    Ok(node)
}

struct Parser<'a> {
    input: &'a str,
    src: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn skip_trivia(&mut self) {
        loop {
            while let Some(c) = self.peek() {
                if c.is_ascii_whitespace() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            // support `//` line comments and `#` line comments
            if self.src.get(self.pos..self.pos + 2) == Some(b"//") {
                while let Some(c) = self.peek() {
                    self.pos += 1;
                    if c == b'\n' {
                        break;
                    }
                }
                continue;
            }
            if self.peek() == Some(b'#') {
                while let Some(c) = self.peek() {
                    self.pos += 1;
                    if c == b'\n' {
                        break;
                    }
                }
                continue;
            }
            break;
        }
    }

    fn value(&mut self) -> Result<Node> {
        self.skip_trivia();
        let start = self.pos;
        match self.peek() {
            None => bail!("unexpected end of input"),
            Some(b'{') => self.compound(start),
            Some(b'[') => self.list(start),
            Some(b'"') | Some(b'\'') => self.quoted(start),
            Some(_) => self.bare(start),
        }
    }

    fn compound(&mut self, start: usize) -> Result<Node> {
        self.pos += 1; // {
        let mut entries: Vec<(String, Node)> = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                None => bail!("unterminated compound at byte {start}"),
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                Some(b',') => {
                    self.pos += 1;
                    continue;
                }
                Some(_) => {}
            }
            let key = self.key()?;
            self.skip_trivia();
            if self.peek() != Some(b':') {
                bail!("expected ':' after key {key:?} at byte {}", self.pos);
            }
            self.pos += 1;
            let value = self.value()?;
            entries.push((key, value));
        }
        Ok(Node {
            kind: Kind::Compound(entries),
            span: start..self.pos,
            quoted: false,
            quote: b'"',
        })
    }

    fn list(&mut self, start: usize) -> Result<Node> {
        self.pos += 1; // [
        let mut items = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                None => bail!("unterminated list at byte {start}"),
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                Some(b',') => {
                    self.pos += 1;
                    continue;
                }
                Some(_) => items.push(self.value()?),
            }
        }
        Ok(Node {
            kind: Kind::List(items),
            span: start..self.pos,
            quoted: false,
            quote: b'"',
        })
    }

    fn key(&mut self) -> Result<String> {
        match self.peek() {
            Some(b'"') | Some(b'\'') => {
                let node = self.quoted(self.pos)?;
                match node.kind {
                    Kind::Str(s) => Ok(s),
                    _ => unreachable!(),
                }
            }
            Some(_) => {
                let start = self.pos;
                while let Some(c) = self.peek() {
                    if c == b':' || c.is_ascii_whitespace() || c == b'{' || c == b'[' {
                        break;
                    }
                    self.pos += 1;
                }
                Ok(self.input[start..self.pos].to_string())
            }
            None => bail!("expected key, found end of input"),
        }
    }

    fn quoted(&mut self, start: usize) -> Result<Node> {
        let quote = self.src[self.pos];
        self.pos += 1;
        let mut out = String::new();
        loop {
            let Some(c) = self.peek() else {
                bail!("unterminated string at byte {start}");
            };
            if c == quote {
                self.pos += 1;
                break;
            }
            if c == b'\\' {
                self.pos += 1;
                let Some(esc) = self.peek() else {
                    bail!("unterminated escape at byte {}", self.pos);
                };
                self.pos += 1;
                match esc {
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    b'r' => out.push('\r'),
                    b'\\' => out.push('\\'),
                    b'"' => out.push('"'),
                    b'\'' => out.push('\''),
                    other => {
                        // Unknown escape: keep it verbatim (e.g. \uXXXX is rare in FTB data).
                        out.push('\\');
                        out.push(other as char);
                    }
                }
                continue;
            }
            let ch = self.input[self.pos..]
                .chars()
                .next()
                .expect("valid utf-8 boundary");
            out.push(ch);
            self.pos += ch.len_utf8();
        }
        Ok(Node {
            kind: Kind::Str(out),
            span: start..self.pos,
            quoted: true,
            quote,
        })
    }

    fn bare(&mut self, start: usize) -> Result<Node> {
        while let Some(c) = self.peek() {
            if c == b',' || c == b']' || c == b'}' || c.is_ascii_whitespace() {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            bail!("expected value at byte {start}");
        }
        let raw = &self.input[start..self.pos];
        let kind = if raw == "true" || raw == "false" || looks_numeric(raw) {
            Kind::Literal(raw.to_string())
        } else {
            Kind::Str(raw.to_string())
        };
        Ok(Node {
            kind,
            span: start..self.pos,
            quoted: false,
            quote: b'"',
        })
    }
}

fn looks_numeric(raw: &str) -> bool {
    let trimmed = raw.trim_end_matches(['b', 'B', 's', 'S', 'l', 'L', 'f', 'F', 'd', 'D']);
    if trimmed.is_empty() {
        return false;
    }
    if let Some(hex) = trimmed.strip_prefix("0x") {
        return !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    trimmed.parse::<f64>().is_ok()
}

/// Encode a plain text value as an SNBT string literal.
pub fn encode_string(text: &str, quoted: bool, quote: char) -> String {
    let needs_quote = quoted || !is_safe_bare(text);
    if !needs_quote {
        return text.to_string();
    }
    let quote = if quote == '\'' { '\'' } else { '"' };
    let mut out = String::with_capacity(text.len() + 2);
    out.push(quote);
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

fn is_safe_bare(text: &str) -> bool {
    !text.is_empty()
        && !looks_numeric(text)
        && text != "true"
        && text != "false"
        && !text.chars().any(|c| {
            c.is_whitespace() || matches!(c, ',' | ']' | '}' | '{' | '[' | ':' | '"' | '\'' | '\\')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
    filename: "welcome"
    quests: [
        {
            description: ["hello world", "second"]
            id: "ABC"
            tasks: [{ title: "&6Welcome&r" type: "checkmark" }]
            x: 1.5d
        }
    ]
}"#;

    #[test]
    fn parses_ftb_style_snbt() {
        let root = parse(SAMPLE).unwrap();
        assert_eq!(root.get("filename").unwrap().as_str(), Some("welcome"));
        let quests = root.get("quests").unwrap().items();
        assert_eq!(quests.len(), 1);
        let desc = quests[0].get("description").unwrap().items();
        assert_eq!(desc[1].as_str(), Some("second"));
        let raw = &SAMPLE[desc[0].span.clone()];
        assert_eq!(raw, "\"hello world\"");
    }

    #[test]
    fn encodes_strings() {
        assert_eq!(encode_string("hello", true, '"'), "\"hello\"");
        assert_eq!(encode_string("a\nb", true, '"'), "\"a\\nb\"");
        assert_eq!(encode_string("a:b", false, '"'), "\"a:b\"");
    }
}
