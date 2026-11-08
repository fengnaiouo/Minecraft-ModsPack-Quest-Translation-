mod config;
mod paths;
mod process;
mod project;
mod ui;
mod updates;
mod workflow;

use config::ConfigField;
use gpui::{Context, FocusHandle, ScrollHandle};
use process::Job;
use qtrans_core::model::Stats;
use std::path::PathBuf;
use updates::{UpdateJob, UpdateStatus};

pub(crate) struct QTransApp {
    pub(in crate::app) zip_path: Option<PathBuf>,
    pub(in crate::app) project_dir: Option<PathBuf>,
    pub(in crate::app) pack_name: String,
    pub(in crate::app) stats: Stats,
    pub(in crate::app) translated: usize,
    pub(in crate::app) batch_progress: Option<(usize, usize)>,
    pub(in crate::app) provider: String,
    pub(in crate::app) service: String,
    pub(in crate::app) model: String,
    pub(in crate::app) openai_base_url: String,
    pub(in crate::app) openai_api_key: String,
    pub(in crate::app) deepl_api_key: String,
    pub(in crate::app) target: String,
    pub(in crate::app) limit: String,
    pub(in crate::app) batch_size: String,
    pub(in crate::app) overwrite: bool,
    pub(in crate::app) focus_handle: FocusHandle,
    pub(in crate::app) editing: Option<ConfigField>,
    pub(in crate::app) cursor: usize,
    pub(in crate::app) select_all: bool,
    pub(in crate::app) show_settings: bool,
    pub(in crate::app) log: Vec<String>,
    pub(in crate::app) log_scroll: ScrollHandle,
    pub(in crate::app) follow_logs: bool,
    pub(in crate::app) status: String,
    pub(in crate::app) busy: bool,
    pub(in crate::app) job: Option<Job>,
    pub(in crate::app) update_status: UpdateStatus,
    pub(in crate::app) update_job: Option<UpdateJob>,
}

impl QTransApp {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        let mut app = Self {
            zip_path: None,
            project_dir: None,
            pack_name: String::new(),
            stats: Stats::default(),
            translated: 0,
            batch_progress: None,
            provider: "google_free".to_string(),
            service: "http://127.0.0.1:8787".to_string(),
            model: std::env::var("OPENAI_MODEL").unwrap_or_default(),
            openai_base_url: std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            openai_api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            deepl_api_key: std::env::var("DEEPL_API_KEY").unwrap_or_default(),
            target: "zh_tw".to_string(),
            limit: String::new(),
            batch_size: "16".to_string(),
            overwrite: false,
            focus_handle: cx.focus_handle(),
            editing: None,
            cursor: 0,
            select_all: false,
            show_settings: false,
            log: vec!["選擇模組包 ZIP，或開啟既有專案開始。".to_string()],
            log_scroll: ScrollHandle::new(),
            follow_logs: true,
            status: "待機".to_string(),
            busy: false,
            job: None,
            update_status: UpdateStatus::Idle,
            update_job: None,
        };
        app.restore_last_project();
        app
    }
}

fn self_status(app: &mut QTransApp, ok: bool) {
    app.status = if ok {
        "完成".to_string()
    } else {
        "失敗，請查看日誌".to_string()
    };
}
