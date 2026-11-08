use super::components::button;
use crate::app::updates::UpdateStatus;
use crate::app::QTransApp;
use crate::theme::*;
use gpui::{div, prelude::*, px, rgb, Context, IntoElement};

impl QTransApp {
    pub(super) fn render_header(
        &mut self,
        status_color: u32,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let header = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_2()
            .w_full()
            .when(compact, |this| this.p_3())
            .when(!compact, |this| this.px_6().py_3())
            .bg(rgb(BG))
            .border_b_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(36.))
                            .h(px(36.))
                            .rounded_md()
                            .bg(rgb(ACCENT))
                            .text_sm()
                            .text_color(rgb(BG))
                            .child("QT"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(div().text_sm().text_color(rgb(TEXT)).child("QTRANS"))
                            .when(!compact, |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(MUTED))
                                        .child("MINECRAFT MODPACK LOCALIZATION"),
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .flex_wrap()
                    .gap_2()
                    .child(button(
                        "check-updates",
                        match &self.update_status {
                            UpdateStatus::Checking => "檢查中…",
                            UpdateStatus::Available { .. } => "有新版",
                            _ => "檢查更新",
                        },
                        if matches!(&self.update_status, UpdateStatus::Available { .. }) {
                            ACCENT
                        } else {
                            PANEL_ALT
                        },
                        !matches!(
                            &self.update_status,
                            UpdateStatus::Checking | UpdateStatus::Downloading { .. }
                        ),
                        cx.listener(|this, _, _, cx| this.check_for_updates(cx)),
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_2()
                            .rounded_full()
                            .bg(rgb(PANEL))
                            .child(
                                div()
                                    .w(px(8.))
                                    .h(px(8.))
                                    .rounded_full()
                                    .bg(rgb(status_color)),
                            )
                            .text_sm()
                            .text_color(rgb(status_color))
                            .child(self.status.clone()),
                    ),
            );
        header
    }
}
