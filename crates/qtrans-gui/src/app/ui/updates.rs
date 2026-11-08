use super::components::button;
use crate::app::updates::UpdateStatus;
use crate::app::QTransApp;
use crate::theme::*;
use gpui::{div, prelude::*, px, rgb, Context, IntoElement};

impl QTransApp {
    pub(super) fn render_updates_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.update_status.clone();
        let visible = !matches!(&status, UpdateStatus::Idle | UpdateStatus::Checking);
        let (message, color) = match &status {
            UpdateStatus::Idle | UpdateStatus::Checking => (String::new(), MUTED),
            UpdateStatus::NoRelease => (
                "尚未發布可供檢查的版本；之後可再按「檢查更新」。".to_string(),
                MUTED,
            ),
            UpdateStatus::UpToDate => (
                format!(
                    "目前使用最新版 v{}。",
                    super::super::updates::CURRENT_VERSION
                ),
                GREEN,
            ),
            UpdateStatus::Available { version, .. } => {
                (format!("發現新版本 v{version}，可下載並安裝。"), ACCENT)
            }
            UpdateStatus::Downloading { version } => {
                (format!("正在下載 v{version} 安裝程式…"), ORANGE)
            }
            UpdateStatus::ReadyToInstall { version, .. } => {
                (format!("v{version} 已下載完成，準備啟動安裝程式。"), GREEN)
            }
            UpdateStatus::Error(error) => (error.clone(), RED),
        };

        div().when(visible, |panel| {
            panel
                .flex()
                .items_center()
                .justify_between()
                .flex_wrap()
                .gap_3()
                .p_4()
                .rounded_lg()
                .bg(rgb(PANEL))
                .border_1()
                .border_color(rgb(BORDER))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(220.))
                        .text_sm()
                        .text_color(rgb(color))
                        .child(message),
                )
                .when(matches!(&status, UpdateStatus::Available { .. }), |panel| {
                    if let UpdateStatus::Available { version, url } = status.clone() {
                        panel.child(button(
                            "download-update",
                            "下載並安裝",
                            ACCENT,
                            !self.busy,
                            cx.listener(move |this, _, _, cx| {
                                this.download_update(version.clone(), url.clone(), cx)
                            }),
                        ))
                    } else {
                        panel
                    }
                })
                .when(
                    matches!(&status, UpdateStatus::ReadyToInstall { .. }),
                    |panel| {
                        if let UpdateStatus::ReadyToInstall { path, .. } = status.clone() {
                            panel.child(button(
                                "launch-update-installer",
                                "執行安裝程式",
                                ACCENT,
                                !self.busy,
                                cx.listener(move |this, _, _, cx| {
                                    this.launch_update(path.clone(), cx)
                                }),
                            ))
                        } else {
                            panel
                        }
                    },
                )
        })
    }
}
