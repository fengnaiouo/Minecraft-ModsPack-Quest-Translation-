# QTrans Copilot 指引

修改前閱讀根目錄 [`AGENTS.md`](../AGENTS.md)，並依目標檔案閱讀最近的 `AGENTS.md`。

- 遵守使用者指定的工作範圍；Markdown-only 工作不得改動程式碼、設定或建置產物。
- 只使用本 repository 內的程式碼、測試、設定、文件及專案自有 Skills（`.agents/skills/`）。不得讀取 repository 外的 Skills、共用 Markdown 或本機資料。
- Rust workspace 包含 `qtrans-core`、`qtrans-cli`、`qtrans-gui`；Go 翻譯服務位於 `go/translator`。
- 核心 ZIP、SNBT／KubeJS、placeholder、翻譯資料格式與 CLI／Go 協定的限制，以程式碼、測試及根目錄 `AGENTS.md` 為準。
- 使用本機 fixture 或 mock 測試，不呼叫真實翻譯 API；保護 API 金鑰與使用者資料。
- 安裝／解除安裝變更不得在未確認下刪除使用者資料；GUI 不提供 `mock`，並在結束時停止它啟動的翻譯服務。
- 執行適用的最小驗證，回報實際命令與結果。未經要求不建立 commit。
