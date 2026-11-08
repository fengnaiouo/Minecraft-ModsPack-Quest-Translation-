use super::paths::{app_root, count_lines};
use super::QTransApp;
use gpui::Context;
use qtrans_core::project::{slugify, Project};
use std::fs;
use std::path::PathBuf;

impl QTransApp {
    pub(super) fn restore_last_project(&mut self) {
        let marker = app_root().join("work").join("last_project.txt");
        if let Ok(text) = fs::read_to_string(&marker) {
            let path = PathBuf::from(text.trim());
            if path.join("manifest.json").exists() {
                self.project_dir = Some(path);
                self.refresh();
            }
        }
    }

    pub(super) fn refresh(&mut self) {
        let Some(dir) = self.project_dir.clone() else {
            return;
        };
        match Project::open(&dir) {
            Ok(project) => {
                self.stats = project.manifest.stats.clone();
                self.pack_name = project.manifest.pack_name.clone();
                self.provider = project.config.provider.clone();
                self.service = project.config.service.clone();
                self.model = project.config.model.clone();
                self.target = project.config.target.clone();
                self.limit = project
                    .config
                    .translation_limit
                    .filter(|limit| *limit > 0)
                    .map(|limit| limit.to_string())
                    .unwrap_or_default();
                self.batch_size = project.config.batch_size.max(1).to_string();
                self.overwrite = project.config.overwrite;
                self.zip_path = Some(PathBuf::from(&project.manifest.source_zip));
                self.translated = count_lines(&project.translations_path());
            }
            Err(error) => {
                self.log
                    .push(format!("無法載入專案 {}：{error:#}", dir.display()));
                self.status = "專案載入失敗".to_string();
            }
        }
    }

    pub(super) fn choose_zip(&mut self, cx: &mut Context<Self>) {
        let root = app_root();
        let modpacks = root.join("modpacks");
        let mut dialog = rfd::FileDialog::new().add_filter("Modpack ZIP", &["zip"]);
        if modpacks.exists() {
            dialog = dialog.set_directory(modpacks);
        }
        if let Some(path) = dialog.pick_file() {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("modpack");
            self.project_dir = Some(root.join("work").join("projects").join(slugify(stem)));
            self.pack_name = stem.to_string();
            self.stats = Default::default();
            self.translated = 0;
            self.batch_progress = None;
            self.log.push(format!("已選擇：{}", path.display()));
            self.zip_path = Some(path);
            self.status = "準備解包".to_string();
            cx.notify();
        }
    }

    pub(super) fn open_project(&mut self, cx: &mut Context<Self>) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            if dir.join("manifest.json").exists() {
                self.project_dir = Some(dir.clone());
                self.log.push(format!("已開啟專案：{}", dir.display()));
                self.refresh();
                self.status = "專案已載入".to_string();
            } else {
                self.log.push(format!(
                    "{} 不是 QTrans 專案（缺少 manifest.json）",
                    dir.display()
                ));
            }
            cx.notify();
        }
    }

    pub(super) fn extract_and_scan(&mut self, cx: &mut Context<Self>) {
        let Some(zip) = self.zip_path.clone() else {
            return;
        };
        let Some(project) = self.project_dir.clone() else {
            return;
        };
        self.status = "解包並掃描中...".to_string();
        self.start_job(
            vec![
                vec![
                    "extract".into(),
                    zip.to_string_lossy().to_string(),
                    "--work".into(),
                    "work".into(),
                ],
                vec![
                    "scan".into(),
                    "--project".into(),
                    project.to_string_lossy().to_string(),
                ],
            ],
            cx,
        );
    }
}
