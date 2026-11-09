use super::QTransApp;
use anyhow::Context as AnyhowContext;
use gpui::{Context, KeyDownEvent, MouseDownEvent, Window};
use qtrans_core::project::Project;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ConfigField {
    Service,
    Model,
    Target,
    Limit,
    BatchSize,
    OpenAiBaseUrl,
    OpenAiKey,
    DeepLKey,
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

fn previous_char_boundary(value: &str, cursor: usize) -> usize {
    value[..cursor]
        .char_indices()
        .next_back()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_char_boundary(value: &str, cursor: usize) -> usize {
    value[cursor..]
        .char_indices()
        .nth(1)
        .map(|(offset, _)| cursor + offset)
        .unwrap_or(value.len())
}

impl QTransApp {
    pub(super) fn save_settings(&mut self) -> anyhow::Result<()> {
        let Some(dir) = self.project_dir.clone() else {
            return Ok(());
        };
        if !dir.join("manifest.json").exists() {
            return Ok(());
        }
        let mut project = Project::open(&dir)?;
        project.config.provider = self.provider.clone();
        project.config.service = self.service.clone();
        project.config.model = self.model.clone();
        project.config.target = self.target.clone();
        project.config.service_lang = service_lang_for(&self.target);
        project.config.translation_limit = if self.limit.trim().is_empty() {
            None
        } else {
            Some(
                self.limit
                    .parse::<usize>()
                    .context("翻譯上限必須是大於 0 的整數")?,
            )
        };
        anyhow::ensure!(
            project
                .config
                .translation_limit
                .is_none_or(|limit| limit > 0),
            "翻譯上限必須大於 0"
        );
        qtrans_core::model::validate_target_code(&project.config.target)?;
        project.config.batch_size = self
            .batch_size
            .parse::<usize>()
            .context("每批字串數必須是大於 0 的整數")?;
        anyhow::ensure!(project.config.batch_size > 0, "每批字串數必須大於 0");
        project.config.overwrite = self.overwrite;
        project.save_config()
    }

    pub(super) fn activate_field(
        &mut self,
        field: ConfigField,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.editing = Some(field);
        self.cursor = self.field_value(field).len();
        self.select_all = false;
        window.focus(&self.focus_handle);
        cx.notify();
    }

    pub(super) fn field_value(&self, field: ConfigField) -> &str {
        match field {
            ConfigField::Service => &self.service,
            ConfigField::Model => &self.model,
            ConfigField::Target => &self.target,
            ConfigField::Limit => &self.limit,
            ConfigField::BatchSize => &self.batch_size,
            ConfigField::OpenAiBaseUrl => &self.openai_base_url,
            ConfigField::OpenAiKey => &self.openai_api_key,
            ConfigField::DeepLKey => &self.deepl_api_key,
        }
    }

    pub(super) fn edit_field(
        &mut self,
        field: ConfigField,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editing != Some(field) {
            return;
        }
        self.editing = Some(field);
        window.focus(&self.focus_handle);
        let key = event.keystroke.key.to_ascii_lowercase();
        let modifiers = event.keystroke.modifiers;
        let shortcut = modifiers.control || modifiers.platform;
        let pasted = if shortcut && key == "v" {
            cx.read_from_clipboard().and_then(|item| item.text())
        } else {
            None
        };
        let value = match field {
            ConfigField::Service => &mut self.service,
            ConfigField::Model => &mut self.model,
            ConfigField::Target => &mut self.target,
            ConfigField::Limit => &mut self.limit,
            ConfigField::BatchSize => &mut self.batch_size,
            ConfigField::OpenAiBaseUrl => &mut self.openai_base_url,
            ConfigField::OpenAiKey => &mut self.openai_api_key,
            ConfigField::DeepLKey => &mut self.deepl_api_key,
        };
        if shortcut && key == "a" {
            self.select_all = true;
        } else if shortcut && key == "v" {
            if let Some(text) = pasted {
                let text = text.replace(['\r', '\n'], "");
                if self.select_all {
                    *value = text;
                    self.cursor = value.len();
                    self.select_all = false;
                } else {
                    value.insert_str(self.cursor, &text);
                    self.cursor += text.len();
                }
            }
        } else if key == "backspace" {
            if self.select_all {
                value.clear();
                self.cursor = 0;
                self.select_all = false;
            } else if self.cursor > 0 {
                let start = previous_char_boundary(value, self.cursor);
                value.replace_range(start..self.cursor, "");
                self.cursor = start;
            }
        } else if key == "delete" {
            if self.select_all {
                value.clear();
                self.cursor = 0;
                self.select_all = false;
            } else if self.cursor < value.len() {
                let end = next_char_boundary(value, self.cursor);
                value.replace_range(self.cursor..end, "");
            }
        } else if key == "left" {
            self.cursor = previous_char_boundary(value, self.cursor);
            self.select_all = false;
        } else if key == "right" {
            self.cursor = next_char_boundary(value, self.cursor);
            self.select_all = false;
        } else if key == "home" {
            self.cursor = 0;
            self.select_all = false;
        } else if key == "end" {
            self.cursor = value.len();
            self.select_all = false;
        } else if !modifiers.control && !modifiers.alt && !modifiers.platform {
            if let Some(text) = &event.keystroke.key_char {
                if self.select_all {
                    *value = text.clone();
                    self.cursor = value.len();
                    self.select_all = false;
                } else {
                    value.insert_str(self.cursor, text);
                    self.cursor += text.len();
                }
            }
        }
        self.status = "設定尚未儲存".to_string();
        cx.notify();
    }

    pub(super) fn translation_options(&self) -> anyhow::Result<Vec<String>> {
        anyhow::ensure!(
            self.service.starts_with("http://") || self.service.starts_with("https://"),
            "服務網址必須以 http:// 或 https:// 開頭"
        );
        qtrans_core::model::validate_target_code(&self.target)?;
        let batch_size = self
            .batch_size
            .parse::<usize>()
            .ok()
            .filter(|size| *size > 0)
            .ok_or_else(|| anyhow::anyhow!("每批字串數必須是大於 0 的整數"))?;
        let limit = if self.limit.trim().is_empty() {
            None
        } else {
            Some(
                self.limit
                    .parse::<usize>()
                    .ok()
                    .filter(|limit| *limit > 0)
                    .ok_or_else(|| anyhow::anyhow!("翻譯上限必須是大於 0 的整數，留空代表不限"))?,
            )
        };
        let mut args = vec![
            "--provider".into(),
            self.provider.clone(),
            "--service".into(),
            self.service.clone(),
            "--target".into(),
            self.target.clone(),
            "--batch-size".into(),
            batch_size.to_string(),
        ];
        if !self.model.trim().is_empty() {
            args.extend(["--model".into(), self.model.clone()]);
        }
        if let Some(limit) = limit {
            args.extend(["--limit".into(), limit.to_string()]);
        }
        if self.overwrite {
            args.push("--overwrite".into());
        }
        Ok(args)
    }

    pub(super) fn toggle_overwrite(&mut self, cx: &mut Context<Self>) {
        self.overwrite = !self.overwrite;
        self.status = "設定尚未儲存".to_string();
        cx.notify();
    }

    pub(super) fn set_page(&mut self, page: super::AppPage, cx: &mut Context<Self>) {
        if self.page == super::AppPage::Settings && page != super::AppPage::Settings {
            if let Err(error) = self.save_settings() {
                self.log.push(format!("儲存設定失敗：{error:#}"));
                self.status = "設定儲存失敗".to_string();
                cx.notify();
                return;
            }
        }
        self.page = page;
        if page == super::AppPage::Settings {
            self.status = "翻譯設定".to_string();
        }
        cx.notify();
    }

    pub(super) fn set_provider(&mut self, provider: &str, cx: &mut Context<Self>) {
        self.provider = provider.to_string();
        self.status = "設定尚未儲存".to_string();
        cx.notify();
    }
}
