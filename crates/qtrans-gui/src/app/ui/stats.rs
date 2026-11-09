use super::components::*;
use crate::app::QTransApp;
use crate::theme::*;
use gpui::{div, prelude::*, px, rgb, IntoElement};

impl QTransApp {
    pub(super) fn render_stats_panel(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        let progress = (self.translate_progress() * 100.0).min(100.0);
        let detail = match self.batch_progress {
            Some((done, total)) if total > 0 => {
                format!("目前批次：{done} / {total} 組不重複文字")
            }
            _ if self.busy => "等待翻譯批次回報…".to_string(),
            _ => "完成批次後會即時更新".to_string(),
        };
        let stats = div()
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
                            .child(div().text_sm().text_color(rgb(TEXT)).child("翻譯進度"))
                            .child(div().text_xs().text_color(rgb(MUTED)).child(format!(
                                "{} / {} 條已翻譯",
                                self.translated, self.stats.total_units
                            ))),
                    )
                    .child(
                        div()
                            .text_xl()
                            .text_color(rgb(ACCENT))
                            .child(format!("{progress:.1}%")),
                    ),
            )
            .child(div().text_xs().text_color(rgb(MUTED)).child(detail))
            .child(
                div()
                    .w_full()
                    .h(px(8.))
                    .rounded_full()
                    .bg(rgb(PANEL_ALT))
                    .child(
                        div()
                            .h_full()
                            .rounded_full()
                            .bg(rgb(ACCENT))
                            .w(gpui::relative(progress as f32 / 100.0)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(stat("字串總數", self.stats.total_units.to_string(), TEXT))
                    .child(stat("Lang", self.stats.lang_units.to_string(), TEXT))
                    .child(stat(
                        "FTB Quests",
                        self.stats.inline_units.to_string(),
                        TEXT,
                    ))
                    .child(stat("KubeJS", self.stats.script_units.to_string(), TEXT))
                    .child(stat("專案已譯", self.translated.to_string(), GREEN))
                    .child(stat(
                        "本次新譯",
                        self.translated_this_run
                            .saturating_sub(self.reused_this_run)
                            .to_string(),
                        GREEN,
                    ))
                    .child(stat("本次快取", self.reused_this_run.to_string(), ACCENT))
                    .child(stat(
                        "本次失敗",
                        self.failed_this_run.to_string(),
                        if self.failed_this_run == 0 { TEXT } else { RED },
                    )),
            );
        stats
    }
}
