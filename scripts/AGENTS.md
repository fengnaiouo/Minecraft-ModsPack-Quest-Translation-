# Windows scripts 指引

本檔只補充 `scripts/` 的規則；共通規則見 repository 根目錄 `AGENTS.md`。

- PowerShell 腳本以 Windows 環境為目標；從 `$PSScriptRoot` 推導 repository 路徑，不依賴呼叫者目前目錄。
- 維持明確錯誤處理；每次呼叫原生程式後檢查 `$LASTEXITCODE`，失敗即停止。
- `build.ps1` 建置 Rust 與 Go 程式；`package-windows.ps1` 建置 Windows 安裝包。修改時以腳本內容為準核對版本及輸出路徑。
- 封裝只清理腳本明確管理的 staging 目錄；不得清除使用者模組包、專案、翻譯資料或快取。
- 安裝程式解除安裝時不得無提示刪除 `work/`、`modpacks/`、`zip/`；預設保留使用者資料並提示位置。
- `run-gui.ps1`、`start-service.ps1`、`translate-pack.ps1` 與 `verify.ps1` 是專案輔助腳本；修改其參數或用途時更新對應文件。
- 完整安裝包驗證需在 Windows 且有 Inno Setup 的環境實際執行；未執行不得宣稱通過。
