# GitHub 專案檔案指引

本檔涵蓋 `.github/` 中的 workflow 與 QTrans AI 指引；共通規則見 repository 根目錄 [`AGENTS.md`](../AGENTS.md)。

- 發布 workflow 位於 `.github/workflows/release.yml`，由 `v*` tag 觸發，並在 Windows runner 建置安裝包。
- Workflow 會測試 Rust workspace 與 Go translator；安裝包由 `scripts/package-windows.ps1` 建置，發布檔名須與 workflow、封裝腳本及 Inno Setup 設定一致。
- 修改 workflow 時維持最小權限；不得將 API 金鑰、憑證、模組包內容或其他私人資料輸出至 log、artifact 或 commit。
- `.github/instructions/` 放置適用於檔案範圍的 Copilot 指引；`.github/prompts/` 放置 QTrans 工作提示。
- AI 文件只能描述本 repository 可確認的行為；不得引用 repository 外的 Skills、共用 Markdown 或本機設定。
