use super::components::*;
use crate::app::config::ConfigField;
use crate::app::QTransApp;
use crate::theme::*;
use gpui::{div, prelude::*, rgb, Context, IntoElement};

impl QTransApp {
    pub(super) fn render_settings_panel(
        &mut self,
        provider_name: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let busy = self.busy;
        let provider_choices = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(provider_button(
                "provider-google",
                "Google",
                "google_free",
                &self.provider,
                !busy,
                cx.listener(|this, _, _, cx| this.set_provider("google_free", cx)),
            ))
            .child(provider_button(
                "provider-openai",
                "OpenAI",
                "openai",
                &self.provider,
                !busy,
                cx.listener(|this, _, _, cx| this.set_provider("openai", cx)),
            ))
            .child(provider_button(
                "provider-deepl",
                "DeepL",
                "deepl",
                &self.provider,
                !busy,
                cx.listener(|this, _, _, cx| this.set_provider("deepl", cx)),
            ))
            .child(provider_button(
                "provider-offline",
                "離線",
                "offline",
                &self.provider,
                !busy,
                cx.listener(|this, _, _, cx| this.set_provider("offline", cx)),
            ))
            .child(provider_button(
                "provider-mock",
                "測試",
                "mock",
                &self.provider,
                !busy,
                cx.listener(|this, _, _, cx| this.set_provider("mock", cx)),
            ));

        let settings_details = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(provider_choices)
            .child(setting_field(
                "setting-service",
                "翻譯服務",
                &self.service,
                "http://127.0.0.1:8787",
                false,
                matches!(self.editing, Some(ConfigField::Service)),
                self.cursor,
                self.select_all,
                !busy,
                &self.focus_handle,
                cx.listener(|this, event, window, cx| {
                    this.activate_field(ConfigField::Service, event, window, cx)
                }),
                cx.listener(|this, event, window, cx| {
                    this.edit_field(ConfigField::Service, event, window, cx)
                }),
            ))
            .when(self.provider == "openai", |this| {
                this.child(setting_field(
                    "setting-openai-base-url",
                    "OpenAI Base URL",
                    &self.openai_base_url,
                    "https://api.openai.com/v1",
                    false,
                    matches!(self.editing, Some(ConfigField::OpenAiBaseUrl)),
                    self.cursor,
                    self.select_all,
                    !busy,
                    &self.focus_handle,
                    cx.listener(|this, event, window, cx| {
                        this.activate_field(ConfigField::OpenAiBaseUrl, event, window, cx)
                    }),
                    cx.listener(|this, event, window, cx| {
                        this.edit_field(ConfigField::OpenAiBaseUrl, event, window, cx)
                    }),
                ))
                .child(setting_field(
                    "setting-model",
                    "模型",
                    &self.model,
                    "gpt-4o-mini",
                    false,
                    matches!(self.editing, Some(ConfigField::Model)),
                    self.cursor,
                    self.select_all,
                    !busy,
                    &self.focus_handle,
                    cx.listener(|this, event, window, cx| {
                        this.activate_field(ConfigField::Model, event, window, cx)
                    }),
                    cx.listener(|this, event, window, cx| {
                        this.edit_field(ConfigField::Model, event, window, cx)
                    }),
                ))
                .child(setting_field(
                    "setting-openai-key",
                    "API 金鑰（本次執行暫存）",
                    &self.openai_api_key,
                    "未設定",
                    true,
                    matches!(self.editing, Some(ConfigField::OpenAiKey)),
                    self.cursor,
                    self.select_all,
                    !busy,
                    &self.focus_handle,
                    cx.listener(|this, event, window, cx| {
                        this.activate_field(ConfigField::OpenAiKey, event, window, cx)
                    }),
                    cx.listener(|this, event, window, cx| {
                        this.edit_field(ConfigField::OpenAiKey, event, window, cx)
                    }),
                ))
            })
            .when(self.provider == "deepl", |this| {
                this.child(setting_field(
                    "setting-deepl-key",
                    "API 金鑰（本次執行暫存）",
                    &self.deepl_api_key,
                    "未設定",
                    true,
                    matches!(self.editing, Some(ConfigField::DeepLKey)),
                    self.cursor,
                    self.select_all,
                    !busy,
                    &self.focus_handle,
                    cx.listener(|this, event, window, cx| {
                        this.activate_field(ConfigField::DeepLKey, event, window, cx)
                    }),
                    cx.listener(|this, event, window, cx| {
                        this.edit_field(ConfigField::DeepLKey, event, window, cx)
                    }),
                ))
            })
            .child(setting_field(
                "setting-target",
                "目標語言",
                &self.target,
                "zh_tw",
                false,
                matches!(self.editing, Some(ConfigField::Target)),
                self.cursor,
                self.select_all,
                !busy,
                &self.focus_handle,
                cx.listener(|this, event, window, cx| {
                    this.activate_field(ConfigField::Target, event, window, cx)
                }),
                cx.listener(|this, event, window, cx| {
                    this.edit_field(ConfigField::Target, event, window, cx)
                }),
            ))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(setting_field(
                        "setting-limit",
                        "翻譯上限（留空不限）",
                        &self.limit,
                        "不限",
                        false,
                        matches!(self.editing, Some(ConfigField::Limit)),
                        self.cursor,
                        self.select_all,
                        !busy,
                        &self.focus_handle,
                        cx.listener(|this, event, window, cx| {
                            this.activate_field(ConfigField::Limit, event, window, cx)
                        }),
                        cx.listener(|this, event, window, cx| {
                            this.edit_field(ConfigField::Limit, event, window, cx)
                        }),
                    ))
                    .child(setting_field(
                        "setting-batch-size",
                        "每批字串數",
                        &self.batch_size,
                        "16",
                        false,
                        matches!(self.editing, Some(ConfigField::BatchSize)),
                        self.cursor,
                        self.select_all,
                        !busy,
                        &self.focus_handle,
                        cx.listener(|this, event, window, cx| {
                            this.activate_field(ConfigField::BatchSize, event, window, cx)
                        }),
                        cx.listener(|this, event, window, cx| {
                            this.edit_field(ConfigField::BatchSize, event, window, cx)
                        }),
                    )),
            )
            .child(button(
                "toggle-overwrite",
                if self.overwrite {
                    "覆寫既有翻譯：開"
                } else {
                    "覆寫既有翻譯：關"
                },
                if self.overwrite { ORANGE } else { PANEL_ALT },
                !busy,
                cx.listener(|this, _, _, cx| this.toggle_overwrite(cx)),
            ));

        let settings = div()
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
                            .child(div().text_sm().text_color(rgb(TEXT)).child("翻譯設定"))
                            .child(div().text_xs().text_color(rgb(MUTED)).child(format!(
                                "{} · {} · 每批 {} 條",
                                provider_name, self.target, self.batch_size
                            ))),
                    )
                    .child(button(
                        "toggle-settings",
                        if self.show_settings {
                            "完成"
                        } else {
                            "編輯"
                        },
                        if self.show_settings {
                            ACCENT
                        } else {
                            PANEL_ALT
                        },
                        !busy,
                        cx.listener(|this, _, _, cx| this.toggle_settings(cx)),
                    )),
            )
            .when(self.show_settings, |this| this.child(settings_details));
        settings
    }
}
