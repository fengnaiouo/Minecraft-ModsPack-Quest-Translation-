#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod theme;

use gpui::{
    prelude::*, px, size, App, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions,
};

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1100.), px(780.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(620.), px(560.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("QTrans — Minecraft 任務漢化工具".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| app::QTransApp::new(cx)),
        )
        .expect("open window");
        cx.activate(true);
    });
}
