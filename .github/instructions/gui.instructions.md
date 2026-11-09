---
name: QTrans GPUI
description: QTrans desktop GUI guidance for GPUI code.
applyTo: "crates/qtrans-gui/**/*.rs"
---

# GPUI GUI 指引

適用於 `crates/qtrans-gui/**/*.rs`。通用限制見根目錄 `AGENTS.md`，GUI 規則見 `crates/qtrans-gui/AGENTS.md`。

- GPUI application 與視窗入口位於 `src/main.rs`；GUI 狀態與操作分布於 `src/app/`。
- UI 元件放在 `src/app/ui/`；專案、設定、工作流程、子程序及更新邏輯放在相應的 app 模組。
- ZIP、網路、檔案 I/O 與子程序等待不得阻塞 UI；錯誤須明確呈現，不可用成功狀態掩蓋。
- 保持視窗縮放下的可操作性；修改版面時一併考量窄寬與有限高度，不擅自改變使用者指定的導覽方式。
- API 金鑰不得寫入設定或日誌；更新安裝程式前保留使用者確認並驗證下載流程。
- GUI 啟動的翻譯服務須在結束時停止（既有服務除外）；更新檔下載至 `work/updates/`；一般 GUI 不提供 `mock` 供應商。
- 修改 CLI 輸出解析或流程時，同步檢查 GUI parser 與其測試。
- 驗證：`cargo test -p qtrans-gui`；相關時執行 `cargo clippy -p qtrans-gui --all-targets -- -D warnings`。
