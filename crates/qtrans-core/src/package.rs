use crate::project::{safe_project_path, Project};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
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
    let (temp_output, out_file) = TempOutput::create(&output_canonical)?;
    let source = fs::File::open(&source_canonical)
        .with_context(|| format!("open {}", project.manifest.source_zip))?;
    let mut archive = ZipArchive::new(BufReader::new(source))?;

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
            let disk = safe_project_path(&extracted, &rel)?;
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
        let disk = safe_project_path(&extracted, rel)?;
        if !disk.exists() {
            continue;
        }
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer.start_file(zip_name, options)?;
        writer.write_all(&fs::read(&disk)?)?;
        added += 1;
    }

    writer.finish()?.sync_all()?;
    install_output(temp_output.path(), &output_canonical)?;
    temp_output.keep();
    Ok(BuildReport {
        output_zip: output_canonical.to_string_lossy().to_string(),
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
        let src = safe_project_path(&extracted, rel)?;
        if !src.exists() {
            continue;
        }
        let target = safe_project_path(dest, rel)?;
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

struct TempOutput {
    path: PathBuf,
    keep: bool,
}

impl TempOutput {
    fn create(output: &Path) -> Result<(Self, fs::File)> {
        let parent = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let file_name = output
            .file_name()
            .context("output ZIP path must include a file name")?
            .to_string_lossy();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before UNIX epoch")?
            .as_nanos();

        for attempt in 0..100u32 {
            let path = parent.join(format!(
                ".{file_name}.qtrans-{}-{nonce}-{attempt}.tmp",
                std::process::id()
            ));
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(file) => {
                    return Ok((Self { path, keep: false }, file));
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("create temporary output {}", path.display()));
                }
            }
        }
        anyhow::bail!("could not reserve a unique temporary output file")
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn keep(mut self) {
        self.keep = true;
    }
}

impl Drop for TempOutput {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn install_output(temp: &Path, output: &Path) -> Result<()> {
    if !output.exists() {
        return fs::rename(temp, output)
            .with_context(|| format!("install output ZIP at {}", output.display()));
    }

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX epoch")?
        .as_nanos();
    let file_name = output
        .file_name()
        .context("output ZIP path must include a file name")?
        .to_string_lossy();
    let mut backup = None;
    for attempt in 0..100u32 {
        let candidate = output.with_file_name(format!(
            ".{file_name}.qtrans-backup-{}-{nonce}-{attempt}.tmp",
            std::process::id()
        ));
        if !candidate.exists() {
            match fs::rename(output, &candidate) {
                Ok(()) => {
                    backup = Some(candidate);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("preserve existing output ZIP {}", output.display())
                    });
                }
            }
        }
    }
    let backup = backup.context("could not reserve a backup path for the existing output ZIP")?;

    if let Err(install_error) = fs::rename(temp, output) {
        return match fs::rename(&backup, output) {
            Ok(()) => Err(install_error)
                .with_context(|| format!("install output ZIP at {}", output.display())),
            Err(restore_error) => Err(anyhow::anyhow!(
                "install output ZIP failed: {install_error}; restoring the previous output also failed: {restore_error}; previous output is preserved at {}",
                backup.display()
            )),
        };
    }

    fs::remove_file(&backup).with_context(|| {
        format!(
            "new output was installed, but could not remove previous output backup {}",
            backup.display()
        )
    })?;
    Ok(())
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

    #[test]
    fn refuses_unsafe_paths_from_project_manifest() {
        use zip::write::SimpleFileOptions;
        use zip::ZipWriter;

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("qtrans-package-path-test-{nonce}"));
        let project_dir = root.join("project");
        let output_dir = root.join("overrides");
        fs::create_dir_all(project_dir.join("extracted")).unwrap();
        let source_zip = root.join("source.zip");
        let mut writer = ZipWriter::new(fs::File::create(&source_zip).unwrap());
        writer
            .start_file("notes.txt", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"source").unwrap();
        writer.finish().unwrap();
        let project = Project {
            dir: project_dir,
            manifest: Manifest {
                pack_name: "pack".into(),
                source_zip: source_zip.to_string_lossy().into_owned(),
                zip_prefix: String::new(),
                target_lang: "zh_tw".into(),
                created_at: String::new(),
                updated_at: String::new(),
                stats: Stats::default(),
                changed: vec!["../../outside.txt".into()],
            },
            config: ProjectConfig::default(),
        };
        fs::write(root.join("existing.zip"), b"keep existing output").unwrap();

        assert!(export_overrides(&project, &output_dir).is_err());
        assert!(build_zip(&project, &root.join("existing.zip")).is_err());
        assert_eq!(
            fs::read(root.join("existing.zip")).unwrap(),
            b"keep existing output"
        );
        assert!(!root.join("outside.txt").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
