# QTrans 專案歷程

本文件分開記錄「使用者提出的需求」與「Git 已提交的工作」，避免把討論中的構想誤寫成已完成的功能。需求次序依目前提供的對話摘要整理；提交節點依本 repository 的本地 Git 歷史核對。

## 起始目標

- 製作 Minecraft modpack 任務漢化工具，面向 FTB Quests 與 KubeJS 內容。
- 使用 Rust 建立核心與命令列工具，Go 提供翻譯服務，GUI 使用 Rust GPUI（Zed UI）。
- 從 modpack ZIP 擷取並處理 `config/ftbquests` 與 `kubejs` 相關內容。
- 目標翻譯為繁體中文；README 將輸出語言列為 `zh_tw`。

## 需求演進

1. 確認翻譯流程是否可用，以及 GUI 是否具備完整功能。
2. 要求 GUI 自適應，並加入檢查及安裝新版的能力。
3. 要求檢查程式碼與 GUI 的不合理處，並提出整體修復及模組化方向。
4. 指出 GUI 不應把所有功能擠在同一頁；偏好完整自適應，或以左側導覽切換功能。
5. 要求專案自己的 AI 文件，並明確排除外部本機 Skills、共用 Markdown 與外部範本。
6. 進一步指出 AI 文件不應只有規則，還要涵蓋專案歷程、對話結論、Git 等專案脈絡。
7. 要求對 UI、功能、安裝及解除安裝進行整體健檢；解除安裝不得無確認刪除使用者資料。
8. 要求同步更新所有 Markdown 文件（排除 Skills），並把 GitHub repository 整理成公開專案應有的樣貌。

## Git 已提交里程碑

- `ceaa354` — `first commit`：加入 README 專案說明。
- `10de6d5` — `Add adaptive GUI and self-update releases`：提交 Rust／Go 專案、GUI、服務及 Windows 發布流程。
- `4cde828` — `Fix CI installer language fallback`：修正安裝程式語系 fallback。
- `8570b0e`、`de8ac24`、`95bf1df` — 整理翻譯流程、加入人工校訂、強化驗證與執行紀錄捲動條。
- `ec1a2a0` — `chore(release): prepare QTrans 0.1.3`：準備 0.1.3 版本。
- `245881c` — `fix: harden uninstall data retention and prepare public repository`：解除安裝預設保留使用者資料、GUI 服務生命週期、更新檔位置、公開專案文件與 CI。
- `f363fde`、`319dfba` — 套用 rustfmt、補齊里程碑，並修正平行測試共用暫存目錄造成的 CI 不穩定。
- `470641c` — 將 cargo major 升級改為手動處理；`v0.1.4` 於同日發布。

以上提交日期均為 2026-11-09；完整提交內容以 Git 歷史為準。細節見 [`GIT_HISTORY.md`](./GIT_HISTORY.md)。

## 尚待確認或持續追蹤

- GUI 的最終導覽方式與窄視窗／低高度版面，需由使用者確認後才實作；曾提出的側欄變更已撤回，不能視為已採用設計。
- 「程式碼全部修復、整理及模組化」是使用者提出的工作方向；每個問題是否已完成，需逐項依目前程式碼與測試確認，不以本段摘要判定完成。
- 翻譯服務商、語言品質及支援範圍以目前程式碼和 README 為準；需求討論不代表所有供應商皆已實測。
