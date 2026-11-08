use super::components::*;
use crate::app::QTransApp;
use crate::theme::*;
use gpui::{div, prelude::*, rgb, Context, IntoElement};

impl QTransApp {
    pub(super) fn render_workflow_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.busy;
        let has_zip = self.zip_path.is_some();
        let has_project = self
            .project_dir
            .as_ref()
            .is_some_and(|path| path.join("manifest.json").exists());
        let workflow = div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
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
                            .child(div().text_sm().text_color(rgb(TEXT)).child("翻譯流程"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .child("依序執行，或直接一鍵完成全部步驟"),
                            ),
                    )
                    .child(button(
                        "run-all",
                        "一鍵全部執行",
                        GREEN,
                        has_zip && !busy,
                        cx.listener(|this, _, _, cx| this.run_all(cx)),
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(button(
                        "step-extract",
                        "01  解包＋掃描",
                        PANEL_ALT,
                        has_zip && !busy,
                        cx.listener(|this, _, _, cx| this.extract_and_scan(cx)),
                    ))
                    .child(button(
                        "step-translate",
                        "02  翻譯",
                        PANEL_ALT,
                        has_project && !busy,
                        cx.listener(|this, _, _, cx| this.translate(cx)),
                    ))
                    .child(button(
                        "step-apply",
                        "03  回寫",
                        PANEL_ALT,
                        has_project && !busy,
                        cx.listener(|this, _, _, cx| this.apply(cx)),
                    ))
                    .child(button(
                        "step-build",
                        "04  打包",
                        PANEL_ALT,
                        has_project && !busy,
                        cx.listener(|this, _, _, cx| this.build(cx)),
                    )),
            );
        workflow
    }
}
