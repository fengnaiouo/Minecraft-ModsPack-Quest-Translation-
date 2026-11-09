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
        let guidance = if has_project {
            "可繼續翻譯、校訂、回寫或打包；翻譯服務與語言可在「翻譯設定」調整。"
        } else if has_zip {
            "可先在「翻譯設定」選擇供應商，或直接一鍵執行完整流程。"
        } else {
            "開始：選擇模組包 ZIP →（選用）設定翻譯服務 → 一鍵執行 → 校訂並打包。"
        };
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
                            .child(div().text_xs().text_color(rgb(MUTED)).child(guidance)),
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
                    .text_xs()
                    .text_color(rgb(ORANGE))
                    .child(provider_data_notice(&self.provider)),
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
