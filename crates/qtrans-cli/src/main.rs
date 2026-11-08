use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use qtrans_core::apply::read_translations;
use qtrans_core::model::{Unit, UnitKind};
use qtrans_core::project::Project;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "qtrans",
    version,
    about = "Minecraft modpack quest translator (FTB Quests + KubeJS)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract quest/kubejs files from a modpack zip into a project.
    Extract {
        zip: PathBuf,
        #[arg(long, default_value = "work")]
        work: PathBuf,
        #[arg(long, default_value = "zh_tw")]
        target: String,
    },
    /// Scan a project and write strings.jsonl / strings.csv.
    Scan {
        #[arg(long)]
        project: PathBuf,
    },
    /// Machine-translate pending strings through the Go service.
    Translate {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        service: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        target: Option<String>,
        /// Translate at most N strings (useful for a quick smoke test).
        #[arg(long)]
        limit: Option<usize>,
        /// Re-translate strings that already have a translation.
        #[arg(long)]
        overwrite: bool,
        /// Number of unique strings sent in each request batch.
        #[arg(long)]
        batch_size: Option<usize>,
    },
    /// Write translations back into lang JSON and snbt files.
    Apply {
        #[arg(long)]
        project: PathBuf,
    },
    /// Repack the modpack zip with translated files.
    Build {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Show project status and translation progress.
    Status {
        #[arg(long)]
        project: PathBuf,
    },
    /// extract -> scan -> translate -> apply -> build
    Run {
        zip: PathBuf,
        #[arg(long, default_value = "work")]
        work: PathBuf,
        #[arg(long, default_value = "zh_tw")]
        target: String,
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        service: String,
        #[arg(long, default_value = "google_free")]
        provider: String,
        #[arg(long, default_value = "")]
        model: String,
        #[arg(long)]
        limit: Option<usize>,
        /// Number of unique strings sent in each request batch.
        #[arg(long)]
        batch_size: Option<usize>,
        /// Re-translate strings even when a previous translation exists.
        #[arg(long)]
        overwrite: bool,
        /// Skip machine translation (reuse existing translations.jsonl).
        #[arg(long)]
        skip_translate: bool,
    },
}

fn main() {
    if let Err(err) = real_main() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Extract { zip, work, target } => {
            let project = Project::create(&zip, &work, &target)?;
            remember(&project)?;
            print_project(&project);
        }
        Commands::Scan { project } => {
            let mut project = Project::open(&project)?;
            let report = qtrans_core::scan::scan(&mut project)?;
            println!(
                "scanned: {} lang + {} inline + {} script = {} strings ({} chars)",
                report.stats.lang_units,
                report.stats.inline_units,
                report.stats.script_units,
                report.stats.total_units,
                report.stats.total_chars
            );
            println!("strings:  {}", project.strings_path().display());
            println!("review:   {}", project.dir.join("strings.csv").display());
        }
        Commands::Translate {
            project,
            service,
            provider,
            model,
            target,
            limit,
            overwrite,
            batch_size,
        } => {
            let mut project = Project::open(&project)?;
            if let Some(service) = service {
                project.config.service = service;
            }
            if let Some(provider) = provider {
                project.config.provider = provider;
            }
            if let Some(model) = model {
                project.config.model = model;
            }
            if let Some(target) = target {
                project.config.target = target.clone();
                project.config.service_lang = service_lang_for(&target);
            }
            if let Some(batch_size) = batch_size {
                project.config.batch_size = batch_size.max(1);
            }
            project.save_config()?;
            let report = translate(
                &project,
                limit.or(project.config.translation_limit.filter(|limit| *limit > 0)),
                overwrite || project.config.overwrite,
            )?;
            println!(
                "translated {} strings via {} ({} reused, {} failed)",
                report.translated, report.provider, report.reused, report.failed
            );
        }
        Commands::Apply { project } => {
            let mut project = Project::open(&project)?;
            let report = qtrans_core::apply::apply(&mut project)?;
            println!(
                "applied {} translations: {} lang + {} snbt + {} KubeJS script files changed",
                report.translated, report.lang_files, report.snbt_files, report.javascript_files
            );
            println!("missing translations: {}", report.missing);
            println!("changed files: {}", report.changed.len());
        }
        Commands::Build { project, out } => {
            let project = Project::open(&project)?;
            let out = out.unwrap_or_else(|| default_output(&project));
            let report = qtrans_core::package::build_zip(&project, &out)?;
            println!(
                "built {} ({} entries, {} replaced, {} added)",
                report.output_zip, report.entries, report.replaced, report.added
            );
            let overrides = out.with_file_name(format!(
                "{}-overrides",
                out.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("translated")
            ));
            let count = qtrans_core::package::export_overrides(&project, &overrides)?;
            println!(
                "overrides folder: {} ({} files)",
                overrides.display(),
                count
            );
        }
        Commands::Status { project } => {
            let project = Project::open(&project)?;
            status(&project)?;
        }
        Commands::Run {
            zip,
            work,
            target,
            service,
            provider,
            model,
            limit,
            batch_size,
            skip_translate,
            overwrite,
        } => {
            println!("== extract ==");
            let mut project = Project::create(&zip, &work, &target)?;
            project.config.service = service;
            project.config.provider = provider;
            project.config.model = model;
            project.config.target = target.clone();
            project.config.service_lang = service_lang_for(&target);
            project.config.translation_limit = limit;
            if let Some(batch_size) = batch_size {
                project.config.batch_size = batch_size.max(1);
            }
            project.config.overwrite = overwrite;
            project.save_config()?;

            println!("== scan ==");
            let scan = qtrans_core::scan::scan(&mut project)?;
            println!(
                "{} lang + {} inline + {} script = {} strings ({} chars)",
                scan.stats.lang_units,
                scan.stats.inline_units,
                scan.stats.script_units,
                scan.stats.total_units,
                scan.stats.total_chars
            );

            if !skip_translate {
                println!("== translate ==");
                let report = translate(
                    &project,
                    limit.or(project.config.translation_limit.filter(|limit| *limit > 0)),
                    project.config.overwrite,
                )?;
                println!(
                    "{} translated via {} ({} reused, {} failed)",
                    report.translated, report.provider, report.reused, report.failed
                );
            }

            println!("== apply ==");
            let report = qtrans_core::apply::apply(&mut project)?;
            println!(
                "{} translations applied, {} missing, {} files changed",
                report.translated,
                report.missing,
                report.changed.len()
            );

            println!("== build ==");
            let out = default_output(&project);
            let report = qtrans_core::package::build_zip(&project, &out)?;
            println!("built {}", report.output_zip);
            let overrides = out.with_file_name(format!(
                "{}-overrides",
                out.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("translated")
            ));
            let count = qtrans_core::package::export_overrides(&project, &overrides)?;
            println!(
                "overrides folder: {} ({} files)",
                overrides.display(),
                count
            );
            remember(&project)?;
        }
    }
    Ok(())
}

fn print_project(project: &Project) {
    println!("project:  {}", project.dir.display());
    println!("zip:      {}", project.manifest.source_zip);
    println!("prefix:   {:?}", project.manifest.zip_prefix);
    println!("target:   {}", project.manifest.target_lang);
}

fn remember(project: &Project) -> Result<()> {
    let marker = Path::new("work").join("last_project.txt");
    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(marker, project.dir.to_string_lossy().as_bytes())?;
    Ok(())
}

fn default_output(project: &Project) -> PathBuf {
    let name = format!(
        "{}-{}.zip",
        project.manifest.pack_name, project.manifest.target_lang
    );
    project.dir.join("output").join(name)
}

fn service_lang_for(target: &str) -> String {
    if target.starts_with("zh_cn") || target == "zh" {
        "zh-CN".to_string()
    } else if target.starts_with("zh_tw") || target.starts_with("zh_hk") {
        "zh-TW".to_string()
    } else {
        target.replace('_', "-")
    }
}

fn status(project: &Project) -> Result<()> {
    println!("project: {}", project.dir.display());
    println!("pack:    {}", project.manifest.pack_name);
    println!("zip:     {}", project.manifest.source_zip);
    let stats = &project.manifest.stats;
    println!(
        "strings: {} lang + {} inline + {} script = {} ({} chars)",
        stats.lang_units,
        stats.inline_units,
        stats.script_units,
        stats.total_units,
        stats.total_chars
    );
    let translations = read_translations(project)?;
    println!("translated: {}", translations.len());
    if stats.total_units > 0 {
        let pct = translations.len() as f64 * 100.0 / stats.total_units as f64;
        println!("progress: {pct:.1}%");
    }
    println!("changed files: {}", project.manifest.changed.len());
    Ok(())
}

// ---------------------------------------------------------------- translate

#[derive(Serialize)]
struct TranslateRequest<'a> {
    provider: &'a str,
    model: &'a str,
    target: &'a str,
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

struct TranslateReport {
    translated: usize,
    reused: usize,
    failed: usize,
    provider: String,
}

fn translate(project: &Project, limit: Option<usize>, overwrite: bool) -> Result<TranslateReport> {
    let units = qtrans_core::apply::read_units(project)?;
    let existing = read_translations(project)?;
    let glossary = load_glossary(&project.dir.join("glossary.csv"))?;

    // Reuse translations for identical source strings.
    let mut cached_by_text: HashMap<String, String> = HashMap::new();
    for unit in &units {
        if let Some(text) = existing.get(&unit.id) {
            cached_by_text.insert(unit.source.clone(), text.clone());
        }
    }

    let mut pending: Vec<&Unit> = units
        .iter()
        .filter(|u| overwrite || !existing.contains_key(&u.id))
        .filter(|u| !u.source.trim().is_empty())
        .collect();
    if let Some(limit) = limit {
        pending.truncate(limit);
    }

    // group units by text so the same English line is only sent once
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
                    .map(|u| u.context.as_str())
                    .unwrap_or(""),
            });
            local.push((id, by_text[*text].clone()));
        }

        if !request_items.is_empty() {
            let request = TranslateRequest {
                provider: &project.config.provider,
                model: &project.config.model,
                target: &project.config.service_lang,
                glossary: glossary.clone(),
                items: request_items,
            };
            let response = call_service(&project.config.service, &request)?;
            let mut by_id: HashMap<&str, (&str, bool)> = HashMap::new();
            for item in &response.items {
                let ok = item.status.as_deref().unwrap_or("ok") == "ok";
                by_id.insert(item.id.as_str(), (item.text.as_str(), ok));
            }
            for (id, units) in &local {
                let Some((text, ok)) = by_id.get(id.as_str()) else {
                    eprintln!("warning: service returned no text for id {id}");
                    failed += units.len();
                    continue;
                };
                if !*ok {
                    failed += units.len();
                    continue;
                }
                write_units(&mut out, units, text, &project.config.provider)?;
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

fn call_service(service: &str, request: &TranslateRequest) -> Result<TranslateResponse> {
    let url = format!("{}/v1/translate", service.trim_end_matches('/'));
    let mut last_error = String::new();
    for attempt in 1..=3 {
        match ureq::post(&url)
            .timeout(Duration::from_secs(600))
            .send_json(request)
        {
            Ok(response) => match response.into_json::<TranslateResponse>() {
                Ok(parsed) => return Ok(parsed),
                Err(err) => last_error = format!("invalid response JSON: {err}"),
            },
            Err(ureq::Error::Status(code, response)) => {
                let body = response.into_string().unwrap_or_default();
                last_error = format!("HTTP {code}: {body}");
            }
            Err(err) => last_error = format!("{err}"),
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
        if term.is_empty() || translation.is_empty() {
            continue;
        }
        entries.push(GlossaryEntry {
            term: term.to_string(),
            translation: translation.to_string(),
        });
    }
    Ok(entries)
}

#[allow(dead_code)]
fn summarize_units(units: &[Unit]) -> (usize, usize) {
    (
        units.iter().filter(|u| u.kind == UnitKind::Lang).count(),
        units.iter().filter(|u| u.kind == UnitKind::Inline).count(),
    )
}

#[allow(dead_code)]
fn open_project(path: &Path) -> Result<Project> {
    Project::open(path).with_context(|| format!("open project {}", path.display()))
}
