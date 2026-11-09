use crate::ids::stable_id;
use crate::javascript;
use crate::lang;
use crate::model::{Stats, Unit, UnitKind};
use crate::project::Project;
use crate::snbt::{self, Kind, Node};
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;
use walkdir::WalkDir;

pub struct ScanReport {
    pub stats: Stats,
    pub skipped_lang_key_references: usize,
}

pub fn scan(project: &mut Project) -> Result<ScanReport> {
    let extracted = project.extracted();
    let mut units: Vec<Unit> = Vec::new();

    // --- KubeJS resource lang files -------------------------------------
    let assets = extracted.join("kubejs").join("assets");
    if assets.exists() {
        for entry in WalkDir::new(&assets) {
            let entry =
                entry.with_context(|| format!("walk KubeJS assets {}", assets.display()))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if !name.eq_ignore_ascii_case("en_us.json") {
                continue;
            }
            let rel = relative(&extracted, path);
            scan_lang_file(path, &rel, &mut units)?;
        }
    }

    // --- player-facing KubeJS script strings ----------------------------
    for script_dir in ["client_scripts", "server_scripts", "startup_scripts"] {
        let root = extracted.join("kubejs").join(script_dir);
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(&root) {
            let entry = entry.with_context(|| format!("walk KubeJS scripts {}", root.display()))?;
            let path = entry.path();
            if !path.is_file()
                || !path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("js"))
            {
                continue;
            }
            let rel = relative(&extracted, path);
            let source = fs::read_to_string(path)
                .with_context(|| format!("read KubeJS script {}", path.display()))?;
            for (index, string) in javascript::scan_player_strings(&source)
                .into_iter()
                .enumerate()
            {
                units.push(Unit {
                    id: stable_id(&["script", &rel, &string.start.to_string(), &string.text]),
                    kind: UnitKind::Script,
                    file: rel.clone(),
                    key: None,
                    path: Some(format!("player_text[{index}]")),
                    span_start: Some(string.start),
                    span_end: Some(string.end),
                    quoted: true,
                    quote: string.quote,
                    source: string.text,
                    context: format!("KubeJS player-facing text: {rel}"),
                });
            }
        }
    }

    // --- FTB Quests snbt -------------------------------------------------
    let quests_root = extracted.join("config").join("ftbquests");
    if quests_root.exists() {
        for entry in WalkDir::new(&quests_root) {
            let entry = entry
                .with_context(|| format!("walk FTB Quests files {}", quests_root.display()))?;
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("snbt") {
                continue;
            }
            let rel = relative(&extracted, path);
            let text =
                fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
            let root =
                snbt::parse(&text).with_context(|| format!("parse SNBT {}", path.display()))?;
            scan_snbt_file(&root, &rel, &mut units);
        }
    }

    // stable order: by file, then location
    units.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.key.cmp(&b.key))
    });

    let stats = Stats {
        lang_units: units.iter().filter(|u| u.kind == UnitKind::Lang).count(),
        inline_units: units.iter().filter(|u| u.kind == UnitKind::Inline).count(),
        script_units: units.iter().filter(|u| u.kind == UnitKind::Script).count(),
        total_units: units.len(),
        total_chars: units.iter().map(|u| u.source.chars().count()).sum(),
    };

    // strings.jsonl
    let path = project.strings_path();
    let mut out = BufWriter::new(fs::File::create(&path)?);
    for unit in &units {
        serde_json::to_writer(&mut out, unit)?;
        out.write_all(b"\n")?;
    }
    out.flush()?;

    // strings.csv for human review (UTF-8 BOM so Excel opens it correctly)
    let csv_path = project.dir.join("strings.csv");
    let mut csv = BufWriter::new(fs::File::create(&csv_path)?);
    csv.write_all(&[0xEF, 0xBB, 0xBF])?;
    writeln!(csv, "id,kind,file,key_or_path,context,source")?;
    for unit in &units {
        writeln!(
            csv,
            "{},{},{},{},{},{}",
            unit.id,
            unit.kind.as_str(),
            csv_escape(&unit.file),
            csv_escape(unit.key.as_deref().or(unit.path.as_deref()).unwrap_or("")),
            csv_escape(&unit.context),
            csv_escape(&unit.source)
        )?;
    }
    csv.flush()?;

    project.manifest.stats = stats.clone();
    project.manifest.changed.clear();
    project.save_manifest()?;

    Ok(ScanReport {
        stats,
        skipped_lang_key_references: 0,
    })
}

fn scan_lang_file(path: &Path, rel: &str, units: &mut Vec<Unit>) -> Result<()> {
    let map = lang::load(path)?;
    for (key, value) in &map {
        let Value::String(text) = value else {
            continue;
        };
        if !lang::is_translatable_value(text) {
            continue;
        }
        let context = lang_context(key);
        units.push(Unit {
            id: stable_id(&["lang", rel, key, text]),
            kind: UnitKind::Lang,
            file: rel.to_string(),
            key: Some(key.clone()),
            path: None,
            span_start: None,
            span_end: None,
            quoted: true,
            quote: '"',
            source: text.clone(),
            context,
        });
    }
    Ok(())
}

fn lang_context(key: &str) -> String {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.first() == Some(&"atm9") && parts.len() > 3 && parts[1] == "quest" {
        format!("ATM9 quest chapter: {}", parts[2])
    } else {
        let prefix = parts.iter().take(2).copied().collect::<Vec<_>>().join(".");
        format!("lang: {prefix}")
    }
}

fn scan_snbt_file(root: &Node, rel: &str, units: &mut Vec<Unit>) {
    let is_reward_table = rel.starts_with("config/ftbquests/quests/reward_tables/");
    let stem = Path::new(rel)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("snbt");
    let chapter_name = root
        .get("title")
        .and_then(|n| n.as_str())
        .filter(|s| !s.starts_with('{'))
        .unwrap_or(stem)
        .to_string();

    push_node(
        root.get("title"),
        rel,
        if is_reward_table {
            "reward_table.title"
        } else {
            "title"
        },
        if is_reward_table {
            "reward table"
        } else {
            &chapter_name
        },
        units,
    );
    push_node(root.get("subtitle"), rel, "subtitle", &chapter_name, units);

    if let Some(groups) = root.get("chapter_groups") {
        for (i, group) in groups.items().iter().enumerate() {
            let path = format!("chapter_groups[{i}].title");
            push_node(group.get("title"), rel, &path, "chapter group", units);
        }
    }

    let quests = root.get("quests");
    if let Some(quests) = quests {
        for (qi, quest) in quests.items().iter().enumerate() {
            let qtitle = quest
                .get("title")
                .and_then(|n| n.as_str())
                .filter(|s| !s.starts_with('{'))
                .unwrap_or("")
                .to_string();
            let ctx = if qtitle.is_empty() {
                format!("chapter: {chapter_name} / quest #{qi}")
            } else {
                format!("chapter: {chapter_name} / quest: {qtitle}")
            };
            push_node(
                quest.get("title"),
                rel,
                &format!("quests[{qi}].title"),
                &ctx,
                units,
            );
            push_node(
                quest.get("subtitle"),
                rel,
                &format!("quests[{qi}].subtitle"),
                &ctx,
                units,
            );
            if let Some(desc) = quest.get("description") {
                collect_string_or_list(
                    desc,
                    rel,
                    &format!("quests[{qi}].description"),
                    &ctx,
                    units,
                );
            }
            if let Some(tasks) = quest.get("tasks") {
                for (ti, task) in tasks.items().iter().enumerate() {
                    push_node(
                        task.get("title"),
                        rel,
                        &format!("quests[{qi}].tasks[{ti}].title"),
                        &ctx,
                        units,
                    );
                }
            }
            if let Some(rewards) = quest.get("rewards") {
                for (ri, reward) in rewards.items().iter().enumerate() {
                    push_node(
                        reward.get("title"),
                        rel,
                        &format!("quests[{qi}].rewards[{ri}].title"),
                        &ctx,
                        units,
                    );
                }
            }
            if let Some(images) = quest.get("images") {
                for (ii, image) in images.items().iter().enumerate() {
                    if let Some(hover) = image.get("hover") {
                        collect_string_or_list(
                            hover,
                            rel,
                            &format!("quests[{qi}].images[{ii}].hover"),
                            &ctx,
                            units,
                        );
                    }
                }
            }
        }
    }
}

fn collect_string_or_list(node: &Node, file: &str, path: &str, ctx: &str, units: &mut Vec<Unit>) {
    match &node.kind {
        Kind::List(items) => {
            for (i, item) in items.iter().enumerate() {
                push_node(Some(item), file, &format!("{path}[{i}]"), ctx, units);
            }
        }
        _ => push_node(Some(node), file, path, ctx, units),
    }
}

fn push_node(node: Option<&Node>, file: &str, path: &str, ctx: &str, units: &mut Vec<Unit>) {
    let Some(node) = node else { return };
    let Some(text) = node.as_str() else { return };
    if !node.is_string() || !is_translatable(text) {
        return;
    }
    units.push(Unit {
        id: stable_id(&["inline", file, path, text]),
        kind: UnitKind::Inline,
        file: file.to_string(),
        key: None,
        path: Some(path.to_string()),
        span_start: Some(node.span.start),
        span_end: Some(node.span.end),
        quoted: node.quoted,
        quote: node.quote_char(),
        source: text.to_string(),
        context: ctx.to_string(),
    });
}

/// Filter out ids, resource locations, urls and pure formatting strings.
pub fn is_translatable(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return false; // translation key reference
    }
    if !trimmed.chars().any(|c| c.is_ascii_alphabetic()) {
        return false; // only colors / numbers / symbols
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return false;
    }
    if looks_like_resource_location(trimmed) {
        return false;
    }
    // ignore Minecraft color/formatting codes when deciding if there is text
    if !strip_formatting(trimmed)
        .chars()
        .any(|c| c.is_ascii_alphabetic())
    {
        return false;
    }
    true
}

fn strip_formatting(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if (chars[i] == '&' || chars[i] == '§') && i + 1 < chars.len() {
            let code = chars[i + 1].to_ascii_lowercase();
            if "0123456789abcdefklmnorx".contains(code) {
                i += 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn looks_like_resource_location(text: &str) -> bool {
    if text.chars().any(char::is_whitespace) {
        return false;
    }
    let has_colon = text.contains(':');
    let has_slash = text.contains('/');
    if !has_colon && !has_slash {
        return false;
    }
    let allowed =
        |c: char| c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '-' | '.' | '/' | '#' | '~');
    text.chars().all(allowed)
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn csv_escape(value: &str) -> String {
    if value.contains(['"', ',', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_non_text() {
        assert!(is_translatable("Welcome to the pack"));
        assert!(!is_translatable("{atm9.quest.welcome}"));
        assert!(!is_translatable("minecraft:stone"));
        assert!(!is_translatable("textures/gui/logo.png"));
        assert!(!is_translatable("&a"));
        assert!(!is_translatable(""));
    }

    #[test]
    fn root_title_is_recorded_once_for_chapters_and_reward_tables() {
        let root = snbt::parse(r#"{title: "Welcome"}"#).unwrap();
        for (file, expected_path) in [
            ("config/ftbquests/quests/chapters/welcome.snbt", "title"),
            (
                "config/ftbquests/quests/reward_tables/drops.snbt",
                "reward_table.title",
            ),
        ] {
            let mut units = Vec::new();
            scan_snbt_file(&root, file, &mut units);
            assert_eq!(units.len(), 1, "{file}");
            assert_eq!(units[0].path.as_deref(), Some(expected_path));
        }
    }
}
