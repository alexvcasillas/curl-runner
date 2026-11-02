//! The first-run walkthrough (mockups `TourRun` and `TourResult`): a
//! callout over the workspace that creates `~/curl-runner/getting-started`,
//! runs its `health.yaml` through the Run button's pipeline and reads the
//! result.
//!
//! The callout sits at fixed offsets from the window's top-right corner:
//! under the title bar's Run button (always the rightmost control) for the
//! first two steps, and beside the inspector (fixed width) for the result.
//! Both anchors follow from the layout, so it never measures elements.

use super::app::{TITLE_BAR_HEIGHT, mono_font};
use super::color;
use super::inspector::INSPECTOR_WIDTH;
use super::new_dialog::display_path;
use super::settings_kit::{primary_button, small_button};
use super::workspace::{Workspace, notify_error};
use crate::model::run::RunTarget;
use crate::model::tour::{
    HEALTH_FILE, HEALTH_URL, TourOutcome, TourStatus, TourStep, ensure_getting_started,
    ensure_getting_started_with, getting_started_dir, health_yaml, tour_outcome,
};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// The mockups' callout width.
const CALLOUT_WIDTH: f32 = 320.;
/// Gap between the callout and what it points at.
const CALLOUT_GAP: f32 = 12.;

/// What a callout button asks of the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TourEvent {
    Create,
    /// Run, or Try again.
    Run,
    Back,
    Skip,
    /// Done, or Finish anyway.
    Done,
    NewRequest,
}

pub struct TourView {
    home: PathBuf,
    url: String,
    step: TourStep,
    outcome: Option<TourOutcome>,
}

impl EventEmitter<TourEvent> for TourView {}

impl TourView {
    /// The walkthrough on `home`'s `curl-runner/getting-started`, its
    /// request sent to `url` ([`HEALTH_URL`] outside tests).
    pub fn new(home: PathBuf, url: impl Into<String>) -> Self {
        TourView {
            home,
            url: url.into(),
            step: TourStep::Create,
            outcome: None,
        }
    }

    pub fn step(&self) -> TourStep {
        self.step
    }

    #[cfg(test)]
    pub fn outcome(&self) -> Option<&TourOutcome> {
        self.outcome.as_ref()
    }

    /// The collection the walkthrough creates.
    pub fn dir(&self) -> PathBuf {
        getting_started_dir(&self.home)
    }

    /// Creates [`TourView::dir`] with `health.yaml`, never replacing it:
    /// exactly [`HEALTH_YAML`](crate::model::tour::HEALTH_YAML) unless the
    /// request goes elsewhere.
    fn ensure(&self) -> std::io::Result<PathBuf> {
        let dir = self.dir();
        if self.url == HEALTH_URL {
            ensure_getting_started(&dir)
        } else {
            ensure_getting_started_with(&dir, &health_yaml(&self.url))
        }
    }

    fn set_step(&mut self, step: TourStep, cx: &mut Context<Self>) {
        self.step = step;
        if step != TourStep::Result {
            self.outcome = None;
        }
        cx.notify();
    }

    fn show_result(&mut self, outcome: TourOutcome, cx: &mut Context<Self>) {
        self.step = TourStep::Result;
        self.outcome = Some(outcome);
        cx.notify();
    }

    /// Title, body (with `code` spans) and the footer's link and buttons.
    fn copy(&self) -> CalloutCopy {
        match (self.step, &self.outcome) {
            (TourStep::Create, _) => CalloutCopy {
                title: "Create a collection".into(),
                body: format!(
                    "We'll make `{}` with one request file in it. It's plain YAML, so the CLI \
                     runs it too.",
                    display_path(&self.dir())
                ),
                link: ("tour-skip", "Skip walkthrough", TourEvent::Skip),
                secondary: None,
                primary: ("tour-create", "Create", TourEvent::Create),
            },
            (TourStep::Run, _) => CalloutCopy {
                title: "Run it".into(),
                body: "This file sends a `GET` to our health endpoint and expects a `200` with \
                       `status: ok` in the body. Press Run or ⌘↵."
                    .into(),
                link: ("tour-skip", "Skip walkthrough", TourEvent::Skip),
                secondary: Some(("tour-back", "Back", TourEvent::Back)),
                primary: ("tour-run", "Run", TourEvent::Run),
            },
            (TourStep::Result, None | Some(TourOutcome::Passed)) => CalloutCopy {
                title: "It passed. Now try your own API".into(),
                body: "Both checks held, so this file would pass in CI too, with \
                       `curl-runner getting-started/`. Add a request for one of your own \
                       endpoints and see what comes back."
                    .into(),
                link: ("tour-back", "Back", TourEvent::Back),
                secondary: Some(("tour-done", "Done", TourEvent::Done)),
                primary: ("tour-new-request", "New request", TourEvent::NewRequest),
            },
            (TourStep::Result, Some(outcome)) => CalloutCopy {
                title: match outcome {
                    TourOutcome::Unreachable { .. } => "No response",
                    TourOutcome::CheckFailed(_) => "A check failed",
                    _ => "It didn't run",
                }
                .into(),
                body: outcome.message(),
                link: ("tour-back", "Back", TourEvent::Back),
                secondary: Some(("tour-finish-anyway", "Finish anyway", TourEvent::Done)),
                primary: ("tour-try-again", "Try again", TourEvent::Run),
            },
        }
    }
}

/// A button: its id, label and event.
type TourButton = (&'static str, &'static str, TourEvent);

struct CalloutCopy {
    title: String,
    body: String,
    link: TourButton,
    secondary: Option<TourButton>,
    primary: TourButton,
}

impl Render for TourView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let copy = self.copy();
        let right = match self.step {
            TourStep::Result => INSPECTOR_WIDTH + CALLOUT_GAP,
            TourStep::Create | TourStep::Run => CALLOUT_GAP,
        };
        let emit = |event: TourEvent| cx.listener(move |_, _, _, cx| cx.emit(event));
        let (link_id, link_label, link_event) = copy.link;
        let link = div()
            .id(link_id)
            .role(Role::Button)
            .test_support()
            .aria_label(link_label)
            .text_size(px(12.))
            .text_color(color("text-tertiary"))
            .hover(|s| s.text_color(color("text-primary")))
            .cursor_pointer()
            .on_click(emit(link_event))
            .child(link_label);
        let buttons = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .children(copy.secondary.map(|(id, label, event)| {
                small_button(id, label).on_click(emit(event)).child(label)
            }))
            .child({
                let (id, label, event) = copy.primary;
                primary_button(id, label).on_click(emit(event)).child(label)
            });
        div()
            .id("tour")
            .role(Role::Dialog)
            .test_support()
            .aria_label("Walkthrough")
            .occlude()
            .absolute()
            .top(px(TITLE_BAR_HEIGHT + CALLOUT_GAP))
            .right(px(right))
            .w(px(CALLOUT_WIDTH))
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(16.))
            .rounded(px(10.))
            .bg(color("surface-raised"))
            .border_1()
            .border_color(color("accent-blue"))
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., 0.6),
                offset: point(px(0.), px(8.)),
                blur_radius: px(24.),
                spread_radius: px(0.),
                inset: false,
            }])
            .child(
                div()
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(color("accent-blue"))
                    .child(format!("STEP {} OF 3", self.step.number())),
            )
            .child(
                div()
                    .id("tour-title")
                    .role(Role::Heading)
                    .test_support()
                    .aria_label(copy.title.clone())
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color("text-primary"))
                    .child(copy.title),
            )
            .child(
                div()
                    .id("tour-body")
                    .test_support()
                    .aria_label(copy.body.replace('`', ""))
                    .child(prose(&copy.body).text_color(color("text-secondary"))),
            )
            .child(
                div()
                    .mt(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(link)
                    .child(buttons),
            )
    }
}

/// 12px text with its `` `code` `` spans in mono, laid out word by word so
/// it wraps.
pub(super) fn prose(text: &str) -> Div {
    let mut line = div()
        .flex()
        .flex_wrap()
        .items_center()
        .min_w_0()
        .text_size(px(12.))
        .line_height(relative(1.5));
    for (i, span) in text.split('`').enumerate() {
        if i % 2 == 1 {
            line = line.child(
                div()
                    .flex_none()
                    .px(px(3.))
                    .rounded(px(3.))
                    .bg(color("surface-300"))
                    .font_family(mono_font())
                    .text_size(px(11.))
                    .child(span.to_string()),
            );
            continue;
        }
        for word in span.split_inclusive(' ') {
            line = line.child(
                div()
                    .flex_none()
                    .whitespace_nowrap()
                    .child(word.to_string()),
            );
        }
    }
    line
}

impl Workspace {
    /// Starts the walkthrough at step 1, whatever its status (Welcome's
    /// card, the palette, Help › Walkthrough).
    pub(super) fn start_walkthrough(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match std::env::home_dir() {
            Some(home) => drop(self.show_tour(home, HEALTH_URL.to_string(), window, cx)),
            None => notify_error("Couldn't find your home folder.".into(), window, cx),
        }
    }

    /// The walkthrough on `home`, its request sent to `url`.
    pub(super) fn show_tour(
        &mut self,
        home: PathBuf,
        url: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TourView> {
        let view = cx.new(|_| TourView::new(home, url));
        self._tour_events = Some(cx.subscribe_in(&view, window, Self::on_tour_event));
        self.tour = Some(view.clone());
        cx.notify();
        view
    }

    /// The walkthrough on `fill`'s `home|url` (dev hook), at `show`:
    /// `tour`, `tour-run` (created) or `tour-result` (run).
    pub(super) fn dev_tour(
        &mut self,
        fill: &str,
        show: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (home, url) = fill.split_once('|').unwrap_or((fill, ""));
        let home = Some(PathBuf::from(home))
            .filter(|h| !h.as_os_str().is_empty())
            .or_else(std::env::home_dir);
        let url = Some(url).filter(|u| !u.is_empty()).unwrap_or(HEALTH_URL);
        let Some(home) = home else { return };
        let tour = self.show_tour(home, url.to_string(), window, cx);
        match show {
            "tour-run" => self.tour_create(tour, false, window, cx),
            "tour-result" => self.tour_create(tour, true, window, cx),
            _ => {}
        }
    }

    /// Whether the walkthrough is on `step`.
    pub(super) fn tour_on(&self, step: TourStep, cx: &App) -> bool {
        self.tour
            .as_ref()
            .is_some_and(|t| t.read(cx).step() == step)
    }

    fn on_tour_event(
        &mut self,
        tour: &Entity<TourView>,
        event: &TourEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            TourEvent::Create => self.tour_create(tour.clone(), false, window, cx),
            TourEvent::Run => self.tour_run(tour.clone(), window, cx),
            TourEvent::Back => tour.update(cx, |t, cx| {
                let back = match t.step {
                    TourStep::Result => TourStep::Run,
                    TourStep::Run | TourStep::Create => TourStep::Create,
                };
                t.set_step(back, cx);
            }),
            TourEvent::Skip => self.end_tour(TourStatus::Skipped, cx),
            TourEvent::Done => self.end_tour(TourStatus::Done, cx),
            TourEvent::NewRequest => {
                self.end_tour(TourStatus::Done, cx);
                drop(self.new_request(None, window, cx));
            }
        }
    }

    fn end_tour(&mut self, status: TourStatus, cx: &mut Context<Self>) {
        self.store.set_tour_status(status);
        self.tour_status = status;
        self.tour = None;
        self._tour_events = None;
        cx.notify();
    }

    /// Step 1: creates the collection (never replacing `health.yaml`),
    /// opens it on `health.yaml`, then moves to step 2, or with `run`
    /// runs it.
    fn tour_create(
        &mut self,
        tour: Entity<TourView>,
        run: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dir = tour.read(cx).dir();
        if let Err(e) = tour.read(cx).ensure() {
            let message = format!("Couldn't create {}: {e}", display_path(&dir));
            notify_error(message, window, cx);
            return;
        }
        self.settle_unsaved(
            Rc::new(move |this, window, cx| {
                if !this.open_collection(&dir, window, cx) {
                    return;
                }
                this.remember_folder(&dir);
                this.open_file(Path::new(HEALTH_FILE), window, cx);
                tour.update(cx, |t, cx| t.set_step(TourStep::Run, cx));
                if run {
                    this.tour_start_run(&tour, window, cx);
                }
            }),
            window,
            cx,
        );
    }

    /// Step 2's Run and step 3's Try again: runs `health.yaml` as the Run
    /// button does, reopening the collection first if another one is open.
    fn tour_run(&mut self, tour: Entity<TourView>, window: &mut Window, cx: &mut Context<Self>) {
        let dir = tour.read(cx).dir();
        let file = Path::new(HEALTH_FILE);
        let open = self
            .state
            .collection
            .as_ref()
            .is_some_and(|c| c.root == dir);
        if !open || !dir.join(file).is_file() {
            return self.tour_create(tour, true, window, cx);
        }
        self.open_file(file, window, cx);
        tour.update(cx, |t, cx| t.set_step(TourStep::Run, cx));
        self.tour_start_run(&tour, window, cx);
    }

    /// Runs `health.yaml`. When no run starts (another one is in flight,
    /// or the file has problems) the walkthrough says why, so Run and Try
    /// again never do nothing.
    fn tour_start_run(
        &mut self,
        tour: &Entity<TourView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let busy = self.runs.active.is_some();
        self.run(RunTarget::File(HEALTH_FILE.into()), window, cx);
        if !busy && self.runs.active.is_some() {
            return;
        }
        let message = if busy {
            "Another run is still going. Wait for it to finish, then try again."
        } else {
            "health.yaml has problems. Fix them in the Problems panel, then try again."
        };
        tour.update(cx, |t, cx| {
            t.show_result(TourOutcome::Other(message.into()), cx)
        });
    }

    /// A run finished; `ran_health` when it ran `health.yaml`. On step 2,
    /// in the walkthrough's collection, the walkthrough shows how it went.
    pub(super) fn tour_run_finished(&mut self, ran_health: bool, cx: &mut Context<Self>) {
        let Some(tour) = self.tour.clone() else {
            return;
        };
        let dir = tour.read(cx).dir();
        let ours = self
            .state
            .collection
            .as_ref()
            .is_some_and(|c| c.root == dir);
        if !ran_health || !ours || tour.read(cx).step() != TourStep::Run {
            return;
        }
        let Some(run) = self.runs.last.get(Path::new(HEALTH_FILE)) else {
            return;
        };
        let outcome = tour_outcome(run.results.get(&0), run.load_error.as_deref());
        tour.update(cx, |t, cx| t.show_result(outcome, cx));
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::StateStore;
    use super::super::workspace::{Workspace, test_secrets, wait_for_run};
    use super::TourView;
    use crate::model::backend::test_backend;
    use crate::model::tour::{TourOutcome, TourStatus, TourStep, getting_started_dir};
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };
    use std::path::Path;

    fn open(cx: &mut TestAppContext, store: StateStore) -> (AnyWindowHandle, Entity<Workspace>) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            super::super::app::init(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::default(),
                    size: size(px(1440.), px(900.)),
                })),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| Workspace::new(None, store, test_secrets(), test_backend(), window, cx))
            })
            .unwrap()
        })
    }

    fn click(cx: &mut TestAppContext, window: AnyWindowHandle, id: &'static str) {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.click(id, cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    fn shows(cx: &mut TestAppContext, window: AnyWindowHandle, label: &str) -> bool {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("workspace")
                .try_find_by_label(label)
                .is_some()
        })
        .unwrap()
    }

    fn tour(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> Option<Entity<TourView>> {
        ws.read_with(cx, |ws, _| ws.tour.clone())
    }

    /// The walkthrough on a temporary home, sending its request to `url`.
    fn start(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        ws: &Entity<Workspace>,
        home: &Path,
        url: String,
    ) {
        let home = home.to_path_buf();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| drop(ws.show_tour(home, url, window, cx)));
        })
        .unwrap();
    }

    /// Create, then Run, until the run is over.
    fn create_and_run(cx: &mut TestAppContext, window: AnyWindowHandle, ws: &Entity<Workspace>) {
        click(cx, window, "tour-create");
        let t = tour(cx, ws).unwrap();
        assert_eq!(t.read_with(cx, |t, _| t.step()), TourStep::Run);
        assert!(shows(cx, window, "Run it"));
        click(cx, window, "tour-run");
        wait_for_run(cx, ws);
        cx.run_until_parked();
    }

    #[gpui_kit::test]
    fn welcome_shows_card_until_done_or_skipped(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let (window, ws) = open(cx, store.clone());
        assert!(shows(cx, window, "Run your first request"));
        click(cx, window, "welcome-tour-start");
        let t = tour(cx, &ws).expect("Start opens the walkthrough");
        assert_eq!(t.read_with(cx, |t, _| t.step()), TourStep::Create);
        assert!(shows(cx, window, "Create a collection"));
        click(cx, window, "tour-skip");
        assert!(tour(cx, &ws).is_none());
        assert_eq!(store.tour_status(), TourStatus::Skipped);
        assert!(!shows(cx, window, "Run your first request"));
        // The next launch neither.
        let (window, _) = open(cx, store.clone());
        assert!(!shows(cx, window, "Run your first request"));

        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        store.set_tour_status(TourStatus::Done);
        let (window, _) = open(cx, store);
        assert!(!shows(cx, window, "Run your first request"));
    }

    #[gpui_kit::test]
    fn tour_pass_reaches_result_step(cx: &mut TestAppContext) {
        let srv = testkit::TestServer::with_handler(|_| {
            testkit::Response::from_data(br#"{"status":"ok"}"#.to_vec())
                .with_header("Content-Type", "application/json")
        });
        let home = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let (window, ws) = open(cx, store.clone());
        start(
            cx,
            window,
            &ws,
            home.path(),
            format!("{}/health", srv.base_url),
        );
        create_and_run(cx, window, &ws);

        let dir = getting_started_dir(home.path());
        ws.read_with(cx, |ws, _| {
            assert_eq!(ws.state.collection.as_ref().unwrap().root, dir);
            let doc = ws.state.active_document().unwrap();
            assert_eq!(doc.rel_path, Path::new("health.yaml"));
        });
        let t = tour(cx, &ws).unwrap();
        t.read_with(cx, |t, _| {
            assert_eq!(t.step(), TourStep::Result);
            assert_eq!(t.outcome(), Some(&TourOutcome::Passed));
        });
        assert!(shows(cx, window, "It passed. Now try your own API"));
        click(cx, window, "tour-done");
        assert!(tour(cx, &ws).is_none());
        assert_eq!(store.tour_status(), TourStatus::Done);
    }

    #[gpui_kit::test]
    fn tour_run_with_lint_errors_says_so(cx: &mut TestAppContext) {
        let home = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let (window, ws) = open(cx, store);
        // A health.yaml someone broke earlier is never replaced.
        let dir = getting_started_dir(home.path());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("health.yaml"),
            "request:\n  url: http://127.0.0.1:1/health\n  headers: [\n",
        )
        .unwrap();
        start(
            cx,
            window,
            &ws,
            home.path(),
            "http://127.0.0.1:1/health".into(),
        );
        click(cx, window, "tour-create");
        click(cx, window, "tour-run");
        cx.run_until_parked();

        let t = tour(cx, &ws).unwrap();
        t.read_with(cx, |t, _| {
            assert_eq!(t.step(), TourStep::Result);
            assert_eq!(
                t.outcome(),
                Some(&TourOutcome::Other(
                    "health.yaml has problems. Fix them in the Problems panel, then try again."
                        .into()
                ))
            );
        });
        assert!(shows(cx, window, "Try again"));
        ws.read_with(cx, |ws, _| assert!(ws.runs.last.is_empty(), "nothing ran"));
    }

    #[gpui_kit::test]
    fn tour_connection_failure_explains_and_offers_retry(cx: &mut TestAppContext) {
        // A port nothing listens on.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let home = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let (window, ws) = open(cx, store.clone());
        start(
            cx,
            window,
            &ws,
            home.path(),
            format!("http://127.0.0.1:{port}/health"),
        );
        create_and_run(cx, window, &ws);

        let t = tour(cx, &ws).unwrap();
        let message = t.read_with(cx, |t, _| {
            assert_eq!(t.step(), TourStep::Result);
            t.outcome().unwrap().message()
        });
        assert!(message.starts_with("Couldn't reach 127.0.0.1"), "{message}");
        assert!(shows(cx, window, &message));
        assert!(shows(cx, window, "Try again"));
        let file = getting_started_dir(home.path()).join("health.yaml");
        assert!(file.is_file(), "the file stays on disk");

        // Try again runs it again, and fails the same way.
        click(cx, window, "tour-try-again");
        wait_for_run(cx, &ws);
        cx.run_until_parked();
        assert_eq!(t.read_with(cx, |t, _| t.step()), TourStep::Result);

        click(cx, window, "tour-finish-anyway");
        assert!(tour(cx, &ws).is_none());
        assert_eq!(store.tour_status(), TourStatus::Done);
        assert!(file.is_file());
    }

    #[gpui_kit::test]
    fn palette_replays_walkthrough(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        store.set_tour_status(TourStatus::Done);
        let (window, ws) = open(cx, store);
        assert!(tour(cx, &ws).is_none());
        let palette = cx
            .update_window(window, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.open_palette(window, cx))
            })
            .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            palette.update(cx, |p, cx| {
                p.set_query("walkthrough", window, cx);
                p.confirm(window, cx);
            })
        })
        .unwrap();
        cx.run_until_parked();
        let t = tour(cx, &ws).expect("the palette starts the walkthrough");
        assert_eq!(t.read_with(cx, |t, _| t.step()), TourStep::Create);
    }
}
