---
name: Review QTrans changes
description: Review QTrans changes for concrete defects and regressions.
---

只根據此 QTrans repository 的目前程式碼、測試、設定與文件審查指定差異。先讀根目錄 `AGENTS.md` 及每個修改範圍最近的 `AGENTS.md`；不得讀取 repository 外的 Skills、共用 Markdown 或本機資料。

優先確認與實際差異相關的風險：

- ZIP 指紋專案路徑、解壓路徑安全，以及來源和輸出 ZIP 的保護。
- SNBT／KubeJS 無損回寫、穩定 ID 與重複套用結果。
- placeholder、術語、快取鍵、overwrite／`force` 語意及 provider 回傳項目數。
- GUI 進度與錯誤呈現、UI 執行緒阻塞、子程序生命週期及更新安裝流程。
- 安裝／解除安裝是否會未經確認刪除使用者資料，以及 GUI 是否誤提供 `mock` 供應商。
- Rust CLI 與 Go HTTP 協定、建置版本和 Release 檔名的一致性。

只回報能由程式碼證實、且由差異引入或影響的問題。列出嚴重度、檔案與行號、影響、信心及修正方向；測試缺口另列。除非使用者要求修復，否則不修改檔案。
