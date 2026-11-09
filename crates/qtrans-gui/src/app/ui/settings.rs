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
            ));

        let settings_details = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(provider_choices)
            .child(setting_field(
                SettingFieldOptions {
                    id: "setting-service",
                    label: "翻譯服務",
                    value: &self.service,
                    placeholder: "http://127.0.0.1:8787",
                    masked: false,
                    active: matches!(self.editing, Some(ConfigField::Service)),
                    cursor: self.cursor,
                    select_all: self.select_all,
                    enabled: !busy,
                    focus_handle: &self.focus_handle,
                },
                cx.listener(|this, event, window, cx| {
                    this.activate_field(ConfigField::Service, event, window, cx)
                }),
                cx.listener(|this, event, window, cx| {
                    this.edit_field(ConfigField::Service, event, window, cx)
                }),
            ))
            .when(self.provider == "openai", |this| {
                this.child(setting_field(
                    SettingFieldOptions {
                        id: "setting-openai-base-url",
                        label: "OpenAI Base URL",
                        value: &self.openai_base_url,
                        placeholder: "https://api.openai.com/v1",
                        masked: false,
                        active: matches!(self.editing, Some(ConfigField::OpenAiBaseUrl)),
                        cursor: self.cursor,
                        select_all: self.select_all,
                        enabled: !busy,
                        focus_handle: &self.focus_handle,
                    },
                    cx.listener(|this, event, window, cx| {
                        this.activate_field(ConfigField::OpenAiBaseUrl, event, window, cx)
                    }),
                    cx.listener(|this, event, window, cx| {
                        this.edit_field(ConfigField::OpenAiBaseUrl, event, window, cx)
                    }),
                ))
                .child(setting_field(
                    SettingFieldOptions {
                        id: "setting-model",
                        label: "模型",
                        value: &self.model,
                        placeholder: "gpt-4o-mini",
                        masked: false,
                        active: matches!(self.editing, Some(ConfigField::Model)),
                        cursor: self.cursor,
                        select_all: self.select_all,
                        enabled: !busy,
                        focus_handle: &self.focus_handle,
                    },
                    cx.listener(|this, event, window, cx| {
                        this.activate_field(ConfigField::Model, event, window, cx)
                    }),
                    cx.listener(|this, event, window, cx| {
                        this.edit_field(ConfigField::Model, event, window, cx)
                    }),
                ))
                .child(setting_field(
                    SettingFieldOptions {
                        id: "setting-openai-key",
                        label: "API 金鑰（本次執行暫存）",
                        value: &self.openai_api_key,
                        placeholder: "未設定",
                        masked: true,
                        active: matches!(self.editing, Some(ConfigField::OpenAiKey)),
                        cursor: self.cursor,
                        select_all: self.select_all,
                        enabled: !busy,
                        focus_handle: &self.focus_handle,
                    },
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
                    SettingFieldOptions {
                        id: "setting-deepl-key",
                        label: "API 金鑰（本次執行暫存）",
                        value: &self.deepl_api_key,
                        placeholder: "未設定",
                        masked: true,
                        active: matches!(self.editing, Some(ConfigField::DeepLKey)),
                        cursor: self.cursor,
                        select_all: self.select_all,
                        enabled: !busy,
                        focus_handle: &self.focus_handle,
                    },
                    cx.listener(|this, event, window, cx| {
                        this.activate_field(ConfigField::DeepLKey, event, window, cx)
                    }),
                    cx.listener(|this, event, window, cx| {
                        this.edit_field(ConfigField::DeepLKey, event, window, cx)
                    }),
                ))
            })
            .child(setting_field(
                SettingFieldOptions {
                    id: "setting-target",
                    label: "目標語言",
                    value: &self.target,
                    placeholder: "zh_tw",
                    masked: false,
                    active: matches!(self.editing, Some(ConfigField::Target)),
                    cursor: self.cursor,
                    select_all: self.select_all,
                    enabled: !busy,
                    focus_handle: &self.focus_handle,
                },
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
                        SettingFieldOptions {
                            id: "setting-limit",
                            label: "翻譯上限（留空不限）",
                            value: &self.limit,
                            placeholder: "不限",
                            masked: false,
                            active: matches!(self.editing, Some(ConfigField::Limit)),
                            cursor: self.cursor,
                            select_all: self.select_all,
                            enabled: !busy,
                            focus_handle: &self.focus_handle,
                        },
                        cx.listener(|this, event, window, cx| {
                            this.activate_field(ConfigField::Limit, event, window, cx)
                        }),
                        cx.listener(|this, event, window, cx| {
                            this.edit_field(ConfigField::Limit, event, window, cx)
                        }),
                    ))
                    .child(setting_field(
                        SettingFieldOptions {
                            id: "setting-batch-size",
                            label: "每批字串數",
                            value: &self.batch_size,
                            placeholder: "16",
                            masked: false,
                            active: matches!(self.editing, Some(ConfigField::BatchSize)),
                            cursor: self.cursor,
                            select_all: self.select_all,
                            enabled: !busy,
                            focus_handle: &self.focus_handle,
                        },
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

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_lg()
            .bg(rgb(PANEL))
            .border_1()
            .border_color(rgb(BORDER))
            .child(
                div().flex().items_center().justify_between().child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_sm().text_color(rgb(TEXT)).child("翻譯設定"))
                        .child(div().text_xs().text_color(rgb(MUTED)).child(format!(
                            "{} · {} · 每批 {} 條",
                            provider_name, self.target, self.batch_size
                        ))),
                ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(ORANGE))
                    .child(provider_data_notice(&self.provider)),
            )
            .child(settings_details)
    }
}
