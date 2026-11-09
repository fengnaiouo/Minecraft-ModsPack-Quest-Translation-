use anyhow::Result;
use clap::Parser;
use qtrans_core::apply::read_translations;
use qtrans_core::project::Project;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::{Cli, Commands};
use crate::translation::translate;

pub fn real_main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Extract { zip, work, target } => {
            let project = Project::create(&zip, &work, &target)?;
            remember(&project)?;
            print_project(&project)?;
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
            qtrans_core::model::validate_target_code(&project.config.target)?;
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
            anyhow::ensure!(
                report.failed == 0,
                "{} translation units failed; see the log and retry before applying",
                report.failed
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
            let overrides = overrides_path(&out);
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
            print_project(&project)?;
            remember(&project)?;
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
                anyhow::ensure!(
                    report.failed == 0,
                    "{} translation units failed; stopping before apply and build",
                    report.failed
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
            let overrides = overrides_path(&out);
            let count = qtrans_core::package::export_overrides(&project, &overrides)?;
            println!(
                "overrides folder: {} ({} files)",
                overrides.display(),
                count
            );
        }
    }
    Ok(())
}

fn print_project(project: &Project) -> Result<()> {
    let path = project.dir.canonicalize()?;
    println!("project: {}", path.display());
    println!("zip:      {}", project.manifest.source_zip);
    println!("prefix:   {:?}", project.manifest.zip_prefix);
    println!("target:   {}", project.manifest.target_lang);
    Ok(())
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

fn overrides_path(out: &Path) -> PathBuf {
    out.with_file_name(format!(
        "{}-overrides",
        out.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("translated")
    ))
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
