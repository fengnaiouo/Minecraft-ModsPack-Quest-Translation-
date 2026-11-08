use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

pub type LangMap = Map<String, Value>;

pub fn load(path: &Path) -> Result<LangMap> {
    let text =
        fs::read_to_string(path).with_context(|| format!("read lang file {}", path.display()))?;
    let value: Value =
        serde_json::from_str(&text).with_context(|| format!("parse JSON {}", path.display()))?;
    match value {
        Value::Object(map) => Ok(map),
        _ => anyhow::bail!("{} is not a JSON object", path.display()),
    }
}

pub fn save(path: &Path, map: &LangMap) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut text = serde_json::to_string_pretty(map)?;
    text.push('\n');
    fs::write(path, text).with_context(|| format!("write lang file {}", path.display()))?;
    Ok(())
}

/// Whether a lang value contains anything worth translating.
pub fn is_translatable_value(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    // pure formatting / color codes
    if !trimmed.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    // already a translation key reference
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return false;
    }
    true
}
