# qtrans-core 指引

本檔只補充 `crates/qtrans-core` 的規則；共通資料來源、安全與驗證方式見根目錄 `AGENTS.md`。

- 此 crate 是 CLI 與 GUI 共用的核心；不得依賴 GPUI、Clap、Go 程序或 UI 狀態。
- 目前主要模組包括 `project`、`model`、`ids`、`scan`、`snbt`、`javascript`、`lang`、`apply` 與 `package`；新增責任應放入最相符的模組。
- 專案目錄以來源 ZIP 的 SHA-256 區隔；維持不同來源檔案間的資料隔離。
- ZIP 路徑必須防止路徑穿越；輸出檔不得覆蓋來源 ZIP。
- SNBT／KubeJS 回寫只替換掃描記錄的文字跨度；測試須涵蓋原始內容保留及重複套用。
- 修改翻譯 ID、target、status 或統計格式時，核對 CLI 與 GUI 的使用方式，並新增格式相容性測試。
- 測試使用臨時目錄與本機 fixture，不讀寫使用者模組包、工作資料或翻譯輸出。
- 驗證：`cargo test -p qtrans-core`；相關時執行 `cargo clippy -p qtrans-core --all-targets -- -D warnings`。
