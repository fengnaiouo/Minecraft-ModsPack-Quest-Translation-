# Go 翻譯服務指引

本檔只補充 `go/translator` 的規則；共通規則見 repository 根目錄 `AGENTS.md`。

- Go 版本依 `go.mod`；`main.go` 啟動服務，`server.go` 提供 HTTP handler，其餘程式依 provider、快取、型別及保護邏輯分工。
- `/health` 提供健康檢查；翻譯 endpoint 的請求和回應須與 Rust CLI 協定相容。
- 驗證 HTTP 輸入、provider 結果數量及 placeholder；不完整或無效結果不得回報成功。
- 快取 key 須涵蓋影響翻譯結果的輸入；`force: true` 必須依現有協定略過舊結果並更新成功的新結果。
- 使用 `httptest`、fake provider 和本機資料測試；不得呼叫真實翻譯供應商或記錄 API 金鑰。
- 服務版本由 repository 的 Windows 建置流程設定；修改版本時檢查 Cargo 版本、scripts、workflow 及 README。
- 格式檢查：在 repository 根目錄執行 `Get-ChildItem go\translator -Filter *.go | ForEach-Object { gofmt -d $_.FullName }`。
- 測試與靜態檢查：在此目錄執行 `go test ./...`、`go vet ./...`。
