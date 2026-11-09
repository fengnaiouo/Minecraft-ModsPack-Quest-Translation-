use crate::javascript;
use crate::lang;
use crate::model::{validate_target_code, Translation, Unit, UnitKind};
use crate::project::{safe_project_path, Project};
use crate::snbt::encode_string;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

pub struct ApplyReport {
    pub translated: usize,
    pub missing: usize,
    pub lang_files: usize,
    pub snbt_files: usize,
    pub javascript_files: usize,
    pub changed: Vec<String>,
}

pub fn read_units(project: &Project) -> Result<Vec<Unit>> {
    let file = fs::File::open(project.strings_path())
        .with_context(|| "strings.jsonl not found, run `qtrans scan` first")?;
    let mut units = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        units.push(serde_json::from_str(&line)?);
    }
    Ok(units)
}

pub fn read_translations(project: &Project) -> Result<HashMap<String, String>> {
    let path = project.translations_path();
    let mut map = HashMap::new();
    if !path.exists() {
        return Ok(map);
    }
    let file = fs::File::open(&path)?;
    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    let mut line_number = 0;
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        line_number += 1;
        let terminated = line.ends_with(b"\n");
        if terminated {
            line.pop();
        }
        if line.ends_with(b"\r") {
            line.pop();
        }
        if line.iter().all(|byte| byte.is_ascii_whitespace()) {
            continue;
        }
        let t: Translation = match serde_json::from_slice(&line) {
            Ok(t) => t,
            Err(_) if !terminated => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "parse translation record at line {line_number} in {}",
                        path.display()
                    )
                });
            }
        };
        if t.status == "ok" && !t.target.trim().is_empty() {
            map.insert(t.id, t.target);
        }
    }
    Ok(map)
}

/// Append an approved manual translation for a scanned project unit.
pub fn save_manual_translation(project: &Project, unit: &Unit, target: &str) -> Result<()> {
    anyhow::ensure!(
        !target.trim().is_empty(),
        "manual translation cannot be empty"
    );
    let translation = Translation {
        id: unit.id.clone(),
        target: target.to_string(),
        status: "ok".to_string(),
        provider: Some("manual".to_string()),
    };
    let path = project.translations_path();
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)?;
    let mut reader = BufReader::new(&mut file);
    let mut line = Vec::new();
    let mut offset = 0u64;
    let mut last_line_start = 0u64;
    let mut last_line = Vec::new();
    loop {
        line.clear();
        let bytes = reader.read_until(b'\n', &mut line)?;
        if bytes == 0 {
            break;
        }
        last_line_start = offset;
        offset += bytes as u64;
        last_line.clone_from(&line);
    }
    drop(reader);
    if !last_line.is_empty() && !last_line.ends_with(b"\n") {
        if serde_json::from_slice::<Translation>(&last_line).is_ok() {
            file.seek(SeekFrom::End(0))?;
            file.write_all(b"\n")?;
        } else {
            file.set_len(last_line_start)?;
        }
    }
    file.seek(SeekFrom::End(0))?;
    serde_json::to_writer(&mut file, &translation)?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}

pub fn apply(project: &mut Project) -> Result<ApplyReport> {
    let units = read_units(project)?;
    let translations = read_translations(project)?;
    let extracted = project.extracted();
    let target = project.config.target.clone();
    validate_target_code(&target)?;

    let mut missing = 0usize;
    let mut changed: Vec<String> = Vec::new();
    let mut lang_files = 0usize;
    let mut snbt_files = 0usize;
    let mut javascript_files = 0usize;

    // ---- lang JSON -------------------------------------------------------
    let mut by_file: BTreeMap<&str, Vec<&Unit>> = BTreeMap::new();
    for unit in &units {
        if unit.kind == UnitKind::Lang {
            by_file.entry(unit.file.as_str()).or_default().push(unit);
        }
    }
    for (file, file_units) in &by_file {
        let source_path = safe_project_path(&extracted, file)?;
        if !source_path.exists() {
            continue;
        }
        let out_path = source_path
            .parent()
            .map(|p| p.join(format!("{target}.json")))
            .unwrap_or_else(|| Path::new(&format!("{target}.json")).to_path_buf());
        let mut merged = if out_path.exists() {
            lang::load(&out_path)?
        } else {
            serde_json::Map::new()
        };
        let mut applied_here = 0usize;
        for unit in file_units {
            let Some(key) = &unit.key else { continue };
            match translations.get(&unit.id) {
                Some(text) => {
                    merged.insert(key.clone(), Value::String(text.clone()));
                    applied_here += 1;
                }
                None => {
                    missing += 1;
                }
            }
        }
        // Untranslated keys are omitted on purpose: Minecraft falls back to
        // en_us for missing entries instead of showing English twice.
        if merged.is_empty() || (applied_here == 0 && !out_path.exists()) {
            continue;
        }
        lang::save(&out_path, &merged)?;
        let rel = out_path
            .strip_prefix(&extracted)
            .unwrap_or(&out_path)
            .to_string_lossy()
            .replace('\\', "/");
        if !changed.contains(&rel) {
            changed.push(rel);
            lang_files += 1;
        }
    }

    // ---- inline snbt -----------------------------------------------------
    let mut by_snbt: BTreeMap<&str, Vec<&Unit>> = BTreeMap::new();
    for unit in &units {
        if unit.kind == UnitKind::Inline {
            by_snbt.entry(unit.file.as_str()).or_default().push(unit);
        }
    }
    // Always start from the pristine source: a previous apply may have already
    // shifted byte offsets inside the extracted SNBT files.
    let snbt_files_list: Vec<String> = by_snbt.keys().map(|k| (*k).to_string()).collect();
    if !snbt_files_list.is_empty() {
        project.restore_from_zip(&snbt_files_list)?;
    }
    for (file, file_units) in &by_snbt {
        let path = safe_project_path(&extracted, file)?;
        if !path.exists() {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        let mut edits: Vec<(usize, usize, String)> = Vec::new();
        for unit in file_units {
            let Some(target_text) = translations.get(&unit.id) else {
                missing += 1;
                continue;
            };
            let (Some(start), Some(end)) = (unit.span_start, unit.span_end) else {
                continue;
            };
            if start >= end
                || end > text.len()
                || !text.is_char_boundary(start)
                || !text.is_char_boundary(end)
            {
                continue;
            }
            let replacement = encode_string(target_text, unit.quoted, unit.quote);
            edits.push((start, end, replacement));
        }
        if edits.is_empty() {
            continue;
        }
        edits.sort_by_key(|(start, _, _)| *start);
        let mut last_end = 0usize;
        edits.retain(|(start, end, _)| {
            if *start < last_end {
                return false;
            }
            last_end = *end;
            true
        });
        let mut output = text.clone();
        for (start, end, replacement) in edits.into_iter().rev() {
            output.replace_range(start..end, &replacement);
        }
        // sanity check: the file must still parse
        if let Err(err) = crate::snbt::parse(&output) {
            eprintln!("warning: patched {} does not parse: {err}", path.display());
            continue;
        }
        fs::write(&path, output)?;
        changed.push(file.to_string());
        snbt_files += 1;
    }

    // ---- player-facing KubeJS script strings ----------------------------
    let mut by_script: BTreeMap<&str, Vec<&Unit>> = BTreeMap::new();
    for unit in &units {
        if unit.kind == UnitKind::Script {
            by_script.entry(unit.file.as_str()).or_default().push(unit);
        }
    }
    let script_files: Vec<String> = by_script.keys().map(|file| (*file).to_string()).collect();
    if !script_files.is_empty() {
        project.restore_from_zip(&script_files)?;
    }
    for (file, file_units) in &by_script {
        let path = safe_project_path(&extracted, file)?;
        if !path.exists() {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("read KubeJS script {}", path.display()))?;
        let mut edits = Vec::new();
        for unit in file_units {
            let Some(target_text) = translations.get(&unit.id) else {
                missing += 1;
                continue;
            };
            let (Some(start), Some(end)) = (unit.span_start, unit.span_end) else {
                continue;
            };
            if start >= end
                || end > text.len()
                || !text.is_char_boundary(start)
                || !text.is_char_boundary(end)
            {
                continue;
            }
            edits.push((
                start,
                end,
                javascript::encode_js_string(target_text, unit.quote),
            ));
        }
        if edits.is_empty() {
            continue;
        }
        edits.sort_by_key(|(start, _, _)| *start);
        let mut last_end = 0usize;
        edits.retain(|(start, end, _)| {
            if *start < last_end {
                return false;
            }
            last_end = *end;
            true
        });
        let mut output = text.clone();
        for (start, end, replacement) in edits.into_iter().rev() {
            output.replace_range(start..end, &replacement);
        }
        fs::write(&path, output)?;
        changed.push(file.to_string());
        javascript_files += 1;
    }

    // ---- review file for untranslated units ------------------------------
    write_untranslated(project, &units, &translations)?;

    project.manifest.changed = changed.clone();
    project.save_manifest()?;

    Ok(ApplyReport {
        translated: translations.len(),
        missing,
        lang_files,
        snbt_files,
        javascript_files,
        changed,
    })
}

fn write_untranslated(
    project: &Project,
    units: &[Unit],
    translations: &HashMap<String, String>,
) -> Result<()> {
    let path = project.dir.join("untranslated.csv");
    let mut out = BufWriter::new(fs::File::create(path)?);
    out.write_all(&[0xEF, 0xBB, 0xBF])?;
    writeln!(out, "id,file,key_or_path,context,source")?;
    for unit in units {
        if translations.contains_key(&unit.id) {
            continue;
        }
        writeln!(
            out,
            "{},{},{},{},{}",
            unit.id,
            crate::scan::csv_escape(&unit.file),
            crate::scan::csv_escape(unit.key.as_deref().or(unit.path.as_deref()).unwrap_or("")),
            crate::scan::csv_escape(&unit.context),
            crate::scan::csv_escape(&unit.source)
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod translation_tests {
    use super::*;
    use crate::model::{Manifest, ProjectConfig, Stats, Unit, UnitKind};

    fn project_with_translations(content: &[u8]) -> Project {
        let dir = crate::test_support::unique_temp_dir("translations-test");
        fs::write(dir.join("translations.jsonl"), content).unwrap();
        Project {
            dir,
            manifest: Manifest {
                pack_name: "test".into(),
                source_zip: String::new(),
                zip_prefix: String::new(),
                target_lang: "zh_tw".into(),
                created_at: String::new(),
                updated_at: String::new(),
                stats: Stats::default(),
                changed: Vec::new(),
            },
            config: ProjectConfig::default(),
        }
    }

    fn write_test_unit(project: &Project) -> Unit {
        let unit = Unit {
            id: "unit-1".into(),
            kind: UnitKind::Lang,
            file: "kubejs/assets/test/lang/en_us.json".into(),
            key: Some("hello".into()),
            path: None,
            span_start: None,
            span_end: None,
            quoted: false,
            quote: '"',
            source: "Hello".into(),
            context: "test".into(),
        };
        fs::write(
            project.strings_path(),
            format!("{}\n", serde_json::to_string(&unit).unwrap()),
        )
        .unwrap();
        unit
    }

    #[test]
    fn read_translations_ignores_only_a_truncated_final_record() {
        let project = project_with_translations(
            b"{\"id\":\"a\",\"target\":\"\xe4\xb8\xad\",\"status\":\"ok\"}\n{\"id\":",
        );

        let result = read_translations(&project).unwrap();

        assert_eq!(result.get("a").map(String::as_str), Some("中"));
        fs::remove_dir_all(project.dir).unwrap();
    }

    #[test]
    fn read_translations_reports_malformed_records_before_the_end() {
        let project = project_with_translations(b"{not-json}\n");

        let error = read_translations(&project).unwrap_err();

        assert!(format!("{error:#}").contains("line 1"));
        fs::remove_dir_all(project.dir).unwrap();
    }

    #[test]
    fn save_manual_translation_appends_an_ok_record_for_a_known_unit() {
        let project = project_with_translations(b"");
        let unit = write_test_unit(&project);

        save_manual_translation(&project, &unit, "你好").unwrap();

        assert_eq!(
            read_translations(&project)
                .unwrap()
                .get("unit-1")
                .map(String::as_str),
            Some("你好")
        );
        let records = fs::read_to_string(project.translations_path()).unwrap();
        assert!(records.contains("\"provider\":\"manual\""));
        fs::remove_dir_all(project.dir).unwrap();
    }

    #[test]
    fn save_manual_translation_rejects_empty_text() {
        let project = project_with_translations(b"");
        let unit = write_test_unit(&project);

        assert!(save_manual_translation(&project, &unit, "  ").is_err());
        assert!(fs::read_to_string(project.translations_path())
            .unwrap()
            .is_empty());
        fs::remove_dir_all(project.dir).unwrap();
    }

    #[test]
    fn save_manual_translation_recovers_a_truncated_final_record() {
        let project = project_with_translations(b"{\"id\":\"unit-1\",\"target\":");
        let unit = write_test_unit(&project);

        save_manual_translation(&project, &unit, "你好").unwrap();

        assert_eq!(
            read_translations(&project)
                .unwrap()
                .get("unit-1")
                .map(String::as_str),
            Some("你好")
        );
        fs::remove_dir_all(project.dir).unwrap();
    }

    #[test]
    fn save_manual_translation_separates_a_valid_unterminated_final_record() {
        let project = project_with_translations(
            r#"{"id":"unit-1","target":"旧譯","status":"ok"}"#.as_bytes(),
        );
        let unit = write_test_unit(&project);

        save_manual_translation(&project, &unit, "新譯").unwrap();

        assert_eq!(
            read_translations(&project)
                .unwrap()
                .get("unit-1")
                .map(String::as_str),
            Some("新譯")
        );
        fs::remove_dir_all(project.dir).unwrap();
    }
}
