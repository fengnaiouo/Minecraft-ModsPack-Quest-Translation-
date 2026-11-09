use super::paths::app_root;
use super::QTransApp;
use gpui::Context;
use qtrans_core::project::Project;
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

    pub(super) fn refresh(&mut self) -> bool {
        let Some(dir) = self.project_dir.clone() else {
            return false;
        };
        match Project::open(&dir) {
            Ok(project) => {
                let units = match qtrans_core::apply::read_units(&project) {
                    Ok(units) => units,
                    Err(error) => {
                        self.log.push(format!("讀取可校訂文字失敗：{error:#}"));
                        self.status = "專案文字載入失敗".to_string();
                        return false;
                    }
                };
                let translations = match qtrans_core::apply::read_translations(&project) {
                    Ok(translations) => translations,
                    Err(error) => {
                        self.log.push(format!("讀取翻譯記錄失敗：{error:#}"));
                        self.status = "翻譯記錄載入失敗".to_string();
                        return false;
                    }
                };
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
                self.review_entries = units
                    .into_iter()
                    .map(|unit| {
                        let translation = translations.get(&unit.id).cloned();
                        super::ReviewEntry { unit, translation }
                    })
                    .collect();
                self.translated = self
                    .review_entries
                    .iter()
                    .filter(|entry| entry.translation.is_some())
                    .count();
                self.review_index = self
                    .review_index
                    .min(self.review_entries.len().saturating_sub(1));
                self.review_translation_draft = self
                    .review_entries
                    .get(self.review_index)
                    .and_then(|entry| entry.translation.clone())
                    .unwrap_or_default();
                true
            }
            Err(error) => {
                self.log
                    .push(format!("無法載入專案 {}：{error:#}", dir.display()));
                self.status = "專案載入失敗".to_string();
                false
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
            self.project_dir = None;
            self.pack_name = stem.to_string();
            self.stats = Default::default();
            self.translated = 0;
            self.translated_this_run = 0;
            self.reused_this_run = 0;
            self.failed_this_run = 0;
            self.review_entries.clear();
            self.review_index = 0;
            self.review_translation_draft.clear();
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
                let previous_project = self.project_dir.clone();
                self.project_dir = Some(dir.clone());
                self.log.push(format!("已開啟專案：{}", dir.display()));
                if self.refresh() {
                    self.review_index = 0;
                    self.review_translation_draft = self
                        .review_entries
                        .first()
                        .and_then(|entry| entry.translation.clone())
                        .unwrap_or_default();
                    self.translated_this_run = 0;
                    self.reused_this_run = 0;
                    self.failed_this_run = 0;
                    self.status = "專案已載入".to_string();
                } else {
                    self.project_dir = previous_project;
                    if self.project_dir.is_some() {
                        self.refresh();
                    }
                }
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
                    "$QTRANS_PROJECT_DIR".into(),
                ],
            ],
            cx,
        );
    }
}
