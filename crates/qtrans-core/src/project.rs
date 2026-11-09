use crate::model::{validate_target_code, Manifest, ProjectConfig, Stats};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use zip::ZipArchive;

pub struct Project {
    pub dir: PathBuf,
    pub manifest: Manifest,
    pub config: ProjectConfig,
}

impl Project {
    pub fn extracted(&self) -> PathBuf {
        self.dir.join("extracted")
    }

    pub fn strings_path(&self) -> PathBuf {
        self.dir.join("strings.jsonl")
    }

    pub fn translations_path(&self) -> PathBuf {
        self.dir.join("translations.jsonl")
    }

    pub fn config_path(&self) -> PathBuf {
        self.dir.join("config.json")
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.dir.join("manifest.json")
    }

    /// Create (or refresh) a project from a modpack zip.
    pub fn create(zip_path: &Path, work_root: &Path, target: &str) -> Result<Self> {
        validate_target_code(target)?;
        let zip_path = zip_path
            .canonicalize()
            .with_context(|| format!("locate {}", zip_path.display()))?;
        let zip_path = PathBuf::from(
            zip_path
                .to_string_lossy()
                .trim_start_matches(r"\\?\")
                .to_string(),
        );
        let file_name = zip_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("modpack");
        let slug = slugify(file_name);
        let fingerprint = source_fingerprint(&zip_path)?;
        fs::create_dir_all(work_root)
            .with_context(|| format!("create work directory {}", work_root.display()))?;
        let work_root = fs::canonicalize(work_root)
            .with_context(|| format!("resolve work directory {}", work_root.display()))?;
        let dir = work_root
            .join("projects")
            .join(format!("{slug}-{}", &fingerprint[..16]));
        let extracted = dir.join("extracted");
        if extracted.exists() {
            fs::remove_dir_all(&extracted)
                .with_context(|| format!("clear extracted project {}", extracted.display()))?;
        }
        fs::create_dir_all(&extracted)?;
        fs::create_dir_all(dir.join("output"))?;

        let file = fs::File::open(&zip_path)
            .with_context(|| format!("open zip {}", zip_path.display()))?;
        let mut archive = ZipArchive::new(BufReader::new(file))
            .with_context(|| format!("read zip {}", zip_path.display()))?;

        // Detect the wrapper folder (`overrides/` for CurseForge, empty otherwise).
        let mut prefix = String::new();
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index(i) {
                let name = entry.name()?.replace('\\', "/");
                if name.starts_with("overrides/") {
                    prefix = "overrides/".to_string();
                    break;
                }
            }
        }

        let mut extracted_count = 0usize;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            if entry.is_dir() {
                continue;
            }
            let name = entry.name()?.replace('\\', "/");
            let rel = match name.strip_prefix(&prefix) {
                Some(rest) => rest.to_string(),
                None => continue,
            };
            if !is_relevant(&rel) {
                continue;
            }
            if !is_safe_relative(&rel) {
                continue;
            }
            let dest = extracted.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out =
                fs::File::create(&dest).with_context(|| format!("create {}", dest.display()))?;
            std::io::copy(&mut entry, &mut out)?;
            extracted_count += 1;
        }

        let now = chrono::Local::now().to_rfc3339();
        let manifest = Manifest {
            pack_name: file_name.to_string(),
            source_zip: zip_path.to_string_lossy().to_string(),
            zip_prefix: prefix,
            target_lang: target.to_string(),
            created_at: now.clone(),
            updated_at: now,
            stats: Stats::default(),
            changed: Vec::new(),
        };
        let mut project = Self {
            dir,
            manifest,
            config: ProjectConfig {
                target: target.to_string(),
                ..ProjectConfig::default()
            },
        };
        project.save_manifest()?;
        project.save_config()?;
        project.write_default_glossary()?;
        eprintln!(
            "extracted {extracted_count} files -> {}",
            project.dir.display()
        );
        Ok(project)
    }

    pub fn open(dir: &Path) -> Result<Self> {
        let manifest_text = fs::read_to_string(dir.join("manifest.json"))
            .with_context(|| format!("open project {}", dir.display()))?;
        let manifest: Manifest = serde_json::from_str(&manifest_text)?;
        let config_path = dir.join("config.json");
        let config = if config_path.exists() {
            serde_json::from_str(&fs::read_to_string(&config_path)?)?
        } else {
            ProjectConfig::default()
        };
        Ok(Self {
            dir: dir.to_path_buf(),
            manifest,
            config,
        })
    }

    pub fn save_manifest(&mut self) -> Result<()> {
        self.manifest.updated_at = chrono::Local::now().to_rfc3339();
        let text = serde_json::to_string_pretty(&self.manifest)?;
        fs::write(self.manifest_path(), text + "\n")?;
        Ok(())
    }

    pub fn save_config(&self) -> Result<()> {
        let text = serde_json::to_string_pretty(&self.config)?;
        fs::write(self.config_path(), text + "\n")?;
        Ok(())
    }

    fn write_default_glossary(&self) -> Result<()> {
        let path = self.dir.join("glossary.csv");
        if path.exists() {
            return Ok(());
        }
        let mut out = fs::File::create(path)?;
        out.write_all(include_bytes!("../assets/glossary.csv"))?;
        Ok(())
    }

    pub fn read_zip_entry(&self, rel: &str) -> Result<Vec<u8>> {
        let file = fs::File::open(&self.manifest.source_zip)?;
        let mut archive = ZipArchive::new(BufReader::new(file))?;
        let name = format!("{}{}", self.manifest.zip_prefix, rel);
        let mut entry = archive
            .by_name(&name)
            .with_context(|| format!("zip entry missing: {name}"))?;
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        Ok(buf)
    }

    /// Restore files from the pristine source zip before applying translations.
    /// `apply` is idempotent because every run starts from unmodified SNBT.
    pub fn restore_from_zip(&self, rels: &[String]) -> Result<usize> {
        let file = fs::File::open(&self.manifest.source_zip)?;
        let mut archive = ZipArchive::new(BufReader::new(file))?;
        let extracted = self.extracted();
        let mut restored = 0usize;
        for rel in rels {
            let name = format!("{}{}", self.manifest.zip_prefix, rel);
            let mut entry = archive
                .by_name(&name)
                .with_context(|| format!("restore source ZIP entry missing: {name}"))?;
            let dest = extracted.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut out = fs::File::create(&dest)?;
            std::io::copy(&mut entry, &mut out)?;
            restored += 1;
        }
        Ok(restored)
    }
}

fn source_fingerprint(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("open source ZIP for fingerprint {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || !ch.is_ascii() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "modpack".to_string()
    } else {
        trimmed
    }
}

fn is_relevant(rel: &str) -> bool {
    if rel.starts_with("config/ftbquests/") {
        return rel.ends_with(".snbt");
    }
    if let Some(rest) = rel.strip_prefix("kubejs/") {
        if rest.starts_with("client_scripts/")
            || rest.starts_with("server_scripts/")
            || rest.starts_with("startup_scripts/")
        {
            return true;
        }
        if rest.starts_with("assets/") {
            let parts: Vec<&str> = rest.split('/').collect();
            // assets/<mod>/lang/<file>.json
            if parts.len() >= 4 && parts[2] == "lang" && parts[3].ends_with(".json") {
                return true;
            }
        }
    }
    false
}

fn is_safe_relative(rel: &str) -> bool {
    let path = Path::new(rel);
    path.components().all(|c| matches!(c, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_keeps_readable_names() {
        assert_eq!(
            slugify("All-The-Mods-9-Expert-0.7.0"),
            "All-The-Mods-9-Expert-0-7-0"
        );
        assert_eq!(slugify("ATM9 中文包"), "ATM9-中文包");
    }

    #[test]
    fn relevant_paths() {
        assert!(is_relevant("config/ftbquests/quests/chapters/welcome.snbt"));
        assert!(is_relevant("kubejs/assets/kubejs/lang/en_us.json"));
        assert!(!is_relevant("kubejs/assets/kubejs/textures/foo.png"));
        assert!(!is_relevant("mods/foo.jar"));
    }

    #[test]
    fn projects_with_same_pack_name_are_isolated_by_zip_content() {
        use std::io::Write;
        use zip::write::SimpleFileOptions;
        use zip::ZipWriter;

        let nonce = chrono::Utc::now().timestamp_nanos_opt().unwrap();
        let root = std::env::temp_dir().join(format!("qtrans-project-test-{nonce}"));
        let first = root.join("first").join("pack.zip");
        let second = root.join("second").join("pack.zip");
        fs::create_dir_all(first.parent().unwrap()).unwrap();
        fs::create_dir_all(second.parent().unwrap()).unwrap();

        for (path, text) in [(&first, "first"), (&second, "second")] {
            let file = fs::File::create(path).unwrap();
            let mut writer = ZipWriter::new(file);
            writer
                .start_file("notes.txt", SimpleFileOptions::default())
                .unwrap();
            writer.write_all(text.as_bytes()).unwrap();
            writer.finish().unwrap();
        }

        let work = root.join("work");
        let first_project = Project::create(&first, &work, "zh_tw").unwrap();
        let second_project = Project::create(&second, &work, "zh_tw").unwrap();
        assert_ne!(first_project.dir, second_project.dir);

        fs::remove_dir_all(root).unwrap();
    }
}
