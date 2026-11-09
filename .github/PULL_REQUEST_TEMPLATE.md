## 變更內容

<!-- 說明這個 PR 解決什麼問題、做了什麼。 -->

## 影響範圍

- [ ] Rust 核心／CLI
- [ ] GPUI GUI
- [ ] Go 翻譯服務
- [ ] 安裝／解除安裝／更新
- [ ] 文件或 CI

## 驗證

請勾選實際執行過的項目；未執行者請說明原因。

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `go test ./...`
- [ ] `go vet ./...`
- [ ] `.\scripts\package-windows.ps1`（若涉及安裝包）
- [ ] Windows 安裝／升級／解除安裝實機驗證（若涉及安裝流程）

## 資料與安全

- [ ] 未包含 API 金鑰、憑證、私人模組包或個人資料。
- [ ] 若涉及解除安裝，已確認預設保留 `work/`、`modpacks/`、`zip/`。
- [ ] 若涉及翻譯流程，已驗證 placeholder、術語與失敗狀態。

## 其他說明

<!-- 截圖、取捨、待確認事項或後續工作。 -->
