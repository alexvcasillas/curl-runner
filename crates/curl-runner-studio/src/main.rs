//! curl-runner studio: a macOS desktop app for YAML request suites.

// Off macOS nothing renders, and `method_color` waits for the request tree.
#[allow(dead_code)]
mod theme;

#[cfg(target_os = "macos")]
mod app {
    use crate::theme;
    use gpui_kit::*;

    struct Studio;

    impl Render for Studio {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().bg(rgba(theme::color("bg-100")))
        }
    }

    fn rgba(c: theme::Rgba) -> Rgba {
        Rgba {
            r: c.r,
            g: c.g,
            b: c.b,
            a: c.a,
        }
    }

    pub fn run() {
        application().run(|cx| {
            gpui_kit::init(cx);
            let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |_, cx| cx.new(|_| Studio))
                .expect("failed to open the studio window");
            cx.activate(true);
        });
    }
}

#[cfg(target_os = "macos")]
fn main() {
    app::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("curl-runner studio is macOS only");
    std::process::exit(1);
}
