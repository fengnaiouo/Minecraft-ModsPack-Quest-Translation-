use crate::project::Project;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub struct BuildReport {
    pub output_zip: String,
    pub entries: usize,
    pub replaced: usize,
    pub added: usize,
}

/// Repack the original modpack zip with every changed file applied.
pub fn build_zip(project: &Project, out: &Path) -> Result<BuildReport> {
    let source = fs::File::open(&project.manifest.source_zip)
        .with_context(|| format!("open {}", project.manifest.source_zip))?;
    let mut archive = ZipArchive::new(BufReader::new(source))?;

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let out_file = fs::File::create(out)?;
    let mut writer = ZipWriter::new(out_file);

    let changed: HashSet<String> = project.manifest.changed.iter().cloned().collect();
    let extracted = project.extracted();
    let mut written: HashSet<String> = HashSet::new();
    let mut replaced = 0usize;
    let mut added = 0usize;
    let mut entries = 0usize;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name()?.replace('\\', "/");
        entries += 1;
        if entry.is_dir() {
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            writer.add_directory(name, options)?;
            continue;
        }
        let rel = name
            .strip_prefix(&project.manifest.zip_prefix)
            .unwrap_or(&name)
            .to_string();
        let method = match entry.compression() {
            CompressionMethod::Stored => CompressionMethod::Stored,
            _ => CompressionMethod::Deflated,
        };
        let options = SimpleFileOptions::default().compression_method(method);
        writer.start_file(name.clone(), options)?;
        if changed.contains(&rel) {
            let disk = extracted.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
            let data =
                fs::read(&disk).with_context(|| format!("read changed file {}", disk.display()))?;
            writer.write_all(&data)?;
            replaced += 1;
        } else {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            writer.write_all(&buf)?;
        }
        written.insert(name);
    }

    // add files that did not exist in the source zip (e.g. zh_tw.json)
    for rel in &project.manifest.changed {
        let zip_name = format!("{}{}", project.manifest.zip_prefix, rel);
        if written.contains(&zip_name) {
            continue;
        }
        let disk = extracted.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !disk.exists() {
            continue;
        }
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer.start_file(zip_name, options)?;
        writer.write_all(&fs::read(&disk)?)?;
        added += 1;
    }

    writer.finish()?;
    Ok(BuildReport {
        output_zip: out.to_string_lossy().to_string(),
        entries,
        replaced,
        added,
    })
}

/// Copy only the changed files into a folder, for users who want to drop them
/// into an existing installation instead of replacing the whole zip.
pub fn export_overrides(project: &Project, dest: &Path) -> Result<usize> {
    let extracted = project.extracted();
    let mut count = 0;
    for rel in &project.manifest.changed {
        let src = extracted.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !src.exists() {
            continue;
        }
        let target = dest.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&src, &target)?;
        count += 1;
    }
    Ok(count)
}
