use super::QTransApp;
use gpui::Context;

impl QTransApp {
    pub(super) fn translate(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project_dir.clone() else {
            return;
        };
        let options = match self.translation_options() {
            Ok(options) => options,
            Err(error) => {
                self.log.push(format!("設定錯誤：{error:#}"));
                self.status = "請修正翻譯設定".to_string();
                cx.notify();
                return;
            }
        };
        if let Err(error) = self.save_settings() {
            self.log.push(format!("儲存設定失敗：{error:#}"));
            self.status = "設定儲存失敗".to_string();
            cx.notify();
            return;
        }
        self.status = format!("翻譯中（{}）...", self.provider);
        let mut args = vec![
            "translate".into(),
            "--project".into(),
            project.to_string_lossy().to_string(),
        ];
        args.extend(options);
        self.start_job(vec![args], cx);
    }

    pub(super) fn apply(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project_dir.clone() else {
            return;
        };
        self.status = "回寫翻譯...".to_string();
        self.start_job(
            vec![vec![
                "apply".into(),
                "--project".into(),
                project.to_string_lossy().to_string(),
            ]],
            cx,
        );
    }

    pub(super) fn build(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project_dir.clone() else {
            return;
        };
        self.status = "打包中...".to_string();
        self.start_job(
            vec![vec![
                "build".into(),
                "--project".into(),
                project.to_string_lossy().to_string(),
            ]],
            cx,
        );
    }

    pub(super) fn run_all(&mut self, cx: &mut Context<Self>) {
        let Some(zip) = self.zip_path.clone() else {
            return;
        };
        let options = match self.translation_options() {
            Ok(options) => options,
            Err(error) => {
                self.log.push(format!("設定錯誤：{error:#}"));
                self.status = "請修正翻譯設定".to_string();
                cx.notify();
                return;
            }
        };
        self.status = "一鍵執行中...".to_string();
        let mut args = vec![
            "run".into(),
            zip.to_string_lossy().to_string(),
            "--work".into(),
            "work".into(),
        ];
        args.extend(options);
        self.start_job(vec![args], cx);
    }

    pub(super) fn translate_progress(&self) -> f64 {
        if self.stats.total_units == 0 {
            0.0
        } else {
            self.translated as f64 / self.stats.total_units as f64
        }
    }
}
