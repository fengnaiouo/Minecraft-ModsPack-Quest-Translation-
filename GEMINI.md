# QTrans 指引

本檔是此 repository 的 Gemini CLI 入口。工作規則以根目錄 [`AGENTS.md`](./AGENTS.md) 及目標目錄最近的 `AGENTS.md` 為準。

- 先確認使用者要求的範圍；只改指定檔案類型，不擅自延伸修改。
- 只以本 repository 內的程式碼、測試、設定和文件為依據。`.agents/skills/` 是專案自有 Skills；不得讀取 repository 外的 Skills、共用 Markdown 或本機資料。
- 維持 Rust 核心、CLI、GPUI GUI 與 Go 翻譯服務的既有責任邊界。
- 保護使用者模組包、翻譯資料、快取、API 金鑰及尚未提交的變更。
- 只執行適用於本次工作的最小驗證；清楚區分已通過、失敗及未執行的檢查。未經要求不建立 commit。
