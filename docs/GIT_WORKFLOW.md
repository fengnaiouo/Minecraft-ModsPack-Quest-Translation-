# QTrans Git 與發布流程

本文件只記錄本 repository 可確認的 Git 與發布慣例。分支命名、commit message 格式、審查者及合併策略目前沒有足夠的專案依據，不在此臆定。

## 修改前與修改後

1. 在 repository 根目錄檢查 `git status --short`，辨識既有工作區變更。
2. 只修改本次任務範圍；不要覆蓋、還原或清理其他未提交檔案。
3. 以 `git diff --check` 檢查空白錯誤，並執行與變更範圍相符的測試。
4. 驗證通過後，使用 `rtk git` 建立 commit 並推送到目前已設定的 GitHub remote；只納入本次工作範圍內的檔案，保留其他未提交內容。
5. 向使用者回報實際修改、驗證結果及 push 結果。若驗證失敗或 push 失敗，明確說明並持續處理，不宣稱已完成。
6. 使用者已授權完整管理本專案 GitHub repository，包括建立 tag 及 GitHub Release；達到下方發布驗收條件時可自動發布，不需再次詢問。

## Windows Release

目前 repository 內可確認的發布流程如下：

- `.github/workflows/release.yml` 在 `v*` tag push 時啟動 Windows workflow。
- Workflow 比對 tag 版本與 `Cargo.toml` 的 workspace 版本，執行 Rust workspace tests、Go translator tests，然後建置 Windows 安裝包。
- Workflow 發布 `dist/QTrans-Setup-x64.exe`；封裝工作由 `scripts/package-windows.ps1` 負責，Inno Setup 設定位於 `scripts/qtrans.iss`。
- Go 版本依 `go.mod` 及 workflow 設定核對；更新版本時檢查建置腳本和發布 workflow。
- 發布前核對乾淨且正確的目標 commit、workspace／tag 版本一致、完整必要測試、安裝包成功建置與版本資訊、checksum 及發行說明。
- 驗收條件全數通過後，由代理建立與 workspace 版本相符的 `v*` tag 並推送，監看 GitHub Actions；確認 workflow 成功且 Release 與安裝包附件可用後才回報發布完成。
- 若任何發布閘門失敗，不建立正式版本 tag；持續修復並重新驗證。此 GitHub 管理授權不包含付費服務、代替使用者購買資源或未經同意傳送使用者資料。

## Git 紀錄維護

- 完整 commit、tag 與工作區狀態以 Git 為準。
- `docs/GIT_HISTORY.md` 僅整理有代表性的已提交里程碑；`CHANGELOG.md` 整理面向使用者的變更。
- 不記錄憑證、API 金鑰、私人模組包資料或其他敏感內容。
