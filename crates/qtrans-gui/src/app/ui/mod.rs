mod components;
mod header;
mod logs;
mod project;
mod settings;
mod stats;
mod updates;
mod workflow;

use super::QTransApp;
use crate::theme::*;
use gpui::{div, prelude::*, px, rgb, Context, IntoElement, Window};

impl Render for QTransApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let compact = window.bounds().size.width < px(980.);
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

        let header = self.render_header(status_color, compact, cx);
        let project_card = self.render_project_panel(cx);
        let settings = self.render_settings_panel(&provider_name, cx);
        let workflow = self.render_workflow_panel(cx);
        let stats = self.render_stats_panel(cx);
        let log_panel = self.render_log_panel(cx);
        let updates = self.render_updates_panel(cx);

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .child(header)
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
                    .child(updates)
                    .child(project_card)
                    .child(workflow)
                    .child(
                        div()
                            .flex()
                            .when(compact, |this| this.flex_col().gap_3())
                            .when(!compact, |this| this.gap_4())
                            .flex_1()
                            .min_h(px(0.))
                            .child(if compact {
                                div().flex().flex_col().gap_3().w_full().child(settings)
                            } else {
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_4()
                                    .w(px(320.))
                                    .flex_none()
                                    .child(settings)
                            })
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .when(compact, |this| this.gap_3())
                                    .when(!compact, |this| this.gap_4())
                                    .flex_1()
                                    .min_h(px(0.))
                                    .child(stats)
                                    .child(log_panel),
                            ),
                    ),
            )
    }
}
