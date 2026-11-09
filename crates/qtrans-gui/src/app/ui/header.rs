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
        let can_cancel = self.can_cancel_job();
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
                            .bg(rgb(BG))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .flex()
                                            .gap_1()
                                            .child(pixel(ACCENT))
                                            .child(pixel(TEXT))
                                            .child(pixel(TEXT)),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap_1()
                                            .child(pixel(TEXT))
                                            .child(div().w(px(6.)).h(px(6.)))
                                            .child(pixel(TEXT)),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap_1()
                                            .child(pixel(TEXT))
                                            .child(div().w(px(6.)).h(px(6.)))
                                            .child(pixel(ACCENT)),
                                    ),
                            ),
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
                    .when(can_cancel, |actions| {
                        actions.child(button(
                            "cancel-job",
                            "取消工作",
                            RED,
                            true,
                            cx.listener(|this, _, _, cx| this.cancel_job(cx)),
                        ))
                    })
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

fn pixel(color: u32) -> impl IntoElement {
    div().w(px(6.)).h(px(6.)).bg(rgb(color))
}
