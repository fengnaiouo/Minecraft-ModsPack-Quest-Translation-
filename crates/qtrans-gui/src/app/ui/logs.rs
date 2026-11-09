use super::components::button;
use crate::app::QTransApp;
use crate::theme::*;
use gpui::{div, prelude::*, px, rgb, ClickEvent, Context, IntoElement};

impl QTransApp {
    pub(super) fn render_log_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let scroll_bounds = self.log_scroll.bounds();
        let viewport_height = f32::from(scroll_bounds.size.height).max(0.0);
        let max_scroll = f32::from(self.log_scroll.max_offset().height).max(0.0);
        let has_overflow = viewport_height > 0.0 && max_scroll > 0.0;
        let thumb_height = if has_overflow {
            (viewport_height * viewport_height / (viewport_height + max_scroll))
                .max(28.0)
                .min(viewport_height)
        } else {
            0.0
        };
        let scroll_offset = (-f32::from(self.log_scroll.offset().y)).clamp(0.0, max_scroll);
        let thumb_top = if has_overflow {
            (viewport_height - thumb_height) * (scroll_offset / max_scroll)
        } else {
            0.0
        };
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
                    .flex()
                    .flex_1()
                    .min_h(px(0.))
                    .gap_2()
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
                    )
                    .when(has_overflow, |this| {
                        this.child(
                            div()
                                .id("execution-log-scrollbar")
                                .w(px(12.))
                                .h_full()
                                .flex_none()
                                .flex()
                                .justify_center()
                                .cursor_pointer()
                                .child(
                                    div()
                                        .w(px(6.))
                                        .h_full()
                                        .rounded_full()
                                        .bg(rgb(BORDER))
                                        .flex()
                                        .flex_col()
                                        .child(div().h(px(thumb_top)).flex_none())
                                        .child(
                                            div()
                                                .w_full()
                                                .h(px(thumb_height))
                                                .flex_none()
                                                .rounded_full()
                                                .bg(rgb(ACCENT)),
                                        )
                                        .child(div().flex_1()),
                                )
                                .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                    let bounds = this.log_scroll.bounds();
                                    let viewport_height = f32::from(bounds.size.height);
                                    let max_scroll =
                                        f32::from(this.log_scroll.max_offset().height).max(0.0);
                                    if viewport_height <= 0.0 || max_scroll <= 0.0 {
                                        return;
                                    }
                                    let position = f32::from(event.position().y - bounds.origin.y);
                                    let progress = (position / viewport_height).clamp(0.0, 1.0);
                                    let offset = this.log_scroll.offset();
                                    this.log_scroll.set_offset(gpui::point(
                                        offset.x,
                                        px(-max_scroll * progress),
                                    ));
                                    this.follow_logs = false;
                                    cx.notify();
                                })),
                        )
                    }),
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
