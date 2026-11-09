use serde::{Deserialize, Serialize};

pub fn validate_target_code(target: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        !target.is_empty()
            && target.len() <= 32
            && target.as_bytes()[0].is_ascii_alphabetic()
            && target
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')),
        "目標語言代碼格式無效：僅允許英文字母、數字、底線與連字號，且須以英文字母開頭"
    );
    Ok(())
}

/// Where a translatable string came from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnitKind {
    /// A value inside `kubejs/assets/<mod>/lang/en_us.json`.
    Lang,
    /// A string embedded directly in an `.snbt` file.
    Inline,
    /// A player-facing string embedded in a KubeJS script.
    Script,
}

impl UnitKind {
    pub fn as_str(self) -> &'static str {
        match self {
            UnitKind::Lang => "lang",
            UnitKind::Inline => "inline",
            UnitKind::Script => "script",
        }
    }
}

/// One translatable string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    /// Stable id derived from file + location + source text.
    pub id: String,
    pub kind: UnitKind,
    /// Path relative to the pack root (first archive component stripped).
    pub file: String,
    /// Lang key, for `UnitKind::Lang`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Location inside the snbt tree, for `UnitKind::Inline`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Byte span of the string literal in the original file (inline only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_start: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_end: Option<usize>,
    /// Whether the original literal was quoted.
    #[serde(default)]
    pub quoted: bool,
    #[serde(default = "default_quote")]
    pub quote: char,
    /// Original English text (decoded).
    pub source: String,
    /// Human readable context, e.g. `chapter: Create / quest: Andesite Alloy`.
    #[serde(default)]
    pub context: String,
}

fn default_quote() -> char {
    '"'
}

/// One translation result. Stored as JSON Lines in `translations.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Translation {
    pub id: String,
    pub target: String,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

fn default_status() -> String {
    "ok".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Stats {
    pub lang_units: usize,
    pub inline_units: usize,
    #[serde(default)]
    pub script_units: usize,
    pub total_units: usize,
    pub total_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub pack_name: String,
    pub source_zip: String,
    /// Prefix inside the zip that maps to the pack root, usually `overrides/`.
    pub zip_prefix: String,
    pub target_lang: String,
    pub created_at: String,
    pub updated_at: String,
    pub stats: Stats,
    /// Files (pack-relative) that `apply` rewrote.
    #[serde(default)]
    pub changed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Go translation service base URL.
    #[serde(default = "default_service")]
    pub service: String,
    /// Provider name passed to the service (`google_free`, `openai`, `offline`, ...).
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Model for OpenAI-compatible providers.
    #[serde(default)]
    pub model: String,
    /// Minecraft language code used for output file names.
    #[serde(default = "default_target")]
    pub target: String,
    /// BCP-47 language tag passed to the service.
    #[serde(default = "default_service_lang")]
    pub service_lang: String,
    #[serde(default = "default_batch")]
    pub batch_size: usize,
    /// Optional maximum number of pending units per translation run.
    #[serde(default)]
    pub translation_limit: Option<usize>,
    /// Translate strings that are already partially translated?
    #[serde(default)]
    pub overwrite: bool,
}

fn default_service() -> String {
    "http://127.0.0.1:8787".to_string()
}
fn default_provider() -> String {
    "google_free".to_string()
}
fn default_target() -> String {
    "zh_tw".to_string()
}
fn default_service_lang() -> String {
    "zh-TW".to_string()
}
fn default_batch() -> usize {
    16
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            service: default_service(),
            provider: default_provider(),
            model: String::new(),
            target: default_target(),
            service_lang: default_service_lang(),
            batch_size: default_batch(),
            translation_limit: None,
            overwrite: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_codes_cannot_escape_the_language_directory() {
        for valid in ["zh_tw", "en_us", "zh", "zh-TW", "zh-Hant-TW"] {
            assert!(validate_target_code(valid).is_ok(), "{valid}");
        }
        for invalid in [
            "",
            "../en_us",
            "zh/tw",
            ".hidden",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ] {
            assert!(validate_target_code(invalid).is_err(), "{invalid}");
        }
    }
}
