use super::QTransApp;
use gpui::Context;
use qtrans_core::project::Project;

impl QTransApp {
    pub(super) fn select_review_entry(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.review_is_dirty() {
            self.status = "請先儲存目前校訂，避免遺失修改".to_string();
            cx.notify();
            return;
        }
        if index >= self.review_entries.len() {
            return;
        }
        self.review_index = index;
        self.review_translation_draft = self.review_entries[index]
            .translation
            .clone()
            .unwrap_or_default();
        self.cursor = self.review_translation_draft.len();
        self.select_all = false;
        self.status = format!("校訂項目 {} / {}", index + 1, self.review_entries.len());
        cx.notify();
    }

    pub(super) fn save_review_translation(&mut self, cx: &mut Context<Self>) {
        let Some(entry) = self.review_entries.get(self.review_index) else {
            self.status = "目前沒有可校訂的文字".to_string();
            cx.notify();
            return;
        };
        let Some(project_dir) = self.project_dir.clone() else {
            self.status = "請先開啟或建立專案".to_string();
            cx.notify();
            return;
        };
        match Project::open(&project_dir).and_then(|project| {
            qtrans_core::apply::save_manual_translation(
                &project,
                &entry.unit,
                &self.review_translation_draft,
            )
        }) {
            Ok(()) => {
                let entry = &mut self.review_entries[self.review_index];
                let was_untranslated = entry.translation.is_none();
                entry.translation = Some(self.review_translation_draft.clone());
                if was_untranslated {
                    self.translated = self.translated.saturating_add(1);
                }
                self.status = "人工校訂已儲存".to_string();
                self.log
                    .push(format!("已儲存人工校訂：{}", entry.unit.context.as_str()));
            }
            Err(error) => {
                self.log.push(format!("儲存人工校訂失敗：{error:#}"));
                self.status = "校訂儲存失敗".to_string();
            }
        }
        cx.notify();
    }

    pub(super) fn review_counts(&self) -> (usize, usize) {
        let translated = self
            .review_entries
            .iter()
            .filter(|entry| entry.translation.is_some())
            .count();
        (translated, self.review_entries.len())
    }

    pub(super) fn review_is_dirty(&self) -> bool {
        self.review_entries
            .get(self.review_index)
            .and_then(|entry| entry.translation.as_deref())
            .unwrap_or_default()
            != self.review_translation_draft.as_str()
    }
}
