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
    let source_path = Path::new(&project.manifest.source_zip);
    let source_canonical = fs::canonicalize(source_path)
        .with_context(|| format!("open {}", project.manifest.source_zip))?;
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let output_canonical = canonical_output_path(out)?;
    anyhow::ensure!(
        !same_path(&source_canonical, &output_canonical),
        "output ZIP must not overwrite the source modpack"
    );
    let source = fs::File::open(&source_canonical)
        .with_context(|| format!("open {}", project.manifest.source_zip))?;
    let mut archive = ZipArchive::new(BufReader::new(source))?;

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

fn canonical_output_path(out: &Path) -> Result<std::path::PathBuf> {
    if out.exists() {
        return fs::canonicalize(out)
            .with_context(|| format!("resolve output ZIP {}", out.display()));
    }
    let parent = out
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = out
        .file_name()
        .context("output ZIP path must include a file name")?;
    Ok(fs::canonicalize(parent)
        .with_context(|| format!("resolve output directory {}", parent.display()))?
        .join(file_name))
}

#[cfg(windows)]
fn same_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(not(windows))]
fn same_path(left: &Path, right: &Path) -> bool {
    left == right
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Manifest, ProjectConfig, Stats};
    use crate::project::Project;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn refuses_to_overwrite_the_source_zip() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("qtrans-package-test-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("pack.zip");
        let original = b"keep original bytes";
        fs::write(&source, original).unwrap();
        let project = Project {
            dir: root.clone(),
            manifest: Manifest {
                pack_name: "pack".into(),
                source_zip: source.to_string_lossy().into_owned(),
                zip_prefix: String::new(),
                target_lang: "zh_tw".into(),
                created_at: String::new(),
                updated_at: String::new(),
                stats: Stats::default(),
                changed: Vec::new(),
            },
            config: ProjectConfig::default(),
        };

        assert!(build_zip(&project, &source).is_err());
        assert_eq!(fs::read(&source).unwrap(), original);

        fs::remove_dir_all(root).unwrap();
    }
}
