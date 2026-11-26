//! agentq desktop interface (GPUI). Build with `--features gui`.

mod assets;
mod ui_text;
mod workspace;

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{
    AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, hsla, point, px, size,
};
use std::path::PathBuf;
use workspace::Workspace;

fn main() {
    // `agentq-gui [PATH]` opens a project straight away.
    let initial: Option<PathBuf> = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .filter(|p| p.is_dir());

    gpui_kit::application()
        .with_assets(assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            Theme::change(ThemeMode::Dark, None, cx);
            let colors = &mut Theme::global_mut(cx).colors;
            colors.background = hsla(0., 0., 0.035, 1.);
            colors.sidebar = hsla(0., 0., 0.05, 1.);
            colors.border = hsla(0., 0., 1., 0.09);
            colors.list_active = hsla(0., 0., 1., 0.09);
            colors.accent = hsla(0., 0., 1., 0.05);
            // White, not blue, wherever the theme would reach for an accent.
            colors.primary = hsla(0., 0., 0.96, 1.);
            colors.primary_hover = hsla(0., 0., 0.86, 1.);
            colors.primary_active = hsla(0., 0., 0.78, 1.);
            colors.primary_foreground = hsla(0., 0., 0.06, 1.);
            colors.ring = hsla(0., 0., 1., 0.35);

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1240.), px(800.)),
                    cx,
                ))),
                // No system title bar: the sidebar and header run to the top
                // edge and the traffic lights float over them.
                titlebar: Some(TitlebarOptions {
                    title: Some("agentq".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(16.), px(16.))),
                }),
                window_min_size: Some(size(px(720.), px(560.))),
                ..Default::default()
            };

            gpui_kit::open_window(options, cx, |_, cx| {
                cx.new(|cx| Workspace::new(initial, cx))
            })
            .expect("failed to open the window");
            // Closing the last window ends the program, so the terminal that
            // started it comes back instead of staying stuck on a window-less app.
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.activate(true);
        });
}
