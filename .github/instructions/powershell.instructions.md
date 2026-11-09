---
name: QTrans Windows Scripts
description: PowerShell guidance for QTrans build and packaging scripts.
applyTo: "scripts/**/*.ps1"
---

# Windows PowerShell 指引

適用於 `scripts/**/*.ps1`。通用限制見根目錄 `AGENTS.md`，腳本規則見 `scripts/AGENTS.md`。

- 路徑由 `$PSScriptRoot` 推導；不可假設腳本啟動時的目前目錄。
- 維持明確錯誤處理；呼叫原生程式後檢查 `$LASTEXITCODE`，失敗時不得繼續回報成功。
- 不清除模組包、使用者工作目錄或翻譯資料；封裝只處理腳本明確管理的 staging 輸出。
- 安裝程式解除安裝不得無提示刪除使用者資料；預設保留 `work/`、`modpacks/`、`zip/` 並提示位置。
- 修改建置版本、安裝包名稱或參數時，核對相關 scripts、Inno Setup、workflow 與 README。
- 只有實際在 Windows 執行安裝包建置後，才可宣稱封裝驗證通過。
- 專案依據限定於本 repository。
