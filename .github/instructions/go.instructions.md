---
name: QTrans Go Translator
description: Go HTTP translation service guidance for QTrans.
applyTo: "go/translator/**/*.go"
---

# Go 翻譯服務指引

適用於 `go/translator/**/*.go`。通用限制見根目錄 `AGENTS.md`，服務規則見 `go/translator/AGENTS.md`。

- 維持 HTTP server、provider、快取、共用型別及 placeholder 保護的既有分工。
- `/health` 是服務健康檢查端點；翻譯請求與回應需和 Rust CLI 使用的協定一致。
- 驗證輸入及 provider 結果；結果缺漏、數量不符或 placeholder 損壞時不得回報成功。
- 保持 `force` 對快取讀取與更新的語意；修改時補上涵蓋命中、略過及更新的測試。
- 測試使用 `httptest`、fake provider 與本機資料，不呼叫真實供應商、不使用真實金鑰。
- 只依據 repository 內資料；Go 驗證命令見 `go/translator/AGENTS.md`。
