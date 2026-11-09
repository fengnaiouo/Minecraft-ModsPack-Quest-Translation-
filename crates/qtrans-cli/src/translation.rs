use anyhow::{bail, Result};
use qtrans_core::apply::read_translations;
use qtrans_core::model::Unit;
use qtrans_core::project::Project;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::time::Duration;

#[derive(Serialize)]
struct TranslateRequest<'a> {
    provider: &'a str,
    model: &'a str,
    target: &'a str,
    force: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    glossary: Vec<GlossaryEntry>,
    items: Vec<RequestItem<'a>>,
}

#[derive(Serialize, Clone)]
struct GlossaryEntry {
    term: String,
    translation: String,
}

#[derive(Serialize)]
struct RequestItem<'a> {
    id: String,
    text: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    context: &'a str,
}

#[derive(Deserialize)]
struct TranslateResponse {
    items: Vec<ResponseItem>,
}

#[derive(Deserialize)]
struct ResponseItem {
    id: String,
    text: String,
    #[serde(default)]
    status: Option<String>,
}

pub(super) struct TranslateReport {
    pub translated: usize,
    pub reused: usize,
    pub failed: usize,
    pub provider: String,
}

pub(super) fn translate(
    project: &Project,
    limit: Option<usize>,
    overwrite: bool,
) -> Result<TranslateReport> {
    let units = qtrans_core::apply::read_units(project)?;
    let existing = read_translations(project)?;
    let glossary = load_glossary(&project.dir.join("glossary.csv"))?;
    let cached_by_text = cached_translations(&units, &existing, overwrite);

    let mut pending: Vec<&Unit> = units
        .iter()
        .filter(|unit| overwrite || !existing.contains_key(&unit.id))
        .filter(|unit| !unit.source.trim().is_empty())
        .collect();
    if let Some(limit) = limit {
        pending.truncate(limit);
    }

    let mut texts: Vec<&str> = Vec::new();
    let mut by_text: HashMap<&str, Vec<&Unit>> = HashMap::new();
    for unit in &pending {
        if !by_text.contains_key(unit.source.as_str()) {
            texts.push(unit.source.as_str());
        }
        by_text.entry(unit.source.as_str()).or_default().push(unit);
    }

    let batch_size = project.config.batch_size.max(1);
    let total = texts.len();
    let mut done = 0usize;
    let mut reused = 0usize;
    let mut translated = 0usize;
    let mut failed = 0usize;
    let out_path = project.translations_path();
    let mut out = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&out_path)?;

    println!(
        "provider={} target={} pending={} unique={}",
        project.config.provider,
        project.config.service_lang,
        pending.len(),
        total
    );

    for chunk in texts.chunks(batch_size) {
        let mut request_items: Vec<RequestItem> = Vec::new();
        let mut local: Vec<(String, Vec<&Unit>)> = Vec::new();
        for text in chunk {
            if let Some(existing_text) = cached_by_text.get(*text) {
                write_units(
                    &mut out,
                    &by_text[*text],
                    existing_text,
                    &project.config.provider,
                )?;
                translated += by_text[*text].len();
                reused += by_text[*text].len();
                continue;
            }
            let id = format!("t{}", done + request_items.len());
            request_items.push(RequestItem {
                id: id.clone(),
                text,
                context: by_text[*text]
                    .first()
                    .map(|unit| unit.context.as_str())
                    .unwrap_or(""),
            });
            local.push((id, by_text[*text].clone()));
        }

        if !request_items.is_empty() {
            let request = TranslateRequest {
                provider: &project.config.provider,
                model: &project.config.model,
                target: &project.config.service_lang,
                force: overwrite,
                glossary: glossary.clone(),
                items: request_items,
            };
            let response = call_service(&project.config.service, &request)?;
            let expected_ids: Vec<String> = local.iter().map(|(id, _)| id.clone()).collect();
            let by_id = index_response_items(&expected_ids, response.items)?;
            for (id, units) in &local {
                let Some(item) = by_id.get(id) else {
                    eprintln!("warning: service returned no text for id {id}");
                    failed += units.len();
                    continue;
                };
                if item.status.as_deref().unwrap_or("ok") != "ok" || item.text.trim().is_empty() {
                    eprintln!("warning: service returned an empty or failed result for id {id}");
                    failed += units.len();
                    continue;
                }
                write_units(&mut out, units, &item.text, &project.config.provider)?;
                translated += units.len();
            }
            out.flush()?;
        }

        done += chunk.len();
        let pct = done as f64 * 100.0 / total.max(1) as f64;
        println!(
            "  {done}/{total} unique strings ({pct:.1}%) | translated units={translated} reused={reused} failed={failed}"
        );
    }

    Ok(TranslateReport {
        translated,
        reused,
        failed,
        provider: project.config.provider.clone(),
    })
}

fn index_response_items(
    expected_ids: &[String],
    items: Vec<ResponseItem>,
) -> Result<HashMap<String, ResponseItem>> {
    let expected: HashSet<&str> = expected_ids.iter().map(String::as_str).collect();
    let mut by_id = HashMap::with_capacity(items.len());
    for item in items {
        anyhow::ensure!(
            expected.contains(item.id.as_str()),
            "translation service returned an unexpected item ID: {}",
            item.id
        );
        anyhow::ensure!(
            by_id.insert(item.id.clone(), item).is_none(),
            "translation service returned a duplicate item ID"
        );
    }
    Ok(by_id)
}

fn cached_translations(
    units: &[Unit],
    existing: &HashMap<String, String>,
    overwrite: bool,
) -> HashMap<String, String> {
    if overwrite {
        return HashMap::new();
    }
    let mut cached = HashMap::new();
    for unit in units {
        if let Some(text) = existing.get(&unit.id) {
            cached.insert(unit.source.clone(), text.clone());
        }
    }
    cached
}

fn write_units(
    out: &mut std::fs::File,
    units: &[&Unit],
    target: &str,
    provider: &str,
) -> Result<()> {
    for unit in units {
        let line = serde_json::json!({
            "id": unit.id,
            "target": target,
            "status": "ok",
            "provider": provider,
        });
        writeln!(out, "{line}")?;
    }
    Ok(())
}

fn call_service(service: &str, request: &TranslateRequest<'_>) -> Result<TranslateResponse> {
    let url = format!("{}/v1/translate", service.trim_end_matches('/'));
    let mut last_error = String::new();
    for attempt in 1..=3 {
        match ureq::post(&url)
            .timeout(Duration::from_secs(600))
            .send_json(request)
        {
            Ok(response) => match response.into_json::<TranslateResponse>() {
                Ok(parsed) => return Ok(parsed),
                Err(error) => last_error = format!("invalid response JSON: {error}"),
            },
            Err(ureq::Error::Status(code, response)) => {
                let body = response.into_string().unwrap_or_default();
                last_error = format!("HTTP {code}: {body}");
            }
            Err(error) => last_error = format!("{error}"),
        }
        if attempt < 3 {
            eprintln!("  retry {attempt}/3 after error: {last_error}");
            std::thread::sleep(Duration::from_secs(attempt as u64));
        }
    }
    bail!("translation service failed: {last_error}")
}

fn load_glossary(path: &Path) -> Result<Vec<GlossaryEntry>> {
    let mut entries = Vec::new();
    if !path.exists() {
        return Ok(entries);
    }
    let reader = BufReader::new(fs::File::open(path)?);
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || (index == 0 && line.starts_with("term,")) {
            continue;
        }
        let mut parts = line.splitn(2, ',');
        let (Some(term), Some(translation)) = (parts.next(), parts.next()) else {
            continue;
        };
        let term = term.trim();
        let translation = translation.trim();
        if !term.is_empty() && !translation.is_empty() {
            entries.push(GlossaryEntry {
                term: term.to_string(),
                translation: translation.to_string(),
            });
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtrans_core::model::UnitKind;

    #[test]
    fn overwrite_flag_is_sent_to_the_translation_service() {
        let request = TranslateRequest {
            provider: "google_free",
            model: "",
            target: "zh-TW",
            force: true,
            glossary: Vec::new(),
            items: vec![RequestItem {
                id: "one".into(),
                text: "Hello",
                context: "",
            }],
        };
        let body = serde_json::to_value(request).unwrap();
        assert_eq!(body["force"], true);
    }

    #[test]
    fn overwrite_disables_identical_text_translation_reuse() {
        let unit = Unit {
            id: "unit".into(),
            kind: UnitKind::Lang,
            file: "kubejs/assets/test/lang/en_us.json".into(),
            key: Some("test.text".into()),
            path: None,
            span_start: None,
            span_end: None,
            quoted: true,
            quote: '"',
            source: "Hello".into(),
            context: String::new(),
        };
        let existing = HashMap::from([("unit".to_string(), "舊翻譯".to_string())]);

        assert_eq!(
            cached_translations(std::slice::from_ref(&unit), &existing, false).get("Hello"),
            Some(&"舊翻譯".to_string())
        );
        assert!(cached_translations(&[unit], &existing, true).is_empty());
    }

    #[test]
    fn response_items_must_have_known_unique_ids() {
        let expected = vec!["one".to_string(), "two".to_string()];
        let valid = vec![
            ResponseItem {
                id: "two".into(),
                text: "二".into(),
                status: Some("ok".into()),
            },
            ResponseItem {
                id: "one".into(),
                text: "一".into(),
                status: Some("ok".into()),
            },
        ];
        let indexed = index_response_items(&expected, valid).unwrap();
        assert_eq!(indexed.len(), 2);

        let duplicate = vec![
            ResponseItem {
                id: "one".into(),
                text: "一".into(),
                status: None,
            },
            ResponseItem {
                id: "one".into(),
                text: "壹".into(),
                status: None,
            },
        ];
        assert!(index_response_items(&expected, duplicate).is_err());
        assert!(index_response_items(
            &expected,
            vec![ResponseItem {
                id: "unexpected".into(),
                text: "結果".into(),
                status: None,
            }]
        )
        .is_err());
    }
}
