use std::path::{Path, PathBuf};

pub(super) fn app_root() -> PathBuf {
    let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    else {
        return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    };
    if matches!(
        exe_dir.file_name().and_then(|name| name.to_str()),
        Some("debug" | "release")
    ) && exe_dir
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name == "target")
    {
        return exe_dir
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .unwrap_or(exe_dir);
    }
    exe_dir
}

pub(super) fn qtrans_exe() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in ["qtrans.exe", "qtrans"] {
                let candidate = dir.join(name);
                if candidate.exists() {
                    return candidate;
                }
            }
        }
    }
    PathBuf::from("qtrans")
}
