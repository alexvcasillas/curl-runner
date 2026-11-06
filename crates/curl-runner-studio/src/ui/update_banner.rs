//! The update banner over a received copy's editor: `Version 14 available ·
//! published 2 hours ago · 3 requests changed`, with **Update** and **What
//! changed**. When the customer edited their copy it adds `You've changed 2
//! files. Updating replaces them.` and **Keep my changes as a copy**.
//! Styled as the ReadOnly board's note bar it takes the place of. The
//! checks that fill it live in [`super::received`].

use super::color;
use super::icons::{self, glyph};
use super::workspace::Workspace;
use crate::model::backend::SharedVar;
use crate::model::history::relative_time;
use crate::model::shared_copy::UpdatePlan;
use chrono::{DateTime, Local, Utc};
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::PathBuf;
use std::time::Duration;

/// How often an open received copy asks for a newer version.
pub const UPDATE_EVERY: Duration = Duration::from_secs(15 * 60);

/// A newer version of a received copy, fetched and planned: what the
/// banner offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateOffer {
    pub version: u32,
    pub published_at: DateTime<Utc>,
    /// The new version against the copy as it was when checked.
    pub plan: UpdatePlan,
    /// Request files added, changed or removed.
    pub requests_changed: usize,
    /// Files the customer changed since the version they hold
    /// ([`local_changes`]): the banner warns, and offers a copy first.
    ///
    /// [`local_changes`]: crate::model::received::local_changes
    pub local_changes: usize,
    /// The inputs as the new version shares them.
    pub variables: Vec<SharedVar>,
}

impl UpdateOffer {
    pub fn new(
        version: u32,
        published_at: DateTime<Utc>,
        plan: UpdatePlan,
        variables: Vec<SharedVar>,
    ) -> UpdateOffer {
        UpdateOffer {
            version,
            published_at,
            // Every path in a plan is a YAML request file.
            requests_changed: plan.count(),
            local_changes: 0,
            plan,
            variables,
        }
    }
}

/// `2 hours ago`, `just now`, `yesterday`, `3 days ago`, `on Oct 23`: the
/// studio's relative time, spelled out for a sentence.
pub fn published_ago(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let unit = |n: i64, what: &str| {
        let s = if n == 1 { "" } else { "s" };
        format!("{n} {what}{s} ago")
    };
    let text = relative_time(at.with_timezone(&Local), now.with_timezone(&Local));
    let ago = now - at;
    if let Some(n) = text.strip_suffix(" min ago") {
        unit(n.parse().unwrap_or(ago.num_minutes()), "minute")
    } else if let Some(n) = text.strip_suffix(" h ago") {
        unit(n.parse().unwrap_or(ago.num_hours()), "hour")
    } else if text == "Yesterday" {
        "yesterday".into()
    } else if text.ends_with(" ago") || text == "just now" {
        text
    } else {
        format!("on {text}")
    }
}

/// `Version 14 available · published 2 hours ago · 3 requests changed`.
pub fn banner_text(offer: &UpdateOffer, now: DateTime<Utc>) -> String {
    let n = offer.requests_changed;
    let requests = if n == 1 { "request" } else { "requests" };
    format!(
        "Version {} available · published {} · {n} {requests} changed",
        offer.version,
        published_ago(offer.published_at, now)
    )
}

/// `You've changed 2 files. Updating replaces them.`: what the banner
/// adds when the customer edited their copy. `None` for no edits.
pub fn local_changes_text(n: usize) -> Option<String> {
    match n {
        0 => None,
        1 => Some("You've changed 1 file. Updating replaces it.".into()),
        n => Some(format!("You've changed {n} files. Updating replaces them.")),
    }
}

/// What changed, a line per file: `Added labels/new.yaml`, then the
/// changed and the removed ones.
pub fn change_lines(plan: &UpdatePlan) -> Vec<String> {
    let line = |what: &'static str| move |p: &PathBuf| format!("{what} {}", p.display());
    plan.added
        .iter()
        .map(line("Added"))
        .chain(plan.changed.iter().map(line("Changed")))
        .chain(plan.removed.iter().map(line("Removed")))
        .collect()
}

/// The note bar's height (the ReadOnly board's).
const BANNER_HEIGHT: f32 = 40.;

/// The Update button while an update (or a kept copy) runs.
pub const UPDATING: &str = "Updating…";

impl Workspace {
    /// The offer for the open received copy, if one is waiting.
    pub(super) fn open_offer(&self) -> Option<(&str, &UpdateOffer)> {
        let token = self.received.as_ref()?.share.link.token.as_str();
        Some((token, self.update_offers.get(token)?))
    }

    /// The banner for the open copy, when a newer version waits.
    pub(super) fn render_update_banner(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (token, offer) = self.open_offer()?;
        let text = banner_text(offer, Utc::now());
        let local = local_changes_text(offer.local_changes);
        let (update, what, keep) = (token.to_string(), token.to_string(), token.to_string());
        // An Update (or a kept copy) runs: every button waits for it.
        let busy = self.updating.is_some();
        Some(
            div()
                .id("update-banner")
                .role(Role::Note)
                .test_support()
                .aria_label(text.clone())
                .flex()
                .flex_none()
                .items_center()
                .gap(px(10.))
                .h(px(BANNER_HEIGHT))
                .pl(px(16.))
                .pr(px(12.))
                .border_b_1()
                .border_color(color("border-100"))
                .bg(color("surface-raised"))
                .text_size(px(12.))
                .text_color(color("text-secondary"))
                .child(
                    glyph(icons::UPDATES)
                        .size(px(13.))
                        .flex_none()
                        .text_color(color("accent-blue")),
                )
                .child(
                    div()
                        .id("update-banner-text")
                        .test_support()
                        .aria_label(text.clone())
                        .min_w_0()
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_color(color("text-primary"))
                        .child(text),
                )
                .children(local.map(|local| {
                    div()
                        .id("update-banner-local")
                        .test_support()
                        .aria_label(local.clone())
                        .min_w_0()
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_color(color("warning"))
                        .child(local)
                }))
                .child(div().flex_1())
                .when(offer.local_changes > 0, |bar| {
                    bar.child(
                        banner_button(
                            "update-banner-keep",
                            "Keep my changes as a copy",
                            false,
                            busy,
                        )
                        .when(!busy, |b| {
                            b.on_click(cx.listener(move |this, _, window, cx| {
                                this.keep_copy_then_update(&keep, window, cx)
                            }))
                        })
                        .test_support(),
                    )
                })
                .child(
                    banner_button("update-banner-what-changed", "What changed", false, busy)
                        .when(!busy, |b| {
                            b.on_click(cx.listener(move |this, _, window, cx| {
                                this.show_what_changed(&what, window, cx)
                            }))
                        })
                        .test_support(),
                )
                .child(
                    banner_button(
                        "update-banner-update",
                        if busy { UPDATING } else { "Update" },
                        true,
                        busy,
                    )
                    .when(!busy, |b| {
                        b.on_click(cx.listener(move |this, _, window, cx| {
                            this.apply_offer(&update, window, cx)
                        }))
                    })
                    .test_support(),
                )
                .into_any_element(),
        )
    }

    /// `What changed`: the offer's added, changed and removed files.
    pub(super) fn show_what_changed(
        &mut self,
        token: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(offer) = self.update_offers.get(token) else {
            return;
        };
        let title = format!("What changed in version {}", offer.version);
        let mut lines = change_lines(&offer.plan);
        if lines.is_empty() {
            lines.push("No request changed; only the variables.".into());
        }
        let (ws, token) = (cx.entity().downgrade(), token.to_string());
        window.open_dialog(cx, move |dialog, _, _| {
            let (ws, token) = (ws.clone(), token.clone());
            dialog
                .w(px(480.))
                .title(title.clone())
                .child(
                    div()
                        .id("what-changed")
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .max_h(px(360.))
                        .overflow_y_scroll()
                        .text_size(px(12.))
                        .text_color(color("text-secondary"))
                        .children(lines.iter().enumerate().map(|(i, line)| {
                            div()
                                .id(SharedString::from(format!("what-changed-row-{i}")))
                                .test_support()
                                .aria_label(line.clone())
                                .child(line.clone())
                        })),
                )
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("what-changed-close")
                                .ghost()
                                .label("Close")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("what-changed-update")
                                .primary()
                                .label("Update")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    ws.update(cx, |this, cx| this.apply_offer(&token, window, cx))
                                        .ok();
                                }),
                        ),
                )
        });
    }
}

/// The note bar's 28px buttons: ghost, or `primary` for Update; dimmed
/// and inert (no hover, the caller adds no click) while `busy`.
fn banner_button(
    id: &'static str,
    label: &'static str,
    primary: bool,
    busy: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .flex()
        .flex_none()
        .items_center()
        .h(px(28.))
        .px(px(12.))
        .rounded(px(6.))
        .text_size(px(12.))
        .whitespace_nowrap()
        .map(|b| {
            if primary {
                b.bg(color("text-primary")).text_color(color("on-primary"))
            } else {
                b.text_color(color("text-secondary"))
            }
        })
        .map(|b| {
            if busy {
                b.opacity(0.55).cursor_default()
            } else if primary {
                b.cursor_pointer().hover(|s| s.opacity(0.9))
            } else {
                b.cursor_pointer().hover(|s| s.bg(color("surface-100")))
            }
        })
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::{
        UpdateOffer, UpdatePlan, Utc, banner_text, change_lines, local_changes_text, published_ago,
    };

    #[test]
    fn banner_warns_about_local_changes() {
        assert_eq!(
            local_changes_text(2).as_deref(),
            Some("You've changed 2 files. Updating replaces them.")
        );
        assert_eq!(
            local_changes_text(1).as_deref(),
            Some("You've changed 1 file. Updating replaces it.")
        );
        assert_eq!(local_changes_text(0), None);
    }

    #[test]
    fn banner_reads_like_the_spec() {
        let now = Utc::now();
        let plan = UpdatePlan {
            added: vec!["labels/new.yaml".into()],
            changed: vec!["rates/get-rates.yaml".into()],
            removed: vec!["old.yaml".into()],
            ..UpdatePlan::default()
        };
        let offer = UpdateOffer::new(14, now - chrono::Duration::hours(2), plan, Vec::new());
        assert_eq!(
            banner_text(&offer, now),
            "Version 14 available · published 2 hours ago · 3 requests changed"
        );
        assert_eq!(
            change_lines(&offer.plan),
            [
                "Added labels/new.yaml",
                "Changed rates/get-rates.yaml",
                "Removed old.yaml"
            ]
        );
        let one = UpdateOffer {
            requests_changed: 1,
            published_at: now - chrono::Duration::minutes(1),
            ..offer
        };
        assert_eq!(
            banner_text(&one, now),
            "Version 14 available · published 1 minute ago · 1 request changed"
        );
        assert_eq!(published_ago(now, now), "just now");
        assert_eq!(
            published_ago(now - chrono::Duration::hours(1), now),
            "1 hour ago"
        );
        assert_eq!(
            published_ago(now - chrono::Duration::days(3), now),
            "3 days ago"
        );
        assert!(published_ago(now - chrono::Duration::days(30), now).starts_with("on "));
    }
}
