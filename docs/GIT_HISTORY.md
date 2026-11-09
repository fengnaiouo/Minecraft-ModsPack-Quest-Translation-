# Git 歷史摘要

本文件是截至 2026-11-09 在目前本地 repository 可見的已提交里程碑摘要，不是完整提交清單。完整歷史會隨 repository 更新，應以 `git log` 為準。

## 已提交節點

| Commit | 日期 | 提交標題 | 摘要 |
|---|---|---|---|
| `ceaa354` | 2026-11-09 | `first commit` | 新增 README 專案說明。 |
| `10de6d5` | 2026-11-09 | `Add adaptive GUI and self-update releases` | 加入 Rust workspace、GPUI GUI、Go 翻譯服務、Windows 建置與封裝流程、tag 發布 workflow。 |
| `4cde828` | 2026-11-09 | `Fix CI installer language fallback` | 修正 CI 安裝程式語系 fallback。 |
| `8570b0e` | 2026-11-09 | `修復 QTrans 翻譯流程並整理專案架構` | 整理翻譯、掃描、回寫與 GUI 流程。 |
| `de8ac24` | 2026-11-09 | `feat: harden translation workflow and add review UI` | 強化翻譯流程、加入人工校訂與相關測試。 |
| `95bf1df` | 2026-11-09 | `feat(gui): add visible execution log scrollbar` | 加入可見的執行紀錄捲動條。 |
| `ec1a2a0` | 2026-11-09 | `chore(release): prepare QTrans 0.1.3` | 準備 0.1.3 發布與版本資訊。 |
| `245881c` | 2026-11-09 | `fix: harden uninstall data retention and prepare public repository` | 解除安裝預設保留使用者資料、GUI 服務生命週期、更新檔位置、公開專案文件與 CI。 |
| `f363fde` | 2026-11-09 | `style: apply rustfmt and record repository milestones` | 套用 rustfmt 並補上 Git／專案里程碑紀錄。 |
| `319dfba` | 2026-11-09 | `test: isolate temporary test directories` | 測試暫存目錄改用 PID 與原子計數器，修正平行測試互相覆寫造成的 CI 不穩定。 |
| `470641c` | 2026-11-09 | `chore(deps): require manual review for cargo major upgrades` | cargo major 升級改為獨立變更；同日發布 `v0.1.4`。 |

## 查詢方式

在 repository 根目錄執行：

```powershell
git log --oneline --decorate
git log --stat
git show --stat <commit>
git status --short
git diff --check
```

不要把本文件當成 Git 的替代品；新增提交後只需更新重要里程碑摘要。未提交工作不應寫成已提交歷史。
