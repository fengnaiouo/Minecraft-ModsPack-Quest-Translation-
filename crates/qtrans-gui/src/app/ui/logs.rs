use super::components::button;
use crate::app::QTransApp;
use crate::theme::*;
use gpui::{div, prelude::*, px, rgb, Context, IntoElement};

impl QTransApp {
    pub(super) fn render_log_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let log_panel = div()
            .flex()
            .flex_col()
            .gap_3()
            .p_5()
            .flex_1()
            .min_h(px(220.))
            .rounded_lg()
            .bg(rgb(PANEL))
            .border_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_sm().text_color(rgb(TEXT)).child("執行紀錄"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .child("顯示最近的流程步驟、結果與必要訊息"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(PANEL_ALT))
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .child(format!("{} 筆", self.log.len())),
                            )
                            .child(button(
                                "toggle-log-follow",
                                if self.follow_logs {
                                    "自動跟隨中"
                                } else {
                                    "跟隨最新"
                                },
                                if self.follow_logs { ACCENT } else { PANEL_ALT },
                                true,
                                cx.listener(|this, _, _, cx| {
                                    this.follow_logs = !this.follow_logs;
                                    if this.follow_logs {
                                        this.log_scroll.scroll_to_bottom();
                                    }
                                    cx.notify();
                                }),
                            )),
                    ),
            )
            .child(
                div()
                    .id("execution-log-scroll")
                    .track_scroll(&self.log_scroll)
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .gap_1()
                    .children(
                        self.log
                            .iter()
                            .map(|line| {
                                let (label, color, detail) = log_presentation(line);
                                div()
                                    .flex()
                                    .items_start()
                                    .gap_3()
                                    .w_full()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .bg(rgb(PANEL_ALT))
                                    .child(
                                        div()
                                            .w(px(42.))
                                            .flex_none()
                                            .text_xs()
                                            .text_color(rgb(color))
                                            .child(label),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_sm()
                                            .text_color(rgb(
                                                if label == "資訊" || label == "詳細" {
                                                    MUTED
                                                } else {
                                                    TEXT
                                                },
                                            ))
                                            .child(detail),
                                    )
                            })
                            .collect::<Vec<_>>(),
                    ),
            );
        log_panel
    }
}

fn log_presentation(line: &str) -> (&'static str, u32, String) {
    if let Some(detail) = line.strip_prefix("錯誤｜") {
        ("錯誤", RED, detail.to_string())
    } else if let Some(detail) = line.strip_prefix("警告｜") {
        ("警告", ORANGE, detail.to_string())
    } else if let Some(detail) = line.strip_prefix("開始｜") {
        ("進行中", ACCENT, detail.to_string())
    } else if let Some(detail) = line.strip_prefix("完成｜") {
        ("完成", GREEN, detail.to_string())
    } else if let Some(detail) = line.strip_prefix("步驟｜") {
        ("步驟", ACCENT, detail.to_string())
    } else if let Some(detail) = line.strip_prefix("詳細｜") {
        ("詳細", MUTED, detail.to_string())
    } else if let Some(detail) = line.strip_prefix("資訊｜") {
        ("資訊", MUTED, detail.to_string())
    } else {
        ("資訊", MUTED, line.to_string())
    }
}
