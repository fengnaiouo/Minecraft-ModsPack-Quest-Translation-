use super::components::button;
use crate::app::{AppPage, QTransApp};
use crate::theme::*;
use gpui::{div, prelude::*, px, rgb, Context, IntoElement};

impl QTransApp {
    pub(super) fn render_navigation(
        &mut self,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.page;
        div()
            .flex()
            .when(compact, |this| this.flex_row().flex_wrap().gap_2())
            .when(!compact, |this| {
                this.flex_col()
                    .w(px(150.))
                    .flex_none()
                    .gap_2()
                    .p_3()
                    .border_r_1()
                    .border_color(rgb(BORDER))
            })
            .when(compact, |this| this.px_3().py_2())
            .child(button(
                "nav-workspace",
                "工作台",
                if selected == AppPage::Workspace {
                    ACCENT
                } else {
                    PANEL_ALT
                },
                true,
                cx.listener(|this, _, _, cx| this.set_page(AppPage::Workspace, cx)),
            ))
            .child(button(
                "nav-review",
                "翻譯校訂",
                if selected == AppPage::Review {
                    ACCENT
                } else {
                    PANEL_ALT
                },
                true,
                cx.listener(|this, _, _, cx| this.set_page(AppPage::Review, cx)),
            ))
            .child(button(
                "nav-settings",
                "翻譯設定",
                if selected == AppPage::Settings {
                    ACCENT
                } else {
                    PANEL_ALT
                },
                true,
                cx.listener(|this, _, _, cx| this.set_page(AppPage::Settings, cx)),
            ))
            .child(button(
                "nav-logs",
                "執行紀錄",
                if selected == AppPage::Logs {
                    ACCENT
                } else {
                    PANEL_ALT
                },
                true,
                cx.listener(|this, _, _, cx| this.set_page(AppPage::Logs, cx)),
            ))
    }
}
