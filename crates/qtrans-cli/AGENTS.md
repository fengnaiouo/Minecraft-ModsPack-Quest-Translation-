# qtrans-cli 指引

本檔只補充 `crates/qtrans-cli` 的規則；共通規則見根目錄 `AGENTS.md`。

- `cli.rs` 定義 Clap 命令與參數；`commands.rs` 協調核心操作；`translation.rs` 負責翻譯服務呼叫。
- 修改子命令或旗標時，核對 README、GUI 呼叫方式及既有命令相容性。
- CLI 的進度與日誌可能由 GUI 解析；修改輸出格式時同步檢查 GUI parser 及測試。
- 維持翻譯結果的續跑與寫入行為；overwrite 必須與 Go 服務的 `force` 快取語意一致。
- 服務測試使用本機 HTTP fixture 或 mock，不連線外部服務，不使用真實 API 金鑰。
- 驗證：`cargo test -p qtrans-cli`；相關時執行 `cargo clippy -p qtrans-cli --all-targets -- -D warnings`。
