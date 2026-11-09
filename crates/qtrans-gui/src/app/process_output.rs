use qtrans_core::model::Stats;

#[derive(Default)]
pub(super) struct ProgressUpdate {
    pub(super) stats: Option<Stats>,
    pub(super) translated_units: Option<usize>,
    pub(super) batch: Option<(usize, usize)>,
}

pub(super) fn command_label(args: &[String]) -> &'static str {
    match args.first().map(String::as_str) {
        Some("extract") => "解包模組包",
        Some("scan") => "掃描可翻譯文字",
        Some("translate") => "翻譯文字",
        Some("apply") => "寫回翻譯",
        Some("build") => "建立翻譯模組包",
        Some("run") => "完整漢化流程",
        _ => "執行作業",
    }
}

pub(super) fn format_stderr(line: &str) -> String {
    if line.starts_with("warning:") {
        format!("警告｜{}", line.trim_start_matches("warning:").trim())
    } else if line.starts_with("error:") {
        format!("錯誤｜{}", line.trim_start_matches("error:").trim())
    } else {
        format!("詳細｜{line}")
    }
}

pub(super) fn format_cli_output(line: &str) -> String {
    let trimmed = line.trim();
    let stage = match trimmed {
        "== extract ==" => Some("步驟｜正在解包模組包"),
        "== scan ==" => Some("步驟｜正在掃描可翻譯文字"),
        "== translate ==" => Some("步驟｜正在翻譯文字"),
        "== apply ==" => Some("步驟｜正在寫回翻譯"),
        "== build ==" => Some("步驟｜正在建立翻譯模組包"),
        _ => None,
    };
    if let Some(stage) = stage {
        return stage.to_string();
    }

    if let Some(details) = trimmed.strip_prefix("provider=") {
        let fields: Vec<&str> = details.split_whitespace().collect();
        if fields.len() >= 4 {
            return format!(
                "資訊｜供應商 {} · 目標 {} · 待處理 {} 條（{} 條不重複）",
                fields[0].trim_start_matches("provider="),
                fields[1].trim_start_matches("target="),
                fields[2].trim_start_matches("pending="),
                fields[3].trim_start_matches("unique=")
            );
        }
    }

    if let Some(details) = trimmed.strip_prefix("scanned: ") {
        return format_scan_summary(details);
    }
    if trimmed.contains(" lang + ") && trimmed.contains(" strings (") {
        return format_scan_summary(trimmed);
    }
    if trimmed.contains(" unique strings (") {
        if let Some((counts, details)) = trimmed.split_once(" unique strings (") {
            let percent = details
                .split_once("%)")
                .map(|(value, _)| value)
                .unwrap_or("0");
            let values: Vec<&str> = counts.split('/').collect();
            if values.len() == 2 {
                let translated = value_after(trimmed, "translated units=");
                let reused = value_after(trimmed, "reused=");
                let failed = value_after(trimmed, "failed=");
                return format!(
                    "資訊｜翻譯進度 {}/{} 組文字（{}%）· 已翻譯 {} 條 · 快取沿用 {} 條 · 失敗 {} 條",
                    values[0], values[1], percent, translated, reused, failed
                );
            }
        }
    }
    if let Some(details) = trimmed.strip_prefix("translated ") {
        if let Some((count, rest)) = details.split_once(" strings via ") {
            let (provider, metrics) = rest.split_once(" (").unwrap_or((rest, ""));
            return format!(
                "資訊｜本批完成：{} 條 · 供應商 {}{}",
                count,
                provider,
                format_translation_metrics(metrics)
            );
        }
    }
    if let Some((count, details)) = trimmed.split_once(" translated via ") {
        if let Some((provider, metrics)) = details.split_once(" (") {
            return format!(
                "資訊｜已翻譯 {} 條 · 供應商 {}{}",
                count,
                provider,
                format_translation_metrics(metrics)
            );
        }
    }
    if let Some(details) = trimmed.strip_prefix("applied ") {
        if let Some((count, rest)) = details.split_once(" translations: ") {
            return format!("資訊｜已寫回 {} 條翻譯 · {}", count, rest);
        }
    }
    if let Some((count, details)) = trimmed.split_once(" translations applied, ") {
        return format!("資訊｜已寫回 {count} 條翻譯 · {details}");
    }
    if let Some(details) = trimmed.strip_prefix("built ") {
        return format!("資訊｜已建立輸出：{details}");
    }
    if let Some(details) = trimmed.strip_prefix("overrides folder: ") {
        return format!("資訊｜已匯出覆寫檔案：{details}");
    }
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("資訊｜{trimmed}")
    }
}

fn value_after<'a>(line: &'a str, key: &str) -> &'a str {
    line.split_once(key)
        .and_then(|(_, value)| value.split_whitespace().next())
        .unwrap_or("0")
}

pub(super) fn parse_cli_progress(line: &str) -> Option<ProgressUpdate> {
    let trimmed = line.trim();
    let scan_details = trimmed.strip_prefix("scanned: ").or_else(|| {
        (trimmed.contains(" lang + ") && trimmed.contains(" strings (")).then_some(trimmed)
    });
    if let Some(details) = scan_details {
        let fields: Vec<&str> = details.split_whitespace().collect();
        let count_before = |label: &str| {
            fields
                .iter()
                .position(|field| *field == label)
                .and_then(|index| index.checked_sub(1))
                .and_then(|index| fields.get(index))
                .and_then(|value| value.parse::<usize>().ok())
        };
        let total_units = fields
            .iter()
            .position(|field| *field == "=")
            .and_then(|index| fields.get(index + 1))
            .and_then(|value| value.parse::<usize>().ok())?;
        let total_chars = fields
            .iter()
            .find(|field| field.starts_with('('))
            .and_then(|value| value.trim_start_matches('(').parse::<usize>().ok())
            .unwrap_or_default();
        return Some(ProgressUpdate {
            stats: Some(Stats {
                lang_units: count_before("lang")?,
                inline_units: count_before("inline")?,
                script_units: count_before("script")?,
                total_units,
                total_chars,
            }),
            ..Default::default()
        });
    }

    if trimmed.contains(" unique strings (") {
        let (counts, _) = trimmed.split_once(" unique strings (")?;
        let (done, total) = counts.split_once('/')?;
        let done = done.trim().parse().ok()?;
        let total = total.trim().parse().ok()?;
        let translated_units = value_after(trimmed, "translated units=").parse().ok();
        return Some(ProgressUpdate {
            translated_units,
            batch: Some((done, total)),
            ..Default::default()
        });
    }
    None
}

fn format_translation_metrics(metrics: &str) -> String {
    let reused = metric_before(metrics, "reused");
    let failed = metric_before(metrics, "failed");
    format!(" · 快取沿用 {reused} 條 · 失敗 {failed} 條")
}

fn metric_before<'a>(metrics: &'a str, key: &str) -> &'a str {
    let fields: Vec<&str> = metrics
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != ' ')
        .split_whitespace()
        .collect();
    fields
        .iter()
        .position(|field| field.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()) == key)
        .and_then(|index| index.checked_sub(1))
        .and_then(|index| fields.get(index))
        .copied()
        .unwrap_or("0")
}

fn format_scan_summary(details: &str) -> String {
    let fields: Vec<&str> = details.split_whitespace().collect();
    let count_before = |label: &str| {
        fields
            .iter()
            .position(|field| *field == label)
            .and_then(|index| index.checked_sub(1))
            .and_then(|index| fields.get(index))
            .copied()
            .unwrap_or("?")
    };
    let total = fields
        .iter()
        .position(|field| *field == "=")
        .and_then(|index| fields.get(index + 1))
        .copied();
    let chars = fields
        .iter()
        .find(|field| field.starts_with('('))
        .map(|field| field.trim_start_matches('('));
    if let (Some(total), Some(chars)) = (total, chars) {
        format!(
            "資訊｜掃描完成：語言檔 {} 條 · 任務 {} 條 · KubeJS {} 條 · 共 {} 條（{} 字元）",
            count_before("lang"),
            count_before("inline"),
            count_before("script"),
            total,
            chars
        )
    } else {
        format!("資訊｜掃描完成：{details}")
    }
}

#[cfg(test)]
mod tests {
    use super::{command_label, format_cli_output, format_stderr, parse_cli_progress};

    #[test]
    fn command_labels_hide_cli_arguments() {
        let args = vec![
            "translate".to_string(),
            "--project".to_string(),
            "work".to_string(),
        ];
        assert_eq!(command_label(&args), "翻譯文字");
    }

    #[test]
    fn cli_output_is_presented_as_readable_log_entries() {
        assert_eq!(format_cli_output("== extract =="), "步驟｜正在解包模組包");
        assert_eq!(
            format_cli_output("scanned: 5 lang + 4 inline + 2 script = 11 strings (100 chars)"),
            "資訊｜掃描完成：語言檔 5 條 · 任務 4 條 · KubeJS 2 條 · 共 11 條（100 字元）"
        );
        assert_eq!(
            format_cli_output("5 lang + 4 inline + 2 script = 11 strings (100 chars)"),
            "資訊｜掃描完成：語言檔 5 條 · 任務 4 條 · KubeJS 2 條 · 共 11 條（100 字元）"
        );
        assert_eq!(
            format_cli_output("provider=google_free target=zh-TW pending=24 unique=12"),
            "資訊｜供應商 google_free · 目標 zh-TW · 待處理 24 條（12 條不重複）"
        );
        assert_eq!(
            format_cli_output(
                "  5/10 unique strings (50.0%) | translated units=6 reused=2 failed=1"
            ),
            "資訊｜翻譯進度 5/10 組文字（50.0%）· 已翻譯 6 條 · 快取沿用 2 條 · 失敗 1 條"
        );
        assert_eq!(
            format_cli_output("12 translated via google_free (2 reused, 0 failed)"),
            "資訊｜已翻譯 12 條 · 供應商 google_free · 快取沿用 2 條 · 失敗 0 條"
        );
    }

    #[test]
    fn stderr_level_is_preserved_in_log_prefix() {
        assert_eq!(
            format_stderr("warning: retrying request"),
            "警告｜retrying request"
        );
        assert_eq!(
            format_stderr("error: service unavailable"),
            "錯誤｜service unavailable"
        );
    }

    #[test]
    fn scan_output_updates_project_totals_immediately() {
        let progress =
            parse_cli_progress("scanned: 5 lang + 4 inline + 2 script = 11 strings (100 chars)")
                .unwrap();
        let stats = progress.stats.unwrap();
        assert_eq!(stats.lang_units, 5);
        assert_eq!(stats.inline_units, 4);
        assert_eq!(stats.script_units, 2);
        assert_eq!(stats.total_units, 11);
        assert_eq!(stats.total_chars, 100);
    }

    #[test]
    fn translation_output_updates_current_counts_and_batch_progress() {
        let progress = parse_cli_progress(
            "  5/10 unique strings (50.0%) | translated units=6 reused=2 failed=1",
        )
        .unwrap();
        assert_eq!(progress.translated_units, Some(6));
        assert_eq!(progress.batch, Some((5, 10)));
    }
}
