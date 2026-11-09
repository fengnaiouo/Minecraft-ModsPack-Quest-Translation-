---
name: Check QTrans release readiness
description: Review Windows release inputs and checks without publishing.
---

只使用 QTrans repository 內的程式碼、設定、文件與工具。先閱讀根目錄 `AGENTS.md`、`.github/AGENTS.md` 及 `scripts/AGENTS.md`。

檢查 Windows 發布準備狀態，但不得發布、建立 tag、push 或建立 commit：

1. 比對 Cargo workspace 版本與預定的 `v<version>` tag。
2. 檢查 Rust workspace 測試及 workflow 所要求的 Go 測試。
3. 檢查封裝腳本、Inno Setup 設定、workflow 與預期安裝檔名稱是否一致。
4. 若環境允許，執行 `scripts/package-windows.ps1`，確認安裝包建置結果；若缺少 Windows 或 Inno Setup，明確標示未執行。
5. 依安裝設定檢查預期檔案，不以未執行的測試或推測代替驗證。

不得刪除使用者資料、安裝工具或修改任務範圍外的檔案。只回報實際執行的命令、結果、警告與未執行項目；不得宣稱未驗證的項目通過。
