# Git 歷史摘要

本文件是截至 2026-11-09 在目前本地 repository 可見的已提交里程碑摘要，不是完整提交清單。完整歷史會隨 repository 更新，應以 `git log` 為準。

## 已提交節點

| Commit | 日期 | 提交標題 | 摘要 |
|---|---|---|---|
| `ceaa354` | 2026-11-09 | `first commit` | 新增 README 專案說明。 |
| `10de6d5` | 2026-11-09 | `Add adaptive GUI and self-update releases` | 加入 Rust workspace、GPUI GUI、Go 翻譯服務、Windows 建置與封裝流程、tag 發布 workflow。 |
| `4cde828` | 2026-11-09 | `Fix CI installer language fallback` | 修正 CI 安裝程式語系 fallback。 |

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
