use crate::scan::is_translatable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerString {
    pub start: usize,
    pub end: usize,
    pub quote: char,
    pub text: String,
}

const PLAYER_TEXT_CALLS: &[&str] = &[
    "Text.of",
    "Text.literal",
    "Component.literal",
    "tell",
    "sendSystemMessage",
    "displayClientMessage",
    "sendMessage",
    "actionBar",
];

pub fn scan_player_strings(source: &str) -> Vec<PlayerString> {
    let bytes = source.as_bytes();
    let mut strings = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index = skip_line_comment(bytes, index + 2);
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index = skip_block_comment(bytes, index + 2);
            }
            quote @ (b'\'' | b'"' | b'`') => {
                let literal_start = index;
                let content_start = index + 1;
                let Some(content_end) = find_string_end(bytes, content_start, quote) else {
                    break;
                };
                let after_literal = content_end + 1;
                if is_player_text_argument(source, literal_start, after_literal)
                    && source.is_char_boundary(content_start)
                    && source.is_char_boundary(content_end)
                {
                    let text = decode_js_string(&source[content_start..content_end], quote);
                    if is_translatable(&text) {
                        strings.push(PlayerString {
                            start: content_start,
                            end: content_end,
                            quote: quote as char,
                            text,
                        });
                    }
                }
                index = after_literal;
            }
            _ => index += 1,
        }
    }

    strings
}

pub fn encode_js_string(text: &str, quote: char) -> String {
    let mut encoded = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if quote == '`' && ch == '\\' && chars.peek() == Some(&'$') {
            let mut lookahead = chars.clone();
            lookahead.next();
            if lookahead.next() == Some('{') {
                encoded.push_str("\\${");
                chars.next();
                chars.next();
                continue;
            }
        }
        match ch {
            '\\' => encoded.push_str("\\\\"),
            '\n' if quote != '`' => encoded.push_str("\\n"),
            '\r' if quote != '`' => encoded.push_str("\\r"),
            '\t' => encoded.push_str("\\t"),
            ch if ch == quote => {
                encoded.push('\\');
                encoded.push(ch);
            }
            _ => encoded.push(ch),
        }
    }
    encoded
}

fn skip_line_comment(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index] != b'\n' {
        index += 1;
    }
    index
}

fn skip_block_comment(bytes: &[u8], mut index: usize) -> usize {
    while index + 1 < bytes.len() {
        if bytes[index] == b'*' && bytes[index + 1] == b'/' {
            return index + 2;
        }
        index += 1;
    }
    bytes.len()
}

fn find_string_end(bytes: &[u8], mut index: usize, quote: u8) -> Option<usize> {
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = (index + 2).min(bytes.len()),
            b'\n' | b'\r' if quote != b'`' => return None,
            ch if ch == quote => return Some(index),
            _ => index += 1,
        }
    }
    None
}

fn is_player_text_argument(source: &str, literal_start: usize, after_literal: usize) -> bool {
    let bytes = source.as_bytes();
    let mut open = literal_start;
    while open > 0 && bytes[open - 1].is_ascii_whitespace() {
        open -= 1;
    }
    if open == 0 || bytes[open - 1] != b'(' {
        return false;
    }

    let mut close = after_literal;
    while close < bytes.len() && bytes[close].is_ascii_whitespace() {
        close += 1;
    }
    if close < bytes.len() && bytes[close] != b')' && bytes[close] != b',' {
        return false;
    }

    let mut name_end = open - 1;
    while name_end > 0 && bytes[name_end - 1].is_ascii_whitespace() {
        name_end -= 1;
    }
    let mut name_start = name_end;
    while name_start > 0
        && (bytes[name_start - 1].is_ascii_alphanumeric() || bytes[name_start - 1] == b'_')
    {
        name_start -= 1;
    }
    if name_start == name_end {
        return false;
    }

    let function = &source[name_start..name_end];
    if function == "of" || function == "literal" {
        let mut receiver_end = name_start;
        while receiver_end > 0 && bytes[receiver_end - 1].is_ascii_whitespace() {
            receiver_end -= 1;
        }
        if receiver_end == 0 || bytes[receiver_end - 1] != b'.' {
            return false;
        }
        let mut receiver_start = receiver_end - 1;
        while receiver_start > 0
            && (bytes[receiver_start - 1].is_ascii_alphanumeric()
                || bytes[receiver_start - 1] == b'_'
                || bytes[receiver_start - 1] == b'.')
        {
            receiver_start -= 1;
        }
        let receiver = &source[receiver_start..receiver_end - 1];
        return matches!(
            (receiver, function),
            ("Text", "of" | "literal") | ("Component", "literal")
        );
    }

    if !PLAYER_TEXT_CALLS.contains(&function) {
        return false;
    }
    let mut receiver_end = name_start;
    while receiver_end > 0 && bytes[receiver_end - 1].is_ascii_whitespace() {
        receiver_end -= 1;
    }
    if receiver_end == 0 || bytes[receiver_end - 1] != b'.' {
        return false;
    }
    if function == "actionBar" {
        return source[..receiver_end].contains("player");
    }
    true
}

fn decode_js_string(raw: &str, quote: u8) -> String {
    let mut output = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('t') => output.push('\t'),
            Some('b') => output.push('\u{0008}'),
            Some('f') => output.push('\u{000C}'),
            Some('$') if quote == b'`' => output.push_str("\\$"),
            Some('u') => {
                if chars.clone().next() == Some('{') {
                    chars.next();
                    let digits: String = chars.by_ref().take_while(|ch| *ch != '}').collect();
                    if let Ok(codepoint) = u32::from_str_radix(&digits, 16) {
                        if let Some(decoded) = char::from_u32(codepoint) {
                            output.push(decoded);
                        }
                    }
                } else {
                    let digits: String = chars.by_ref().take(4).collect();
                    if let Ok(codepoint) = u32::from_str_radix(&digits, 16) {
                        if let Some(decoded) = char::from_u32(codepoint) {
                            output.push(decoded);
                        }
                    }
                }
            }
            Some('x') => {
                let digits: String = chars.by_ref().take(2).collect();
                if let Ok(codepoint) = u32::from_str_radix(&digits, 16) {
                    if let Some(decoded) = char::from_u32(codepoint) {
                        output.push(decoded);
                    }
                }
            }
            Some('\n') => {}
            Some('\r') => {
                if chars.clone().next() == Some('\n') {
                    chars.next();
                }
            }
            Some(escaped) => output.push(escaped),
            None => output.push('\\'),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply::read_units;
    use crate::model::Translation;
    use crate::project::Project;
    use std::fs;
    use std::io::{Read, Write};
    use std::time::{SystemTime, UNIX_EPOCH};
    use zip::write::SimpleFileOptions;
    use zip::{ZipArchive, ZipWriter};

    #[test]
    fn scans_only_player_facing_literals_and_skips_comments() {
        let source = r#"
            // Text.of("ignored comment")
            const id = "kubejs:internal_id";
            player.tell("Welcome to the station");
            Text.of(`Requires ${tier} circuits`);
            Component.literal('Visible tooltip');
            other("not visible");
            Text.of("Concatenated " + suffix);
        "#;
        let found = scan_player_strings(source);
        assert_eq!(
            found
                .iter()
                .map(|string| string.text.as_str())
                .collect::<Vec<_>>(),
            [
                "Welcome to the station",
                "Requires ${tier} circuits",
                "Visible tooltip"
            ]
        );
    }

    #[test]
    fn encodes_quotes_and_control_characters_for_javascript() {
        assert_eq!(encode_js_string("a`b\\c", '`'), "a\\`b\\\\c");
        assert_eq!(encode_js_string("a'b\nc", '\''), "a\\'b\\nc");
        assert_eq!(encode_js_string(r"\${literal}", '`'), r"\${literal}");
    }

    #[test]
    fn decodes_unicode_escapes_and_preserves_escaped_template_interpolation() {
        let found =
            scan_player_strings(r#"Text.of("Hello \u0041"); Text.of(`literal \${notVariable}`);"#);
        assert_eq!(found[0].text, "Hello A");
        assert_eq!(found[1].text, r"literal \${notVariable}");
    }

    #[test]
    fn scan_apply_and_repack_preserve_unrelated_script_text() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("qtrans-js-test-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let zip_path = root.join("pack.zip");
        let source = r#"// Text.of("comment text")
const registryId = "kubejs:internal_id";
player.tell("Welcome, traveler!");
Text.of(`Requires ${tier} circuits`);
other("not player-facing");
"#;
        let zip_file = fs::File::create(&zip_path).unwrap();
        let mut writer = ZipWriter::new(zip_file);
        writer
            .start_file(
                "kubejs/server_scripts/messages.js",
                SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(source.as_bytes()).unwrap();
        writer.finish().unwrap();

        let mut project = Project::create(&zip_path, &root.join("work"), "zh_tw").unwrap();
        let report = crate::scan::scan(&mut project).unwrap();
        assert_eq!(report.stats.script_units, 2);
        let units = read_units(&project).unwrap();
        let translations = units
            .iter()
            .map(|unit| Translation {
                id: unit.id.clone(),
                target: match unit.source.as_str() {
                    "Welcome, traveler!" => "歡迎，旅人！".to_string(),
                    "Requires ${tier} circuits" => "需要 ${tier} 級電路".to_string(),
                    other => panic!("unexpected script unit {other:?}"),
                },
                status: "ok".to_string(),
                provider: Some("test".to_string()),
            })
            .collect::<Vec<_>>();
        let mut output = fs::File::create(project.translations_path()).unwrap();
        for translation in &translations {
            serde_json::to_writer(&mut output, translation).unwrap();
            output.write_all(b"\n").unwrap();
        }

        crate::apply::apply(&mut project).unwrap();
        let script_path = project
            .extracted()
            .join("kubejs/server_scripts/messages.js");
        let applied = fs::read_to_string(&script_path).unwrap();
        assert!(applied.contains(r#"player.tell("歡迎，旅人！")"#));
        assert!(applied.contains("需要 ${tier} 級電路"));
        assert!(applied.contains(r#"const registryId = "kubejs:internal_id";"#));
        assert!(applied.contains(r#"// Text.of("comment text")"#));
        assert!(applied.contains(r#"other("not player-facing");"#));

        crate::apply::apply(&mut project).unwrap();
        assert_eq!(fs::read_to_string(&script_path).unwrap(), applied);

        let output_zip = project.dir.join("output").join("translated.zip");
        crate::package::build_zip(&project, &output_zip).unwrap();
        let mut archive = ZipArchive::new(fs::File::open(output_zip).unwrap()).unwrap();
        let mut entry = archive
            .by_name("kubejs/server_scripts/messages.js")
            .unwrap();
        let mut packed = String::new();
        entry.read_to_string(&mut packed).unwrap();
        assert_eq!(packed, applied);

        fs::remove_dir_all(root).unwrap();
    }
}
