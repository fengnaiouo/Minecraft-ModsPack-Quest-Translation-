use super::components::*;
use crate::app::config::ConfigField;
use crate::app::QTransApp;
use crate::theme::*;
use gpui::{div, prelude::*, rgb, Context, IntoElement};

impl QTransApp {
    pub(super) fn render_review_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let (translated, total) = self.review_counts();
        let has_entries = !self.review_entries.is_empty();
        let dirty = self.review_is_dirty();
        let entry = self.review_entries.get(self.review_index);
        let position = if has_entries {
            format!("{} / {}", self.review_index + 1, total)
        } else {
            "尚無可校訂文字".to_string()
        };
        let header = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_sm().text_color(rgb(TEXT)).child("翻譯校訂"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!("已完成 {translated} / {total} 條 · {position}")),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(button(
                        "review-previous",
                        "上一條",
                        PANEL_ALT,
                        has_entries && !dirty && self.review_index > 0,
                        cx.listener(|this, _, _, cx| {
                            this.select_review_entry(this.review_index.saturating_sub(1), cx)
                        }),
                    ))
                    .child(button(
                        "review-next",
                        "下一條",
                        PANEL_ALT,
                        has_entries && !dirty && self.review_index + 1 < total,
                        cx.listener(|this, _, _, cx| {
                            this.select_review_entry(this.review_index + 1, cx)
                        }),
                    )),
            );
        let mut panel = div()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .rounded_lg()
            .bg(rgb(PANEL))
            .border_1()
            .border_color(rgb(BORDER))
            .child(header);

        if let Some(entry) = entry {
            let kind = match entry.unit.kind {
                qtrans_core::model::UnitKind::Lang => "語言檔",
                qtrans_core::model::UnitKind::Inline => "FTB Quests",
                qtrans_core::model::UnitKind::Script => "KubeJS",
            };
            let context = if entry.unit.context.is_empty() {
                entry
                    .unit
                    .key
                    .as_deref()
                    .or(entry.unit.path.as_deref())
                    .unwrap_or("無額外情境")
            } else {
                entry.unit.context.as_str()
            };
            panel = panel
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_2()
                                .child(stat("類型", kind.to_string(), ACCENT))
                                .child(div().text_xs().text_color(rgb(MUTED)).child(format!(
                                    "狀態：{}",
                                    if entry.translation.is_some() {
                                        "已有翻譯"
                                    } else {
                                        "待翻譯"
                                    }
                                ))),
                        )
                        .child(path_line("檔案", entry.unit.file.clone()))
                        .child(path_line("情境", context.to_string()))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_xs().text_color(rgb(MUTED)).child("原文"))
                                .child(
                                    div()
                                        .p_3()
                                        .rounded_md()
                                        .bg(rgb(PANEL_ALT))
                                        .text_sm()
                                        .text_color(rgb(TEXT))
                                        .child(entry.unit.source.clone()),
                                ),
                        ),
                )
                .child(setting_field(
                    SettingFieldOptions {
                        id: "review-translation",
                        label: "繁體中文翻譯",
                        value: &self.review_translation_draft,
                        placeholder: "輸入或修訂翻譯",
                        masked: false,
                        active: matches!(self.editing, Some(ConfigField::ReviewTranslation)),
                        cursor: self.cursor,
                        select_all: self.select_all,
                        enabled: !self.busy,
                        focus_handle: &self.focus_handle,
                    },
                    cx.listener(|this, event, window, cx| {
                        this.activate_field(ConfigField::ReviewTranslation, event, window, cx)
                    }),
                    cx.listener(|this, event, window, cx| {
                        this.edit_field(ConfigField::ReviewTranslation, event, window, cx)
                    }),
                ))
                .when(dirty, |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(rgb(ORANGE))
                            .child("校訂尚未儲存；請先儲存，避免切換項目時遺失修改。"),
                    )
                })
                .child(button(
                    "save-review-translation",
                    "儲存校訂",
                    GREEN,
                    !self.busy && dirty,
                    cx.listener(|this, _, _, cx| this.save_review_translation(cx)),
                ));
        } else {
            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .items_start()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child("請先選擇 ZIP 並完成掃描，或開啟既有專案。"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child("校訂內容會直接追加到專案翻譯記錄；回寫前可隨時再次修改。"),
                    ),
            );
        }
        panel
    }
}
