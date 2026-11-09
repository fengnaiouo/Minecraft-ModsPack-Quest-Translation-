# QTrans 指引

本檔是此 repository 的 Claude Code 入口，不是全域規則。開始前閱讀根目錄 [`AGENTS.md`](./AGENTS.md)，再閱讀目標目錄最近的 `AGENTS.md`。

- 嚴格遵守使用者指定的檔案範圍；若使用者只要求 Markdown，不修改任何程式碼或設定。
- 專案資料來源限定於本 repository。`.agents/skills/` 是專案自有 Skills；不要讀取 repository 外的 Skills、共用 Markdown 或本機資料。
- 依現有 crate、Go service 與 scripts 的分工修改；不複製核心檔案處理邏輯。
- 修改翻譯流程時保護 ZIP、SNBT／KubeJS 原始內容、placeholder、術語及 API 金鑰。
- 執行與修改相符的最小檢查，並只回報實際結果。未經要求不建立 commit。
