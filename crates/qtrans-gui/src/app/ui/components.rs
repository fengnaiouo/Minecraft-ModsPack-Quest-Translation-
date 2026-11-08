use crate::theme::*;
use gpui::{
    div, prelude::*, px, rgb, App, ClickEvent, CursorStyle, FocusHandle, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, Window,
};

pub(super) fn button(
    id: &'static str,
    label: impl IntoElement,
    tone: u32,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex_none()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(rgb(tone))
        .text_sm()
        .text_color(rgb(if tone == PANEL_ALT { TEXT } else { 0x10131a }))
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|style| style.opacity(0.88))
                .active(|style| style.opacity(0.7))
                .on_click(on_click)
        })
        .when(!enabled, |this| this.opacity(0.45))
        .child(label)
}

pub(super) fn provider_button(
    id: &'static str,
    label: &'static str,
    provider: &'static str,
    selected: &str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let active = selected == provider;
    button(
        id,
        label,
        if active { ACCENT } else { PANEL_ALT },
        enabled,
        on_click,
    )
}

pub(super) fn setting_field(
    id: &'static str,
    label: &'static str,
    value: &str,
    placeholder: &'static str,
    masked: bool,
    active: bool,
    cursor: usize,
    select_all: bool,
    enabled: bool,
    focus_handle: &FocusHandle,
    on_focus: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    on_key: impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let display = if value.is_empty() {
        placeholder.to_string()
    } else if masked {
        "•".repeat(value.chars().count())
    } else {
        value.to_string()
    };
    let display_cursor = if masked {
        value
            .char_indices()
            .take_while(|(index, _)| *index < cursor)
            .count()
    } else {
        cursor.min(display.len())
    };
    let display_cursor = display
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(display.len()))
        .filter(|index| *index <= display_cursor)
        .last()
        .unwrap_or(0);
    let (before, after) = display.split_at(display_cursor);
    div()
        .flex()
        .flex_col()
        .gap_1()
        .w_full()
        .child(div().text_xs().text_color(rgb(MUTED)).child(label))
        .child(
            div()
                .id(id)
                .track_focus(focus_handle)
                .px_2()
                .py_2()
                .rounded_md()
                .bg(rgb(PANEL_ALT))
                .border_1()
                .border_color(rgb(if active { ACCENT } else { BORDER }))
                .text_sm()
                .text_color(rgb(if value.is_empty() { MUTED } else { TEXT }))
                .when(enabled, |this| {
                    this.cursor(CursorStyle::IBeam)
                        .on_mouse_down(MouseButton::Left, on_focus)
                        .on_key_down(on_key)
                })
                .when(!enabled, |this| this.opacity(0.5))
                .flex()
                .items_center()
                .when(select_all, |this| {
                    this.bg(rgb(ACCENT)).child(display.clone())
                })
                .when(!select_all, |this| {
                    this.child(before.to_string())
                        .child(div().w(px(1.)).h(px(14.)).bg(rgb(if active {
                            ACCENT
                        } else {
                            PANEL_ALT
                        })))
                        .child(after.to_string())
                }),
        )
}

pub(super) fn stat(label: &'static str, value: String, color: u32) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .w(px(88.))
        .px_3()
        .py_2()
        .rounded_md()
        .bg(rgb(PANEL_ALT))
        .border_1()
        .border_color(rgb(BORDER))
        .child(div().text_xs().text_color(rgb(MUTED)).child(label))
        .child(div().text_lg().text_color(rgb(color)).child(value))
}

pub(super) fn path_line(label: &'static str, value: String) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .w(px(64.))
                .text_sm()
                .text_color(rgb(MUTED))
                .child(label),
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .text_sm()
                .text_color(rgb(TEXT))
                .child(value),
        )
}
