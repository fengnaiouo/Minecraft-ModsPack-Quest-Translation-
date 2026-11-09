# qtrans-gui 指引

本檔只補充 `crates/qtrans-gui` 的規則；共通規則見根目錄 `AGENTS.md`。

- 使用 workspace 中的 GPUI；應用程式入口位於 `src/main.rs`，app 狀態與操作分布於 `src/app/`。
- `src/app/ui/` 負責介面呈現；設定、路徑、專案、流程、子程序與更新邏輯放在相應模組。
- 重要操作需呈現清楚的繁體中文名稱、狀態、進度及錯誤；不得把失敗呈現成成功。
- 版面調整須符合使用者要求，並考慮視窗寬度、高度與可操作性；不得擅自改變導覽或工作流程。
- ZIP、網路、檔案 I/O 與子程序等待不得阻塞 UI；明確處理啟動失敗、非零結束與取消。
- API 金鑰不得寫入設定檔或日誌；啟動服務時僅透過既有安全方式傳遞。
- CLI 輸出或工作流程修改時，補上 GUI parser、畫面呈現或流程測試。
- 驗證：`cargo test -p qtrans-gui`；相關時執行 `cargo clippy -p qtrans-gui --all-targets -- -D warnings`。
