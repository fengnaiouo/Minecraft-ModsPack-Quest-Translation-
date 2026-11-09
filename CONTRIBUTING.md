# 貢獻指南

感謝你對 QTrans 有興趣。本專案以 Windows x64 為首發平台，處理 Minecraft 模組包中的 FTB Quests 與 KubeJS 玩家可見文字。

## 開始之前

- 請先閱讀根目錄 [`AGENTS.md`](./AGENTS.md) 與目標目錄最近的 `AGENTS.md`；這些文件說明模組責任與不可破壞的行為。
- 請勿在 issue、PR、log 或 commit 中附上 API 金鑰、私人模組包、翻譯成果或個人資料。
- 安全性問題請依 [`SECURITY.md`](./SECURITY.md) 私下回報，不要開公開 issue。

## 開發環境

需求：

- Rust stable
- Go 1.24+
- Windows 10／11 x64（GUI 與安裝包驗證）
- Inno Setup 6 或更新版本（僅打包安裝程式時需要）

建置：

```powershell
cargo build --release
cd go/translator
go build -o ../../bin/qtrans-translator.exe .
```

## 驗證

提交前請執行與變更範圍相符的檢查：

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd go/translator
go test ./...
go vet ./...
```

修改 GUI 或安裝流程時，另請說明是否已實際執行 `.\scripts\package-windows.ps1` 及 Windows 安裝／解除安裝驗證。未執行的項目請明確標示，不要宣稱通過。

## Pull Request

1. 從 `main` 建立分支，保持變更聚焦。
2. 說明問題、做法、影響範圍與實際執行的驗證。
3. UI 變更請附上截圖或操作說明；安裝流程變更請說明資料保留行為。
4. 不要順便重構無關程式碼，也不要提交建置產物、快取或個人資料。
5. 通過 CI 與維護者審查後合併。

## 翻譯與格式貢獻

- 修改掃描、回寫或 placeholder 保護時，請補上涵蓋原始內容保留與重複套用的測試。
- 新增翻譯供應商時，請保持 Rust CLI 與 Go 服務的請求／回應及 `force` 語意一致。
- 測試不得呼叫真實翻譯服務；請使用本機 fixture 或 `mock`。

## 授權

提交貢獻即表示你同意以本專案的 [MIT License](./LICENSE) 授權該貢獻。
