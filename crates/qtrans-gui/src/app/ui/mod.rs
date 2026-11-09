mod components;
mod header;
mod logs;
mod navigation;
mod project;
mod review;
mod settings;
mod stats;
mod updates;
mod workflow;

use super::QTransApp;
use crate::theme::*;
use gpui::{div, prelude::*, px, rgb, Context, IntoElement, Window};

impl Render for QTransApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let compact = window.bounds().size.width < px(760.);
        let header_compact = window.bounds().size.width < px(980.);
        let provider_name = match self.provider.as_str() {
            "google_free" => "Google 免費".to_string(),
            "openai" => "OpenAI".to_string(),
            "deepl" => "DeepL".to_string(),
            "offline" => "離線術語表".to_string(),
            "mock" => "測試模式".to_string(),
            other => other.to_string(),
        };
        let status_color = if self.busy {
            ORANGE
        } else if self.status.contains("失敗") || self.status.contains("錯誤") {
            RED
        } else {
            GREEN
        };

        let header = self.render_header(status_color, header_compact, cx);
        let project_card = self.render_project_panel(cx);
        let review = self.render_review_panel(cx);
        let settings = self.render_settings_panel(&provider_name, cx);
        let workflow = self.render_workflow_panel(cx);
        let stats = self.render_stats_panel(cx);
        let log_panel = self.render_log_panel(cx);
        let updates = self.render_updates_panel(cx);
        let navigation = self.render_navigation(compact, cx);
        let main_content = match self.page {
            super::AppPage::Workspace => div().child(project_card).child(workflow).child(stats),
            super::AppPage::Review => div().child(review),
            super::AppPage::Settings => div().child(settings),
            super::AppPage::Logs => div().child(stats).child(log_panel),
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .child(header)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(px(0.))
                    .child(updates)
                    .child(
                        div()
                            .flex()
                            .when(compact, |this| this.flex_col())
                            .when(!compact, |this| this.flex_row())
                            .flex_1()
                            .min_h(px(0.))
                            .child(navigation)
                            .child(
                                div()
                                    .id("main-content-scroll")
                                    .flex()
                                    .flex_col()
                                    .gap_4()
                                    .when(compact, |this| this.p_3())
                                    .when(!compact, |this| this.p_6())
                                    .flex_1()
                                    .min_h(px(0.))
                                    .overflow_y_scroll()
                                    .child(main_content),
                            ),
                    ),
            )
    }
}
