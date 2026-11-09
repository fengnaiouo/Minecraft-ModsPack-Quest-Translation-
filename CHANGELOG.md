# 變更紀錄

本檔是人工整理的功能與維護變更摘要，不取代完整 Git 歷史。日期與提交資訊依本地 repository 歷史整理；不代表每項功能都已在正式版本發布。

## 2026-11-09

- **初始專案提交**（`ceaa354`）：加入 QTrans 的 README 與專案目的說明。
- **GUI 與自動更新**（`10de6d5`）：建立 Rust workspace、Go 翻譯服務、GPUI GUI、Windows 建置／封裝流程及 tag 發布 workflow；提交標題為「Add adaptive GUI and self-update releases」。
- **安裝程式語系回退**（`4cde828`）：修正 CI 安裝程式語系 fallback。

## 維護方式

- 新增項目應由已合併／提交的變更或明確確認的發布內容整理，不把未完成的工作區變更寫成已發布功能。
- 正式版本號與發布狀態以 Cargo workspace、Git tag 及 GitHub Release 的實際狀態為準。
- 需要完整逐筆提交資訊時，查看 `git log`；不要將完整 log 複製到此檔。
