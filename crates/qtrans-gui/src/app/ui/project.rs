use super::components::*;
use crate::app::QTransApp;
use crate::theme::*;
use gpui::{div, prelude::*, rgb, Context, IntoElement};

impl QTransApp {
    pub(super) fn render_project_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.busy;
        let has_project = self
            .project_dir
            .as_ref()
            .is_some_and(|path| path.join("manifest.json").exists());
        let project_card = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .p_5()
            .rounded_lg()
            .bg(rgb(PANEL))
            .border_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .flex_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(ACCENT))
                            .child("WORKSPACE  /  MODPACK PROJECT"),
                    )
                    .child(div().text_xl().text_color(rgb(TEXT)).child(
                        if self.pack_name.is_empty() {
                            "開始新的漢化專案".to_string()
                        } else {
                            self.pack_name.clone()
                        },
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(path_line(
                                "來源 ZIP",
                                self.zip_path
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|| "尚未選擇模組包".to_string()),
                            ))
                            .child(path_line(
                                "工作目錄",
                                self.project_dir
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|| "選擇 ZIP 後自動建立".to_string()),
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_end()
                    .gap_2()
                    .child(button(
                        "choose-zip",
                        "選擇 ZIP",
                        ACCENT,
                        !busy,
                        cx.listener(|this, _, _, cx| this.choose_zip(cx)),
                    ))
                    .child(button(
                        "open-project",
                        "開啟專案",
                        PANEL_ALT,
                        !busy,
                        cx.listener(|this, _, _, cx| this.open_project(cx)),
                    ))
                    .child(button(
                        "start-service",
                        "啟動翻譯服務",
                        PANEL_ALT,
                        !busy,
                        cx.listener(|this, _, _, cx| this.start_service(cx)),
                    ))
                    .child(button(
                        "open-output",
                        "開啟輸出",
                        PANEL_ALT,
                        has_project,
                        cx.listener(|this, _, _, _| this.open_output()),
                    )),
            );
        project_card
    }
}
