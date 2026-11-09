---
name: QTrans Rust
description: Rust workspace guidance for QTrans.
applyTo: "crates/**/*.rs"
---

# Rust workspace 指引

適用於 `crates/**/*.rs`。通用限制見根目錄 `AGENTS.md`，並閱讀目標 crate 的 `AGENTS.md`。

- `qtrans-core` 維護共用 ZIP 專案、掃描、格式解析、穩定 ID、回寫與封裝。
- `qtrans-cli` 維護命令列介面與翻譯服務呼叫；`qtrans-gui` 維護 GPUI 操作流程，不複製核心檔案處理。
- SNBT／KubeJS 回寫只替換掃描所得跨度；不得破壞未翻譯位元組、程式碼或註解。
- 修改序列化資料、CLI 輸出或 Rust／Go 協定時，檢查所有呼叫端並新增相容性測試。
- 測試使用臨時目錄、本機 fixture 或 mock；不可呼叫真實翻譯 API，也不可動用使用者模組包。
- 驗證優先執行目標 crate 的測試與 Clippy；跨 crate 行為再執行 workspace 驗證。格式只用 `cargo fmt --all -- --check` 檢查。
