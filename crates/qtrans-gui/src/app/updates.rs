use super::QTransApp;
use anyhow::{anyhow, Context, Result};
use gpui::{Context as GpuiContext, Timer};
use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const RELEASES_API: &str =
    "https://api.github.com/repos/fengnaiouo/Minecraft-ModsPack-Quest-Translation-/releases/latest";
const INSTALLER_NAME: &str = "QTrans-Setup-x64.exe";
const MAX_INSTALLER_SIZE: u64 = 300 * 1024 * 1024;
pub(super) const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub(super) enum UpdateStatus {
    Idle,
    Checking,
    NoRelease,
    UpToDate,
    Available { version: String, url: String },
    Downloading { version: String },
    ReadyToInstall { version: String, path: PathBuf },
    Error(String),
}

#[derive(Clone)]
pub(super) struct UpdateJob {
    pending: Arc<Mutex<Option<UpdateStatus>>>,
    done: Arc<AtomicBool>,
}

impl UpdateJob {
    fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(None)),
            done: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl QTransApp {
    pub(super) fn check_for_updates(&mut self, cx: &mut GpuiContext<Self>) {
        if matches!(
            &self.update_status,
            UpdateStatus::Checking | UpdateStatus::Downloading { .. }
        ) {
            return;
        }
        self.start_update_job(UpdateStatus::Checking, check_latest_release, cx);
    }

    pub(super) fn download_update(
        &mut self,
        version: String,
        url: String,
        cx: &mut GpuiContext<Self>,
    ) {
        if self.busy {
            self.update_status =
                UpdateStatus::Error("請先等待目前的翻譯工作完成，再安裝更新。".to_string());
            cx.notify();
            return;
        }
        let loading = UpdateStatus::Downloading {
            version: version.clone(),
        };
        self.start_update_job(
            loading,
            move || match download_installer(&version, &url) {
                Ok(path) => UpdateStatus::ReadyToInstall { version, path },
                Err(error) => UpdateStatus::Error(format!("下載更新失敗：{error:#}")),
            },
            cx,
        );
    }

    pub(super) fn launch_update(&mut self, path: PathBuf, cx: &mut GpuiContext<Self>) {
        if self.busy {
            self.update_status =
                UpdateStatus::Error("請先等待目前的翻譯工作完成，再安裝更新。".to_string());
            cx.notify();
            return;
        }
        match Command::new(&path).spawn() {
            Ok(_) => cx.quit(),
            Err(error) => {
                self.update_status = UpdateStatus::Error(format!("無法啟動安裝程式：{error}"));
                cx.notify();
            }
        }
    }

    fn start_update_job(
        &mut self,
        loading: UpdateStatus,
        work: impl FnOnce() -> UpdateStatus + Send + 'static,
        cx: &mut GpuiContext<Self>,
    ) {
        let job = UpdateJob::new();
        let pending = job.pending.clone();
        let done = job.done.clone();
        self.update_status = loading;
        self.update_job = Some(job.clone());

        std::thread::spawn(move || {
            let result = work();
            if let Ok(mut slot) = pending.lock() {
                *slot = Some(result);
            }
            done.store(true, Ordering::SeqCst);
        });

        self.poll_update(cx);
        cx.notify();
    }

    fn poll_update(&mut self, cx: &mut GpuiContext<Self>) {
        let Some(job) = self.update_job.clone() else {
            return;
        };
        cx.spawn(async move |this, cx| loop {
            Timer::after(Duration::from_millis(120)).await;
            let finished = match this.update(cx, |this, cx| {
                if job.done.load(Ordering::SeqCst) {
                    if let Ok(mut pending) = job.pending.lock() {
                        if let Some(status) = pending.take() {
                            this.update_status = status;
                        } else {
                            this.update_status =
                                UpdateStatus::Error("更新作業結束，但沒有收到結果。".to_string());
                        }
                    } else {
                        this.update_status = UpdateStatus::Error("無法讀取更新結果。".to_string());
                    }
                    this.update_job = None;
                }
                cx.notify();
                job.done.load(Ordering::SeqCst)
            }) {
                Ok(finished) => finished,
                Err(_) => break,
            };
            if finished {
                break;
            }
        })
        .detach();
    }
}

fn check_latest_release() -> UpdateStatus {
    match latest_release() {
        Ok(Some((version, url))) if is_newer(&version, CURRENT_VERSION) => {
            UpdateStatus::Available { version, url }
        }
        Ok(Some(_)) => UpdateStatus::UpToDate,
        Ok(None) => UpdateStatus::NoRelease,
        Err(error) => UpdateStatus::Error(format!("檢查更新失敗：{error:#}")),
    }
}

fn latest_release() -> Result<Option<(String, String)>> {
    let response = match ureq::get(RELEASES_API)
        .set("User-Agent", "QTrans")
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(20))
        .call()
    {
        Ok(response) => response,
        Err(ureq::Error::Status(404, _)) => return Ok(None),
        Err(error) => return Err(error).context("無法連線到 GitHub Releases"),
    };
    let release: Value = response
        .into_json()
        .context("GitHub 回傳的版本資訊格式不正確")?;
    let version = release
        .get("tag_name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("最新版本缺少 tag_name"))?
        .trim_start_matches('v')
        .to_string();
    parse_version(&version).ok_or_else(|| anyhow!("版本標籤格式無效：{version}"))?;

    let assets = release
        .get("assets")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("最新版本沒有安裝檔附件"))?;
    let installer = assets
        .iter()
        .find(|asset| asset.get("name").and_then(Value::as_str) == Some(INSTALLER_NAME))
        .ok_or_else(|| anyhow!("最新版本未附上 {INSTALLER_NAME}"))?;
    let url = installer
        .get("browser_download_url")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("安裝檔下載網址不存在"))?;
    let expected_prefix =
        "https://github.com/fengnaiouo/Minecraft-ModsPack-Quest-Translation-/releases/download/";
    if !url.starts_with(expected_prefix) {
        return Err(anyhow!("安裝檔網址不是此 GitHub 專案的 Release 附件"));
    }
    Ok(Some((version, url.to_string())))
}

fn download_installer(version: &str, url: &str) -> Result<PathBuf> {
    let expected_prefix =
        "https://github.com/fengnaiouo/Minecraft-ModsPack-Quest-Translation-/releases/download/";
    if !url.starts_with(expected_prefix) {
        return Err(anyhow!("拒絕從非官方專案網址下載安裝檔"));
    }
    let (major, minor, patch) =
        parse_version(version).ok_or_else(|| anyhow!("版本標籤格式無效：{version}"))?;
    let response = ureq::get(url)
        .set("User-Agent", "QTrans")
        .timeout(Duration::from_secs(120))
        .call()
        .context("無法連線下載安裝檔")?;
    if let Some(length) = response
        .header("Content-Length")
        .and_then(|length| length.parse::<u64>().ok())
    {
        if length > MAX_INSTALLER_SIZE {
            return Err(anyhow!("安裝檔超過允許的大小上限"));
        }
    }

    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_INSTALLER_SIZE + 1)
        .read_to_end(&mut bytes)
        .context("讀取安裝檔失敗")?;
    if bytes.len() as u64 > MAX_INSTALLER_SIZE {
        return Err(anyhow!("安裝檔超過允許的大小上限"));
    }
    if !bytes.starts_with(b"MZ") {
        return Err(anyhow!("下載內容不是有效的 Windows 安裝程式"));
    }

    let directory = std::env::temp_dir().join("QTrans").join("updates");
    fs::create_dir_all(&directory).context("建立更新暫存資料夾失敗")?;
    let path = directory.join(format!("QTrans-Setup-x64-{major}.{minor}.{patch}.exe"));
    let mut file = fs::File::create(&path).context("建立安裝程式檔案失敗")?;
    file.write_all(&bytes).context("儲存安裝程式失敗")?;
    Ok(path)
}

fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.split_once('-').map_or(version, |(core, _)| core);
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison_handles_release_tags() {
        assert!(is_newer("0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
        assert!(!is_newer("invalid", "0.1.0"));
    }

    #[test]
    fn version_parser_accepts_semver_release_suffix() {
        assert_eq!(parse_version("1.2.3-rc.1"), Some((1, 2, 3)));
        assert_eq!(parse_version("v1.2.3"), None);
        assert_eq!(parse_version("1.2"), None);
    }
}
