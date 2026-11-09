use super::paths::{app_root, qtrans_exe};
use super::process_output::{
    command_label, format_cli_output, format_stderr, parse_cli_progress, ProgressUpdate,
};
use super::QTransApp;
use gpui::{Context, Timer};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PROJECT_DIR_PLACEHOLDER: &str = "$QTRANS_PROJECT_DIR";

#[derive(Clone)]
pub(super) struct Job {
    pending: Arc<Mutex<Vec<ProcessMessage>>>,
    done: Arc<AtomicBool>,
    success: Arc<AtomicBool>,
    cancel_requested: Arc<AtomicBool>,
    active_child: Arc<Mutex<Option<Arc<Mutex<Child>>>>>,
    cancelable: bool,
    translated_base: usize,
    success_status: String,
    failure_status: String,
}

enum ProcessMessage {
    Log(String),
    Progress(ProgressUpdate),
    ProjectPath(std::path::PathBuf),
}

impl Job {
    fn new(
        translated_base: usize,
        success_status: impl Into<String>,
        failure_status: impl Into<String>,
        cancelable: bool,
    ) -> Self {
        Self {
            pending: Arc::new(Mutex::new(Vec::new())),
            done: Arc::new(AtomicBool::new(false)),
            success: Arc::new(AtomicBool::new(false)),
            cancel_requested: Arc::new(AtomicBool::new(false)),
            active_child: Arc::new(Mutex::new(None)),
            cancelable,
            translated_base,
            success_status: success_status.into(),
            failure_status: failure_status.into(),
        }
    }
}

impl QTransApp {
    pub(super) fn can_cancel_job(&self) -> bool {
        self.job
            .as_ref()
            .is_some_and(|job| job.cancelable && !job.done.load(Ordering::SeqCst))
    }

    pub(super) fn start_service(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.log.clear();
        let job = Job::new(self.translated, "翻譯服務已啟動", "翻譯服務啟動失敗", false);
        self.job = Some(job.clone());
        let pending = job.pending.clone();
        let done = job.done.clone();
        let success = job.success.clone();
        let openai_api_key = self.openai_api_key.clone();
        let openai_base_url = self.openai_base_url.clone();
        let deepl_api_key = self.deepl_api_key.clone();
        let service_child = self.service_child.clone();
        let service_shutdown = self.service_shutdown.clone();
        self.status = "正在啟動翻譯服務…".to_string();

        std::thread::spawn(move || {
            let result = start_translation_service(
                &openai_api_key,
                &openai_base_url,
                &deepl_api_key,
                &service_shutdown,
            );
            match result {
                Ok((already_running, child)) => {
                    if let Some(child) = child {
                        if let Ok(mut slot) = service_child.lock() {
                            *slot = Some(child);
                        }
                    }
                    if service_shutdown.load(Ordering::SeqCst) {
                        if let Ok(mut slot) = service_child.lock() {
                            if let Some(mut child) = slot.take() {
                                let _ = child.kill();
                                let _ = child.wait();
                            }
                        }
                    }
                    let message = if already_running {
                        "資訊｜翻譯服務已在執行：http://127.0.0.1:8787"
                    } else {
                        "完成｜翻譯服務已啟動：http://127.0.0.1:8787"
                    };
                    push_message(&pending, ProcessMessage::Log(message.to_string()));
                    success.store(true, Ordering::SeqCst);
                }
                Err(error) => {
                    push_message(
                        &pending,
                        ProcessMessage::Log(format!("錯誤｜翻譯服務啟動失敗：{error:#}")),
                    );
                }
            }
            done.store(true, Ordering::SeqCst);
        });

        self.poll_job(cx);
        cx.notify();
    }

    pub(super) fn open_output(&mut self) {
        if let Some(dir) = self.project_dir.as_ref() {
            let output = dir.join("output");
            let target = if output.exists() { output } else { dir.clone() };
            #[cfg(windows)]
            {
                let _ = Command::new("explorer").arg(target).spawn();
            }
        }
    }

    pub(super) fn start_job(&mut self, commands: Vec<Vec<String>>, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.log.clear();
        self.batch_progress = None;
        self.translated_this_run = 0;
        self.reused_this_run = 0;
        self.failed_this_run = 0;
        self.job = Some(Job::new(self.translated, "完成", "失敗，請查看日誌", true));
        let job = self.job.clone().expect("job was just initialized");
        let pending = job.pending.clone();
        let done = job.done.clone();
        let success = job.success.clone();
        let cancel_requested = job.cancel_requested.clone();
        let active_child = job.active_child.clone();

        std::thread::spawn(move || {
            let exe = qtrans_exe();
            let mut all_ok = true;
            let mut project_dir: Option<String> = None;
            for mut args in commands {
                if cancel_requested.load(Ordering::SeqCst) {
                    all_ok = false;
                    break;
                }
                for arg in &mut args {
                    if arg == PROJECT_DIR_PLACEHOLDER {
                        let Some(path) = &project_dir else {
                            push_message(
                                &pending,
                                ProcessMessage::Log("錯誤｜前置步驟沒有回報專案路徑".to_string()),
                            );
                            all_ok = false;
                            break;
                        };
                        *arg = path.clone();
                    }
                }
                if !all_ok {
                    break;
                }

                let label = command_label(&args);
                push_message(&pending, ProcessMessage::Log(format!("開始｜{label}")));
                let mut command = Command::new(&exe);
                command
                    .args(&args)
                    .current_dir(app_root())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    command.creation_flags(0x08000000);
                }
                let mut child = match command.spawn() {
                    Ok(child) => child,
                    Err(error) => {
                        push_message(
                            &pending,
                            ProcessMessage::Log(format!("錯誤｜無法啟動 qtrans：{error}")),
                        );
                        all_ok = false;
                        break;
                    }
                };

                let stdout = child.stdout.take();
                let stderr = child.stderr.take();
                let child = Arc::new(Mutex::new(child));
                match active_child.lock() {
                    Ok(mut active) => *active = Some(child.clone()),
                    Err(_) => {
                        push_message(
                            &pending,
                            ProcessMessage::Log(
                                "錯誤｜無法註冊作業程序，已停止以避免失控程序".to_string(),
                            ),
                        );
                        if let Ok(mut child) = child.lock() {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                        all_ok = false;
                        break;
                    }
                }
                if cancel_requested.load(Ordering::SeqCst) {
                    if let Ok(mut child) = child.lock() {
                        let _ = child.kill();
                    }
                }

                let stderr_thread = stderr.map(|stderr| {
                    let reader_pending = pending.clone();
                    std::thread::spawn(move || -> std::io::Result<()> {
                        for line in BufReader::new(stderr).lines() {
                            let line = match line {
                                Ok(line) => line,
                                Err(error) => {
                                    push_message(
                                        &reader_pending,
                                        ProcessMessage::Log(format!(
                                            "錯誤｜讀取子程序錯誤輸出失敗：{error}"
                                        )),
                                    );
                                    return Err(error);
                                }
                            };
                            push_message(
                                &reader_pending,
                                ProcessMessage::Log(format_stderr(&line)),
                            );
                        }
                        Ok(())
                    })
                });
                if let Some(stdout) = stdout {
                    for line in BufReader::new(stdout).lines() {
                        let line = match line {
                            Ok(line) => line,
                            Err(error) => {
                                push_message(
                                    &pending,
                                    ProcessMessage::Log(format!(
                                        "錯誤｜讀取子程序輸出失敗：{error}"
                                    )),
                                );
                                all_ok = false;
                                if let Ok(mut child) = child.lock() {
                                    let _ = child.kill();
                                }
                                break;
                            }
                        };
                        if let Some(path) = line
                            .strip_prefix("project:")
                            .map(str::trim)
                            .filter(|path| !path.is_empty())
                        {
                            project_dir = Some(path.to_string());
                            push_message(&pending, ProcessMessage::ProjectPath(path.into()));
                        }
                        if let Some(progress) = parse_cli_progress(&line) {
                            push_message(&pending, ProcessMessage::Progress(progress));
                        }
                        let formatted = format_cli_output(&line);
                        if !formatted.is_empty() {
                            push_message(&pending, ProcessMessage::Log(formatted));
                        }
                    }
                }
                if let Some(handle) = stderr_thread {
                    match handle.join() {
                        Ok(Ok(())) => {}
                        Ok(Err(_)) => all_ok = false,
                        Err(_) => {
                            push_message(
                                &pending,
                                ProcessMessage::Log(
                                    "錯誤｜讀取子程序錯誤輸出的背景工作異常結束".to_string(),
                                ),
                            );
                            all_ok = false;
                        }
                    }
                }

                let wait_result = match child.lock() {
                    Ok(mut child) => child.wait(),
                    Err(_) => {
                        push_message(
                            &pending,
                            ProcessMessage::Log(
                                "錯誤｜無法等待作業程序（程序鎖定狀態損壞）".to_string(),
                            ),
                        );
                        all_ok = false;
                        break;
                    }
                };
                match active_child.lock() {
                    Ok(mut active) => *active = None,
                    Err(_) => {
                        push_message(
                            &pending,
                            ProcessMessage::Log(
                                "錯誤｜無法清除已結束的作業程序控制狀態".to_string(),
                            ),
                        );
                        all_ok = false;
                    }
                }
                match wait_result {
                    Ok(status) if status.success() && !cancel_requested.load(Ordering::SeqCst) => {
                        push_message(&pending, ProcessMessage::Log(format!("完成｜{label}")));
                    }
                    Ok(_) if cancel_requested.load(Ordering::SeqCst) => {
                        push_message(
                            &pending,
                            ProcessMessage::Log(
                                "已取消｜已完成的批次已保存，可重新執行未完成翻譯".to_string(),
                            ),
                        );
                        all_ok = false;
                        break;
                    }
                    Ok(status) => {
                        push_message(
                            &pending,
                            ProcessMessage::Log(format!("錯誤｜{label}失敗（結束碼：{status}）")),
                        );
                        all_ok = false;
                        break;
                    }
                    Err(error) => {
                        push_message(
                            &pending,
                            ProcessMessage::Log(format!("錯誤｜等待作業結束失敗：{error}")),
                        );
                        all_ok = false;
                        break;
                    }
                }
            }
            success.store(all_ok, Ordering::SeqCst);
            done.store(true, Ordering::SeqCst);
        });

        self.poll_job(cx);
        cx.notify();
    }

    pub(super) fn cancel_job(&mut self, cx: &mut Context<Self>) {
        let Some(job) = self.job.clone().filter(|job| job.cancelable) else {
            return;
        };
        if job.done.load(Ordering::SeqCst) {
            return;
        }

        job.cancel_requested.store(true, Ordering::SeqCst);
        match job.active_child.lock() {
            Ok(active) => {
                if let Some(child) = active.as_ref() {
                    match child.lock() {
                        Ok(mut child) => match terminate_child(&mut child) {
                            Ok(_) => {}
                            Err(error) => {
                                self.log.push(format!("停止作業程序時發生錯誤：{error:#}"))
                            }
                        },
                        Err(_) => self
                            .log
                            .push("停止作業程序失敗：程序鎖定狀態損壞".to_string()),
                    }
                }
            }
            Err(_) => self
                .log
                .push("停止作業失敗：無法讀取程序控制狀態".to_string()),
        }
        self.log.push("已要求停止；正在結束目前步驟…".to_string());
        self.status = "正在取消…".to_string();
        cx.notify();
    }

    pub(super) fn poll_job(&mut self, cx: &mut Context<Self>) {
        let Some(job) = self.job.clone() else {
            return;
        };
        cx.spawn(async move |this, cx| loop {
            Timer::after(Duration::from_millis(140)).await;
            let finished = match this.update(cx, |this, cx| {
                this.drain(job.pending.clone(), job.translated_base);
                cx.notify();
                job.done.load(Ordering::SeqCst)
            }) {
                Ok(value) => value,
                Err(_) => break,
            };
            if finished {
                let _ = this.update(cx, |this, cx| {
                    this.busy = false;
                    let refreshed = this.project_dir.is_none() || this.refresh();
                    let ok = job.success.load(Ordering::SeqCst) && refreshed;
                    this.status = if job.cancel_requested.load(Ordering::SeqCst) && refreshed {
                        "已取消；請檢查專案狀態後再繼續".to_string()
                    } else if ok {
                        job.success_status.clone()
                    } else if refreshed {
                        job.failure_status.clone()
                    } else {
                        "專案載入失敗".to_string()
                    };
                    this.job = None;
                    cx.notify();
                });
                break;
            }
        })
        .detach();
    }

    fn drain(&mut self, pending: Arc<Mutex<Vec<ProcessMessage>>>, translated_base: usize) {
        let messages: Vec<ProcessMessage> = match pending.lock() {
            Ok(mut guard) => std::mem::take(&mut *guard),
            Err(_) => {
                self.log
                    .push("錯誤｜無法讀取背景作業訊息（鎖定狀態損壞）".to_string());
                self.status = "背景作業通訊失敗".to_string();
                return;
            }
        };
        let mut added = false;
        for message in messages {
            match message {
                ProcessMessage::Log(line) => {
                    if !line.is_empty() {
                        self.log.push(line);
                        added = true;
                    }
                }
                ProcessMessage::Progress(progress) => {
                    if let Some(stats) = progress.stats {
                        self.stats = stats;
                    }
                    if let Some(translated_units) = progress.translated_units {
                        self.translated_this_run = translated_units;
                        let translated = translated_base.saturating_add(translated_units);
                        self.translated = if self.stats.total_units == 0 {
                            translated
                        } else {
                            translated.min(self.stats.total_units)
                        };
                    }
                    if let Some(reused_units) = progress.reused_units {
                        self.reused_this_run = reused_units;
                    }
                    if let Some(failed_units) = progress.failed_units {
                        self.failed_this_run = failed_units;
                    }
                    if let Some(batch) = progress.batch {
                        self.batch_progress = Some(batch);
                    }
                }
                ProcessMessage::ProjectPath(path) => {
                    self.project_dir = Some(path);
                }
            }
        }
        if added && self.follow_logs {
            self.log_scroll.scroll_to_bottom();
        }
        if self.log.len() > 600 {
            let drop_count = self.log.len() - 600;
            self.log.drain(0..drop_count);
        }
    }
}

fn push_message(pending: &Arc<Mutex<Vec<ProcessMessage>>>, message: ProcessMessage) {
    match pending.lock() {
        Ok(mut guard) => guard.push(message),
        Err(_) => eprintln!("QTrans GUI background message queue is poisoned"),
    }
}

fn terminate_child(child: &mut Child) -> anyhow::Result<bool> {
    if child.try_wait()?.is_some() {
        return Ok(false);
    }
    match child.kill() {
        Ok(()) => Ok(true),
        Err(_) if child.try_wait()?.is_some() => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn start_translation_service(
    openai_api_key: &str,
    openai_base_url: &str,
    deepl_api_key: &str,
    shutdown: &AtomicBool,
) -> anyhow::Result<(bool, Option<Child>)> {
    if service_is_healthy()? {
        return Ok((true, None));
    }

    let root = app_root();
    let exe = root.join("bin").join("qtrans-translator.exe");
    let mut command = Command::new(&exe);
    command
        .current_dir(&root)
        .arg("-addr")
        .arg("127.0.0.1:8787")
        .arg("-cache")
        .arg("work/cache/translations.json")
        .env("OPENAI_API_KEY", openai_api_key)
        .env("OPENAI_BASE_URL", openai_base_url)
        .env("DEEPL_API_KEY", deepl_api_key)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let mut child = command.spawn()?;
    if let Err(error) = wait_for_service(&mut child) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    if shutdown.load(Ordering::SeqCst) {
        let _ = child.kill();
        let _ = child.wait();
        anyhow::bail!("應用程式已結束，取消啟動翻譯服務");
    }
    Ok((false, Some(child)))
}

fn service_is_healthy() -> anyhow::Result<bool> {
    let address = "127.0.0.1:8787"
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("無法解析翻譯服務位址"))?;
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(150)) else {
        return Ok(false);
    };
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    stream.write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")?;
    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response)?;
    Ok(response.contains(" 200 "))
}

fn wait_for_service(child: &mut Child) -> anyhow::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let address = "127.0.0.1:8787"
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("無法解析翻譯服務位址"))?;
    loop {
        if let Some(status) = child.try_wait()? {
            anyhow::bail!("服務程序已提前結束，結束狀態：{status}");
        }
        let last_error = match TcpStream::connect_timeout(&address, Duration::from_millis(150)) {
            Ok(mut stream) => {
                stream.set_read_timeout(Some(Duration::from_millis(250)))?;
                stream.write_all(
                    b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
                )?;
                let mut response = String::new();
                BufReader::new(stream).read_line(&mut response)?;
                if response.contains(" 200 ") {
                    if let Some(status) = child.try_wait()? {
                        anyhow::bail!("服務程序已提前結束，結束狀態：{status}");
                    }
                    return Ok(());
                }
                format!("健康檢查回應無效：{}", response.trim())
            }
            Err(error) => error.to_string(),
        };
        if Instant::now() >= deadline {
            anyhow::bail!("等待健康檢查逾時（{last_error}）");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;

    #[test]
    fn cancellation_terminates_a_running_child_process() {
        let mut command = if cfg!(windows) {
            let mut command = Command::new("ping");
            command.args(["-n", "3", "127.0.0.1"]);
            command
        } else {
            let mut command = Command::new("sleep");
            command.arg("2");
            command
        };
        let mut child = command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start test child process");

        assert!(terminate_child(&mut child).expect("terminate test child"));
        assert!(!child.wait().expect("wait for test child").success());
    }

    #[test]
    fn cancellation_does_not_report_termination_for_an_exited_child() {
        let mut child = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "exit", "0"])
                .spawn()
                .expect("start completed test child")
        } else {
            Command::new("true")
                .spawn()
                .expect("start completed test child")
        };
        assert!(child.wait().expect("wait for test child").success());

        assert!(!terminate_child(&mut child).expect("check completed test child"));
    }
}
