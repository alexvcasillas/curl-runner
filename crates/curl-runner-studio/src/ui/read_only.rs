//! A received (shared-with-me) collection, board ReadOnly: every write
//! refused with `Read only · shared by {org}`, `Check for updates` from the
//! owner, and `Duplicate to my workspace` for an editable copy.
//!
//! The lock itself sits in the model: [`Collection`] refuses to create,
//! rename or delete files and each [`Document`] refuses edits, saves and
//! fixes. The workspace checks first, so the user gets the toast instead of
//! an error.
//!
//! [`Collection`]: crate::model::collection::Collection
//! [`Document`]: crate::model::document::Document

use super::backend_call;
use super::workspace::{Workspace, edit_notes, notify_error};
use crate::model::backend::SyncFile;
use crate::model::collection::read_only_text;
use crate::model::history::relative_time;
use crate::model::shared_copy::{
    STATE_FOLDER_TEXT, UpdatePlan, apply_update, duplicate, holds_studio_state, plan_update,
    received_from,
};
use crate::model::sync_base::SyncBase;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::notification::Notification;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// What a duplicate says once it opens.
pub const DUPLICATED: &str = "Duplicated — enter your own values in Environments";

/// `3 days ago`, lowercase: `updated yesterday`.
pub fn updated_ago(
    at: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> String {
    let text = relative_time(
        at.with_timezone(&chrono::Local),
        now.with_timezone(&chrono::Local),
    );
    let mut c = text.chars();
    c.next()
        .map_or_else(String::new, |f| f.to_lowercase().chain(c).collect())
}

/// `Update 3 files from Acme?`.
pub fn update_title(n: usize, org: &str) -> String {
    let files = if n == 1 { "file" } else { "files" };
    format!("Update {n} {files} from {org}?")
}

/// `2 changed · 1 added · 1 removed`, each left out at zero.
pub fn update_detail(plan: &UpdatePlan) -> String {
    [
        (plan.changed.len(), "changed"),
        (plan.added.len(), "added"),
        (plan.removed.len(), "removed"),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, what)| format!("{n} {what}"))
    .collect::<Vec<_>>()
    .join(" · ")
}

impl Workspace {
    /// The org a received copy came from, while one is open: it is read
    /// only.
    pub(super) fn read_only_org(&self) -> Option<&str> {
        self.state.collection.as_ref()?.read_only.as_deref()
    }

    /// For a received copy, shows `Read only · shared by {org}` and returns
    /// `true`: the caller does nothing.
    pub(super) fn refuse_read_only(&mut self, window: &mut Window, cx: &mut App) -> bool {
        let Some(org) = self.read_only_org().map(str::to_string) else {
            return false;
        };
        self.toast_read_only(&org, window, cx);
        true
    }

    /// For a folder picked to write into (an import target, a new
    /// collection) that is a received copy, shows `Read only · shared by
    /// {org}` (or, for the studio's own folder, [`STATE_FOLDER_TEXT`]) and
    /// returns `true`: nothing is written.
    pub(super) fn refuse_received(
        &mut self,
        dir: &Path,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        match self.refusal_of(dir) {
            Some(text) => {
                self.toast(text, window, cx);
                true
            }
            None => false,
        }
    }

    /// Why a folder picked by hand can't take files: a received copy (or a
    /// folder in one), or the studio's own data folder or one holding it.
    fn refusal_of(&self, dir: &Path) -> Option<String> {
        let base = self.store.base();
        if let Some(org) = received_from(base, dir) {
            Some(read_only_text(&org))
        } else if holds_studio_state(base, dir) {
            Some(STATE_FOLDER_TEXT.to_string())
        } else {
            None
        }
    }

    /// [`refuse_read_only`] or [`refuse_received`] for writing into
    /// `dir`: the refusal's text, toasted, or `None` to go ahead.
    ///
    /// [`refuse_read_only`]: Workspace::refuse_read_only
    /// [`refuse_received`]: Workspace::refuse_received
    pub(super) fn refuse_write_into(
        &mut self,
        dir: &Path,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<String> {
        let text = self
            .read_only_org()
            .map(read_only_text)
            .or_else(|| self.refusal_of(dir))?;
        self.toast(text.clone(), window, cx);
        Some(text)
    }

    fn toast_read_only(&mut self, org: &str, window: &mut Window, cx: &mut App) {
        self.toast(read_only_text(org), window, cx);
    }

    fn toast(&mut self, text: String, window: &mut Window, cx: &mut App) {
        #[cfg(test)]
        self.toasts.push(text.clone());
        window.push_notification(Notification::warning(text), cx);
    }

    /// The open collection as the boards name it: a received copy by its
    /// shared name (`Shipping API`), not its folder's.
    pub(super) fn collection_title(&self) -> Option<String> {
        let c = self.state.collection.as_ref()?;
        Some(match &self.received {
            Some(r) if c.read_only.is_some() => r.name.clone(),
            _ => c.name.clone(),
        })
    }

    /// Where a received copy's exports start: the user's Documents (else
    /// home), never the copy's own folder. `None`: the collection root.
    pub(super) fn export_dir(&self) -> Option<PathBuf> {
        self.read_only_org()?;
        let home = std::env::home_dir()?;
        let documents = home.join("Documents");
        Some(if documents.is_dir() { documents } else { home })
    }

    /// `updated 3 days ago`: when the owner last changed the open copy.
    pub(super) fn received_updated(&self, cx: &App) -> Option<String> {
        let at = self.meta(cx).received_updated?;
        Some(updated_ago(at, chrono::Utc::now()))
    }

    /// `Check for updates`: fetches the owner's copy, counts what differs
    /// and asks `Update 3 files from Acme?` before writing anything.
    pub(super) fn check_for_updates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(c) = self
            .state
            .collection
            .as_ref()
            .filter(|c| c.read_only.is_some())
        else {
            return;
        };
        let Some(share) = self.received.clone() else {
            // A copy with no share record (removed on the server, or a
            // folder under shared/ added by hand) has nowhere to ask.
            let text = "Couldn't check for updates: this copy has no share to ask.".to_string();
            #[cfg(test)]
            self.toasts.push(text.clone());
            window.push_notification(Notification::warning(text), cx);
            return;
        };
        if self.update_check.is_some() {
            return;
        }
        let root = c.root.clone();
        let id = share.id.clone();
        let call = backend_call(cx, &self.backend, move |b| b.fetch_shared(&id));
        self.update_check = Some(cx.spawn_in(window, async move |this, cx| {
            let planned = match call.await {
                Ok(snapshot) => {
                    let at = root.clone();
                    cx.background_spawn(async move { plan_update(&at, snapshot.files) })
                        .await
                }
                Err(e) => Err(format!(
                    "Couldn't check for updates: {}",
                    e.link_text(&share.owner_org)
                )),
            };
            this.update_in(cx, |this, window, cx| {
                this.update_check = None;
                // Another collection opened meanwhile: not this one's.
                let same = this
                    .state
                    .collection
                    .as_ref()
                    .is_some_and(|c| c.root == root);
                if same {
                    this.update_checked(planned, window, cx);
                }
            })
            .ok();
        }));
        cx.notify();
    }

    fn update_checked(
        &mut self,
        planned: Result<UpdatePlan, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(org) = self.read_only_org().map(str::to_string) else {
            return;
        };
        let plan = match planned {
            Ok(plan) => plan,
            Err(e) => return notify_error(e, window, cx),
        };
        if plan.count() == 0 {
            window.push_notification(Notification::info(format!("Up to date with {org}")), cx);
            return;
        }
        let (title, detail) = (update_title(plan.count(), &org), update_detail(&plan));
        #[cfg(test)]
        self.toasts.push(title.clone());
        let ws = cx.entity().downgrade();
        let plan = Rc::new(plan);
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (ws, plan) = (ws.clone(), plan.clone());
            alert
                .title(title.clone())
                .description(format!(
                    "{detail}. Your values in Environments stay as they are."
                ))
                .ok_text("Update")
                .show_cancel(true)
                .on_ok(move |_, window, cx| {
                    ws.update(cx, |this, cx| this.apply_shared_update(&plan, window, cx))
                        .ok();
                    true
                })
        });
    }

    /// Writes the confirmed update: the YAML files only, through the same
    /// path checks as a sync pull; then the sync base, the notes' updated
    /// time and the tree.
    pub(super) fn apply_shared_update(
        &mut self,
        plan: &UpdatePlan,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (Some(c), Some(org)) = (&self.state.collection, self.read_only_org()) else {
            return;
        };
        let (root, org) = (c.root.clone(), org.to_string());
        if let Err(e) = apply_update(&root, plan) {
            notify_error(e, window, cx);
            // Part may be on disk: the tree shows what is there.
            self.disk_changed(&touched(plan), window, cx);
            return;
        }
        edit_notes(&self.notes, cx, |meta| {
            meta.received_updated = Some(chrono::Utc::now());
        });
        // Synced as of now: nothing shows as waiting. The base goes through
        // the sync's saver, so its writes stay in order.
        let base = SyncBase {
            files: plan
                .files
                .iter()
                .map(|f| (f.rel.clone(), SyncFile::hash_of(&f.text)))
                .collect(),
            ..SyncBase::default()
        };
        self.synced_as(base, window, cx);
        self.disk_changed(&touched(plan), window, cx);
        let n = plan.count();
        let files = if n == 1 { "file" } else { "files" };
        window.push_notification(
            Notification::success(format!("Updated {n} {files} from {org}")),
            cx,
        );
    }

    /// `Duplicate to my workspace`: a folder picker, then [`duplicate_to`].
    ///
    /// [`duplicate_to`]: Workspace::duplicate_to
    pub(super) fn duplicate_to_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.read_only_org().is_none() {
            return;
        }
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Duplicate here".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            let Some(dest) = paths.into_iter().next() else {
                return;
            };
            this.update_in(cx, |this, window, cx| this.duplicate_to(&dest, window, cx))
                .ok();
        })
        .detach();
    }

    /// Copies the open received copy's YAML into `dest` (no `.env`, no
    /// Keychain value; secret names and descriptions come along), then
    /// opens `dest` as an ordinary, editable collection.
    pub(super) fn duplicate_to(
        &mut self,
        dest: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(c) = self
            .state
            .collection
            .as_ref()
            .filter(|c| c.read_only.is_some())
        else {
            return;
        };
        let from = c.root.clone();
        let meta = self.meta(cx).clone();
        if let Err(e) = duplicate(self.store.base(), &from, &meta, dest) {
            return notify_error(e, window, cx);
        }
        let dest: PathBuf = dest.to_path_buf();
        if self.open_collection(&dest, window, cx) {
            self.remember_folder(&dest);
            #[cfg(test)]
            self.toasts.push(DUPLICATED.into());
            window.push_notification(Notification::success(DUPLICATED), cx);
        }
    }

    /// Opens another collection from the sidebar (Shared with me, My
    /// collections), once unsaved changes are settled.
    pub(super) fn switch_to(&mut self, root: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.invalidate_secrets();
        self.settle_unsaved(
            Rc::new(move |this, window, cx| {
                if this.open_collection(&root, window, cx) {
                    this.remember_folder(&root);
                }
            }),
            window,
            cx,
        );
    }
}

/// The paths an update wrote or removed.
fn touched(plan: &UpdatePlan) -> Vec<PathBuf> {
    plan.added
        .iter()
        .chain(&plan.changed)
        .chain(&plan.removed)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{update_detail, update_title, updated_ago};
    use crate::model::shared_copy::UpdatePlan;

    #[test]
    fn copy_reads_like_the_board() {
        let now = chrono::Utc::now();
        assert_eq!(
            updated_ago(now - chrono::Duration::days(3), now),
            "3 days ago"
        );
        assert_eq!(update_title(3, "Acme"), "Update 3 files from Acme?");
        assert_eq!(update_title(1, "Acme"), "Update 1 file from Acme?");
        let plan = UpdatePlan {
            changed: vec!["a.yaml".into(), "b.yaml".into()],
            removed: vec!["c.yaml".into()],
            ..UpdatePlan::default()
        };
        assert_eq!(update_detail(&plan), "2 changed · 1 removed");
    }
}

#[cfg(test)]
mod gpui_tests {
    use super::super::app::{Import, NewRequest, QuickFix, RunActive, SaveFile};
    use super::super::palette::PaletteAction;
    use super::super::state::StateStore;
    use super::super::workspace::{Workspace, test_backend, wait_for_run};
    use super::{DUPLICATED, STATE_FOLDER_TEXT};
    use crate::model::backend::{
        Account, Backend, BoxFut, CollectionKey, ConnectionInfo, DevControls, Person,
        ReceivedShare, Server, ShareLink, ShareSettings, ShareUpdate, SharedCollection,
        SharedSnapshot, SignInMethod, SyncDeletion, SyncFile, SyncReport, SyncStatus, sync_files,
    };
    use crate::model::collection_meta::CollectionMeta;
    use crate::model::secrets::{MemorySecrets, SecretStore, service_name};
    use gpui_kit::component::WindowExt as _;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    const LINK: &str = "curl-runner://open/acme/shipping-api?share=7Kx2Qm";
    const TOKEN: &str = "sk-live-customer-4242";
    const RATES: &str = "rates/get-rates.yaml";

    /// The backend, recording every file a sync sends (text included).
    struct Spy {
        inner: Arc<dyn Backend>,
        sent: Mutex<Vec<SyncFile>>,
    }

    impl Backend for Spy {
        fn server(&self) -> Server {
            self.inner.server()
        }
        fn set_server(&self, server: Server) -> BoxFut<'_, ()> {
            self.inner.set_server(server)
        }
        fn test_connection(&self, server: &Server) -> BoxFut<'_, ConnectionInfo> {
            self.inner.test_connection(server)
        }
        fn sign_in(&self, method: SignInMethod) -> BoxFut<'_, Account> {
            self.inner.sign_in(method)
        }
        fn sign_out(&self) -> BoxFut<'_, ()> {
            self.inner.sign_out()
        }
        fn account(&self) -> Option<Account> {
            self.inner.account()
        }
        fn sync_status(&self, collection: &CollectionKey) -> SyncStatus {
            self.inner.sync_status(collection)
        }
        fn sync(
            &self,
            collection: &CollectionKey,
            local: &[SyncFile],
            deleted: &[SyncDeletion],
        ) -> BoxFut<'_, SyncReport> {
            self.sent.lock().unwrap().extend(local.iter().cloned());
            self.inner.sync(collection, local, deleted)
        }
        fn share_settings(&self, collection: &CollectionKey) -> BoxFut<'_, ShareSettings> {
            self.inner.share_settings(collection)
        }
        fn update_share(
            &self,
            collection: &CollectionKey,
            update: ShareUpdate,
        ) -> BoxFut<'_, ShareSettings> {
            self.inner.update_share(collection, update)
        }
        fn invite(&self, collection: &CollectionKey, email: &str) -> BoxFut<'_, Person> {
            self.inner.invite(collection, email)
        }
        fn remove_person(&self, collection: &CollectionKey, email: &str) -> BoxFut<'_, ()> {
            self.inner.remove_person(collection, email)
        }
        fn resolve_link(&self, link: &ShareLink) -> BoxFut<'_, SharedCollection> {
            self.inner.resolve_link(link)
        }
        fn fetch_shared(&self, id: &str) -> BoxFut<'_, SharedSnapshot> {
            self.inner.fetch_shared(id)
        }
        fn received(&self) -> Vec<ReceivedShare> {
            self.inner.received()
        }
        fn add_received(&self, share: ReceivedShare) -> BoxFut<'_, ()> {
            self.inner.add_received(share)
        }
        fn dev(&self) -> Option<&dyn DevControls> {
            self.inner.dev()
        }
    }

    struct Studio {
        state: tempfile::TempDir,
        secrets: Arc<MemorySecrets>,
        spy: Arc<Spy>,
        window: AnyWindowHandle,
        ws: Entity<Workspace>,
    }

    impl Studio {
        fn root(&self) -> PathBuf {
            self.state.path().join("shared/acme-shipping-api")
        }

        fn read(&self, rel: &str) -> String {
            std::fs::read_to_string(self.root().join(rel)).unwrap()
        }
    }

    /// A studio with Acme's Shipping API added from its link (the Add
    /// flow, `BASE_URL` as given) and open: a received, read-only copy.
    fn received(cx: &mut TestAppContext, signed_in: bool, base_url: Option<&str>) -> Studio {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let secrets = Arc::new(MemorySecrets::default());
        let inner = test_backend();
        if signed_in {
            futures::executor::block_on(inner.sign_in(SignInMethod::GitHub)).unwrap();
        }
        let spy = Arc::new(Spy {
            inner,
            sent: Mutex::default(),
        });
        let (s, b) = (
            secrets.clone() as Arc<dyn SecretStore>,
            spy.clone() as Arc<dyn Backend>,
        );
        let (window, ws) = cx.update(|cx| {
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
                cx.new(|cx| Workspace::new(None, store, s, b, window, cx))
            })
            .unwrap()
        });
        cx.run_until_parked();
        let view = cx
            .update_window(window, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.open_shared(None, window, cx))
            })
            .unwrap();
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| v.paste(LINK, window, cx));
            window.render_frame(cx);
            window.click("open-shared-open", cx);
        })
        .unwrap();
        cx.run_until_parked();
        click(cx, window, "open-shared-continue");
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| {
                if let Some(url) = base_url {
                    v.fill("BASE_URL", url, window, cx);
                }
                v.fill("API_TOKEN", TOKEN, window, cx);
            });
        })
        .unwrap();
        click(cx, window, "open-shared-add");
        let s = Studio {
            state,
            secrets,
            spy,
            window,
            ws,
        };
        s.ws.read_with(cx, |ws, _| {
            let c = ws.state.collection.as_ref().expect("the copy is open");
            assert_eq!(c.root, s.root());
            assert_eq!(c.read_only.as_deref(), Some("Acme"));
        });
        s
    }

    fn click(cx: &mut TestAppContext, window: AnyWindowHandle, id: &str) {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.click(id.to_string(), cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    fn open_file(cx: &mut TestAppContext, s: &Studio, rel: &str) {
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.open_file(Path::new(rel), window, cx));
            window.render_frame(cx);
        })
        .unwrap();
    }

    fn dispatch(cx: &mut TestAppContext, s: &Studio, action: &dyn gpui_kit::Action) {
        let action = action.boxed_clone();
        cx.update_window(s.window, |_, window, cx| {
            window.render_frame(cx);
            window.dispatch_action(action, cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    fn toasts(cx: &mut TestAppContext, s: &Studio) -> Vec<String> {
        s.ws.read_with(cx, |ws, _| ws.toasts.clone())
    }

    fn dialog_open(cx: &mut TestAppContext, s: &Studio) -> bool {
        cx.update_window(s.window, |_, window, cx| window.has_active_dialog(cx))
            .unwrap()
    }

    /// Every YAML file of the copy, as on disk.
    fn snapshot(s: &Studio) -> Vec<SyncFile> {
        sync_files(&s.root())
    }

    #[gpui_kit::test]
    fn read_only_blocks_every_write_path(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        // A file with a quick fix, as the owner could have shared it.
        let typo = "request:\n  url: https://api.example.com\n  timout: 5000\n";
        std::fs::write(s.root().join("rates/typo.yaml"), typo).unwrap();
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.disk_changed(&["rates/typo.yaml".into()], window, cx)
            });
        })
        .unwrap();
        let before = snapshot(&s);
        let refused = "Read only · shared by Acme";
        let mut expected = 0;
        let mut refused_once = |cx: &mut TestAppContext, what: &str| {
            expected += 1;
            let told = toasts(cx, &s);
            assert_eq!(told.len(), expected, "{what}: {told:?}");
            assert_eq!(told.last().map(String::as_str), Some(refused), "{what}");
            assert!(!dialog_open(cx, &s), "{what} opened a dialog");
        };

        // Typing into the editor.
        open_file(cx, &s, RATES);
        let rates = s.read(RATES);
        cx.update_window(s.window, |_, window, cx| {
            window.render_frame(cx);
            window.input("# mine\n", cx);
        })
        .unwrap();
        cx.run_until_parked();
        s.ws.read_with(cx, |ws, cx| {
            let doc = &ws.state.open[0];
            assert_eq!(doc.text, rates);
            assert!(!doc.is_dirty());
            assert_eq!(ws.tabs[0].editor.read(cx).value().as_ref(), rates);
        });

        // ⌘S.
        dispatch(cx, &s, &SaveFile);
        refused_once(cx, "⌘S");

        // Rename and delete, as the tree's menu would ask.
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.prompt_rename(RATES.into(), window, cx));
        })
        .unwrap();
        refused_once(cx, "rename");
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.prompt_delete("rates".into(), window, cx));
        })
        .unwrap();
        refused_once(cx, "delete");

        // Quick fix (⌘.), and the editor offers no code action.
        open_file(cx, &s, "rates/typo.yaml");
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                let ix = ws.state.active.unwrap();
                ws.tabs[ix].editor.update(cx, |e, cx| {
                    e.set_cursor_position(
                        gpui_kit::component::input::Position::new(2, 4),
                        window,
                        cx,
                    )
                });
                let editor = ws.tabs[ix].editor.clone();
                let at = typo.find("timout").unwrap() + 2;
                assert!(ws.code_actions(&editor, at..at, cx).is_empty());
            });
        })
        .unwrap();
        dispatch(cx, &s, &QuickFix);
        refused_once(cx, "quick fix");
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                let ix = ws.state.active.unwrap();
                let fix = ws.state.open[ix].diagnostics[0].fix.clone().unwrap();
                let path = ws.state.open[ix].diagnostics[0].path.clone();
                ws.apply_named_fix(ix, &path, &fix, window, cx);
            });
        })
        .unwrap();
        refused_once(cx, "the popover's fix");

        // New request: ⌘N and the palette (which doesn't list it).
        dispatch(cx, &s, &NewRequest);
        refused_once(cx, "⌘N");
        let palette = cx
            .update_window(s.window, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.open_palette(window, cx))
            })
            .unwrap();
        palette.read_with(cx, |p, _| {
            assert!(!p.lists(PaletteAction::NewRequest));
            assert!(!p.lists(PaletteAction::Import));
            assert!(p.lists(PaletteAction::Environments));
        });
        cx.update_window(s.window, |_, window, cx| window.close_dialog(cx))
            .unwrap();
        cx.run_until_parked();

        // Import (⌘I).
        dispatch(cx, &s, &Import);
        refused_once(cx, "import");

        // An edit on disk reloads (the owner's copy wins); Keep mine is
        // refused and changes nothing.
        let theirs = rates.replace("Get rates", "Get quotes");
        std::fs::write(s.root().join(RATES), &theirs).unwrap();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.disk_changed(&[RATES.into()], window, cx);
                let ix = ws.state.position(Path::new(RATES)).unwrap();
                ws.activate_tab(ix, window, cx);
                assert!(!ws.state.open[ix].disk_conflict);
                assert_eq!(ws.state.open[ix].text, theirs);
                ws.resolve_conflict(true, window, cx);
            });
        })
        .unwrap();
        refused_once(cx, "keep mine");

        // The model refuses even when asked directly.
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                let ix = ws.state.active.unwrap();
                ws.state.open[ix].set_text("mine\n".into());
                assert!(!ws.save(ix, window, cx));
                let c = ws.state.collection.as_mut().unwrap();
                assert!(c.create_file(Path::new("x.yaml"), "x").is_err());
                assert!(c.delete(Path::new("rates")).is_err());
            });
        })
        .unwrap();
        assert_eq!(s.read(RATES), theirs, "nothing was saved");
        let after = snapshot(&s);
        let changed: Vec<_> = after
            .iter()
            .filter(|f| !before.contains(f))
            .map(|f| f.rel.clone())
            .collect();
        assert_eq!(changed, [PathBuf::from(RATES)], "only the disk edit");
        assert_eq!(after.len(), before.len());
        // Typing said nothing (the editor took no change); the direct
        // save above was the ninth refusal.
        assert_eq!(toasts(cx, &s).len(), 9);
    }

    #[gpui_kit::test]
    fn read_only_view_reads_like_the_board(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        open_file(cx, &s, RATES);
        let label = |cx: &mut TestAppContext, id: &str| {
            cx.update_window(s.window, |_, window, cx| {
                window.render_frame(cx);
                window
                    .try_find(id.to_string())
                    .and_then(|e| e.label().map(str::to_string))
            })
            .unwrap()
        };
        assert_eq!(
            label(cx, "read-only-note").as_deref(),
            Some("Read only · shared by Acme · updated 3 days ago")
        );
        assert_eq!(
            label(cx, "title-read-only").as_deref(),
            Some("Read only · Acme")
        );
        assert_eq!(label(cx, "sidebar-title").as_deref(), Some("Shipping API"));
        assert_eq!(
            label(cx, "sidebar-updated").as_deref(),
            Some("Updated 3 days ago")
        );
        assert_eq!(
            label(cx, "status-language").as_deref(),
            Some("YAML · read only")
        );
        assert!(label(cx, "sidebar-new-request").is_none());
        // Back in a collection of the user's own: Shared with me lists it.
        let mine = tempfile::tempdir().unwrap();
        std::fs::write(mine.path().join("a.yaml"), "request:\n  url: https://a\n").unwrap();
        let ws = s.ws.clone();
        let path = mine.path().to_path_buf();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                assert!(ws.open_collection(&path, window, cx));
                ws.remember_folder(&path);
            });
        })
        .unwrap();
        assert_eq!(
            label(cx, "shared-with-me").as_deref(),
            Some("Shared with me")
        );
        assert_eq!(
            label(cx, "other-collection-0").as_deref(),
            Some("Shipping API")
        );
        assert!(label(cx, "read-only-note").is_none());
        click(cx, s.window, "other-collection-0");
        s.ws.read_with(cx, |ws, _| {
            assert_eq!(ws.state.collection.as_ref().unwrap().root, s.root());
        });
        // And from the copy, My collections lists the user's own.
        assert_eq!(
            label(cx, "my-collections").as_deref(),
            Some("My collections")
        );
        let name = mine
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(label(cx, "other-collection-0"), Some(name));
    }

    #[gpui_kit::test]
    fn run_works_in_read_only(cx: &mut TestAppContext) {
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let log = seen.clone();
        let srv = testkit::TestServer::with_handler(move |req| {
            let auth = req
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            log.lock().unwrap().push(auth);
            testkit::Response::from_data(br#"{"rates":[{"carrier":"ground"}]}"#.to_vec())
                .with_header("Content-Type", "application/json")
        });
        let s = received(cx, false, Some(&srv.base_url));
        open_file(cx, &s, RATES);
        let rates = s.read(RATES);
        dispatch(cx, &s, &RunActive);
        wait_for_run(cx, &s.ws);
        // The customer's own token, from their keychain.
        assert_eq!(*seen.lock().unwrap(), [format!("Bearer {TOKEN}")]);
        s.ws.read_with(cx, |ws, _| {
            let run = &ws.runs.last[Path::new(RATES)];
            // The fixture passes core's checks (status, body.rates.length).
            assert!(run.results[&0].success, "{:?}", run.results[&0].error);
            assert_eq!(run.keychain_names, ["API_TOKEN"]);
        });
        // The board's summary line, as drawn.
        let summary = cx
            .update_window(s.window, |_, window, cx| {
                window.render_frame(cx);
                window
                    .try_find("inspector-summary")
                    .and_then(|e| e.label().map(str::to_string))
            })
            .unwrap();
        assert_eq!(
            summary.as_deref(),
            Some("2 of 2 checks passed · ran with your API_TOKEN")
        );
        // The run saved nothing and asked nothing.
        assert_eq!(s.read(RATES), rates);
        assert!(toasts(cx, &s).is_empty(), "{:?}", toasts(cx, &s));
    }

    #[gpui_kit::test]
    fn check_for_updates_replaces_files(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let owners = snapshot(&s);
        let env = std::fs::read_to_string(s.root().join(".env")).unwrap();
        // Out of date: two files changed, one gone, one extra.
        std::fs::write(s.root().join(RATES), "request:\n  url: https://old\n").unwrap();
        std::fs::write(
            s.root().join("rates/list-carriers.yaml"),
            "request:\n  url: https://old\n",
        )
        .unwrap();
        std::fs::remove_file(s.root().join("rates/validate-address.yaml")).unwrap();
        std::fs::write(
            s.root().join("rates/old.yaml"),
            "request:\n  url: https://x\n",
        )
        .unwrap();
        let ws = s.ws.clone();
        open_file(cx, &s, RATES);
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.check_for_updates(window, cx));
        })
        .unwrap();
        cx.run_until_parked();
        assert!(dialog_open(cx, &s));
        assert_eq!(
            toasts(cx, &s).last().map(String::as_str),
            Some("Update 4 files from Acme?")
        );
        // Nothing is written before the confirm.
        assert!(s.root().join("rates/old.yaml").exists());
        click(cx, s.window, "ok");
        assert!(!dialog_open(cx, &s));
        assert_eq!(snapshot(&s), owners, "the owner's files, exactly");
        assert!(!s.root().join("rates/old.yaml").exists());
        assert_eq!(std::fs::read_to_string(s.root().join(".env")).unwrap(), env);
        let id = s.ws.read_with(cx, |ws, cx| ws.meta(cx).id.clone());
        let service = service_name("acme-shipping-api", "Default", &id);
        assert_eq!(s.secrets.get(&service, "API_TOKEN").as_deref(), Some(TOKEN));
        cx.run_until_parked();
        s.ws.read_with(cx, |ws, cx| {
            // The open tab took the new text; the tree has the old file gone.
            let ix = ws.state.position(Path::new(RATES)).unwrap();
            let theirs = owners.iter().find(|f| f.rel == Path::new(RATES)).unwrap();
            assert_eq!(ws.state.open[ix].text, theirs.text);
            assert!(ws.method_of(Path::new("rates/old.yaml")).is_none());
            let at = ws.meta(cx).received_updated.unwrap();
            assert!(chrono::Utc::now() - at < chrono::Duration::minutes(1));
            assert_eq!(ws.sync.pending, 0);
        });
        let base = crate::model::sync_base::load(Some(s.state.path()), &id);
        assert_eq!(base.files.len(), owners.len());
        // Again: up to date, nothing asked.
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.check_for_updates(window, cx));
        })
        .unwrap();
        cx.run_until_parked();
        assert!(!dialog_open(cx, &s));
    }

    #[gpui_kit::test]
    fn duplicate_copies_yaml_not_env_and_is_editable(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let dest = tempfile::tempdir().unwrap();
        let from = s.ws.read_with(cx, |ws, cx| ws.meta(cx).clone());
        let ws = s.ws.clone();
        // Not into the shared copy itself.
        let inside = s.root().join("rates");
        let before = snapshot(&s);
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.duplicate_to(&inside, window, cx));
        })
        .unwrap();
        assert!(s.ws.read_with(cx, |ws, _| ws.read_only_org().is_some()));
        assert_eq!(snapshot(&s), before, "nothing copied into itself");
        let path = dest.path().to_path_buf();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.duplicate_to(&path, window, cx));
        })
        .unwrap();
        cx.run_until_parked();
        let copied: Vec<_> = sync_files(dest.path()).into_iter().map(|f| f.rel).collect();
        let theirs: Vec<_> = snapshot(&s).into_iter().map(|f| f.rel).collect();
        assert_eq!(copied, theirs);
        assert!(!dest.path().join(".env").exists(), "no .env");
        assert_eq!(toasts(cx, &s).last().map(String::as_str), Some(DUPLICATED));
        let notes = CollectionMeta::load(s.state.path(), dest.path());
        assert_eq!(notes.read_only_from, None);
        assert!(notes.received_vars.is_empty());
        assert_ne!(notes.id, from.id, "a new id");
        assert_eq!(notes.secret_names, from.secret_names);
        assert_eq!(
            notes.vars["API_TOKEN"].description,
            from.vars["API_TOKEN"].description
        );
        // No Keychain value under the copy's service.
        let service = service_name(
            &dest.path().file_name().unwrap().to_string_lossy(),
            "Default",
            &notes.id,
        );
        assert_eq!(s.secrets.get(&service, "API_TOKEN"), None);
        // Open, and editable: a save writes.
        open_file(cx, &s, RATES);
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                let c = ws.state.collection.as_ref().unwrap();
                assert_eq!(c.root, dest.path());
                assert_eq!(c.read_only, None);
                let ix = ws.state.active.unwrap();
                assert!(!ws.state.open[ix].is_read_only());
                ws.state.open[ix].set_text("request:\n  url: https://mine\n".into());
                assert!(ws.save(ix, window, cx));
            });
        })
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(dest.path().join(RATES)).unwrap(),
            "request:\n  url: https://mine\n"
        );
        assert_ne!(s.read(RATES), "request:\n  url: https://mine\n");
    }

    #[gpui_kit::test]
    fn a_received_copy_resolves_only_by_taking_theirs(cx: &mut TestAppContext) {
        let s = received(cx, true, None);
        let owners = s.read(RATES);
        let mine = "request:\n  url: https://mine\n";
        std::fs::write(s.root().join(RATES), mine).unwrap();
        let conflict = crate::model::backend::Conflict {
            rel: RATES.into(),
            local: mine.into(),
            server: owners.clone(),
            server_author: "Acme".into(),
            server_at: chrono::Utc::now(),
            local_deleted: false,
            server_deleted: false,
        };
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.sync.conflicts = vec![conflict];
                ws.open_conflicts(window, cx);
            });
            window.render_frame(cx);
            assert!(window.try_find("conflict-take-theirs").is_some());
            assert!(
                window.try_find("conflict-keep-mine").is_none(),
                "the owner wins"
            );
            assert!(window.try_find("conflict-keep-both").is_none());
        })
        .unwrap();
        // Nor by hand.
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.resolve_shown(crate::model::conflict::Choice::KeepMine, false, window, cx)
            })
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(s.read(RATES), mine);
        assert_eq!(s.ws.read_with(cx, |ws, _| ws.sync.conflicts.len()), 1);
        s.spy.sent.lock().unwrap().clear();
        click(cx, s.window, "conflict-take-theirs");
        assert_eq!(s.read(RATES), owners);
        assert!(s.ws.read_with(cx, |ws, _| ws.sync.conflicts.is_empty()));
        let sent = s.spy.sent.lock().unwrap().clone();
        assert!(
            sent.iter().all(|f| f.text.is_empty()),
            "no text left the studio"
        );
    }

    #[gpui_kit::test]
    fn a_received_copy_never_pushes(cx: &mut TestAppContext) {
        let s = received(cx, true, None);
        let owners = s.read(RATES);
        // Changed on disk behind the studio's back, then synced.
        std::fs::write(s.root().join(RATES), "request:\n  url: https://mine\n").unwrap();
        std::fs::write(
            s.root().join("rates/mine.yaml"),
            "request:\n  url: https://x\n",
        )
        .unwrap();
        s.spy.sent.lock().unwrap().clear();
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.sync_now(window, cx));
        })
        .unwrap();
        cx.run_until_parked();
        let sent = s.spy.sent.lock().unwrap().clone();
        assert!(!sent.is_empty(), "the sync ran");
        assert!(
            sent.iter().all(|f| f.text.is_empty()),
            "no text left the studio: {:?}",
            sent.iter().map(|f| &f.rel).collect::<Vec<_>>()
        );
        // The owner's copy wins on pull.
        assert_eq!(s.read(RATES), owners);
        assert!(
            !s.spy
                .dev()
                .unwrap()
                .recorded()
                .iter()
                .any(|c| c.starts_with("sync(") && c.contains("SyncDeletion")),
            "no deletions"
        );
    }

    /// A second window on the same studio state with nothing open.
    fn nothing_open(cx: &mut TestAppContext, s: &Studio) -> (AnyWindowHandle, Entity<Workspace>) {
        let store = StateStore::at(s.state.path());
        let (secrets, backend) = (
            s.secrets.clone() as Arc<dyn SecretStore>,
            s.spy.clone() as Arc<dyn Backend>,
        );
        let out = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| Workspace::new(None, store, secrets, backend, window, cx))
            })
            .unwrap()
        });
        cx.run_until_parked();
        out
    }

    #[gpui_kit::test]
    fn a_received_copy_picked_by_hand_takes_nothing(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let before = snapshot(&s);
        let (window, ws) = nothing_open(cx, &s);
        let refused = "Read only · shared by Acme";
        let told = |cx: &mut TestAppContext| ws.read_with(cx, |ws, _| ws.toasts.clone());
        // Import: the copy can't be picked as the target…
        let dialog = cx
            .update_window(window, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.import(window, cx).unwrap())
            })
            .unwrap();
        let root = s.root();
        cx.update_window(window, |_, window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill("curl https://api.example.com/v1/x", None, window, cx);
                d.set_target(Some(root.join("rates")), window, cx);
                assert_eq!(d.picked_target, None);
            });
        })
        .unwrap();
        assert_eq!(told(cx), [refused]);
        // …and should it get there anyway, nothing is written.
        cx.update_window(window, |_, window, cx| {
            dialog.update(cx, |d, cx| {
                d.picked_target = Some(root.clone());
                d.import(window, cx);
                assert_eq!(d.error.as_deref(), Some(refused));
            });
        })
        .unwrap();
        assert_eq!(told(cx), [refused, refused]);
        cx.update_window(window, |_, window, cx| window.close_dialog(cx))
            .unwrap();
        // New request or collection with no collection open.
        let text = "request:\n  url: https://api.example.com\n".to_string();
        for dir in [s.root(), s.root().join("new-folder")] {
            let text = text.clone();
            cx.update_window(window, |_, window, cx| {
                ws.update(cx, |ws, cx| {
                    ws.create_collection(dir, "first.yaml".into(), text, window, cx)
                });
            })
            .unwrap();
            cx.run_until_parked();
        }
        assert_eq!(told(cx).len(), 4);
        assert!(told(cx).iter().all(|t| t == refused));
        ws.read_with(cx, |ws, _| assert!(ws.state.collection.is_none()));
        // The model refuses on its own too.
        let base = Some(s.state.path());
        let mut c = crate::model::collection::Collection::open_in(&s.root(), base).unwrap();
        assert!(c.create_file(Path::new("x.yaml"), "x").is_err());
        assert_eq!(snapshot(&s), before, "nothing was written");
        assert!(!s.root().join("new-folder").exists());
        // An ordinary folder still takes a new collection.
        let mine = tempfile::tempdir().unwrap();
        let dir = mine.path().to_path_buf();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.create_collection(dir, "first.yaml".into(), text, window, cx)
            });
        })
        .unwrap();
        assert!(mine.path().join("first.yaml").exists());
    }

    #[gpui_kit::test]
    fn the_studio_folder_is_never_a_target(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let (window, ws) = nothing_open(cx, &s);
        let base = s.state.path().to_path_buf();
        let told = |cx: &mut TestAppContext| ws.read_with(cx, |ws, _| ws.toasts.clone());
        let dialog = cx
            .update_window(window, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.import(window, cx).unwrap())
            })
            .unwrap();
        let mine = tempfile::tempdir().unwrap();
        let (ok, state) = (mine.path().to_path_buf(), base.clone());
        cx.update_window(window, |_, window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill("curl https://api.example.com/v1/x", None, window, cx);
                d.set_target(Some(ok.clone()), window, cx);
                assert_eq!(d.picked_target.as_deref(), Some(ok.as_path()));
                // Refused: the earlier pick goes too.
                d.set_target(Some(state.clone()), window, cx);
                assert_eq!(d.picked_target, None);
                d.picked_target = Some(state);
                d.import(window, cx);
                assert_eq!(d.error.as_deref(), Some(STATE_FOLDER_TEXT));
            });
        })
        .unwrap();
        cx.update_window(window, |_, window, cx| window.close_dialog(cx))
            .unwrap();
        let text = "request:\n  url: https://api.example.com\n".to_string();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.create_collection(base.clone(), "first.yaml".into(), text, window, cx)
            });
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(told(cx), [STATE_FOLDER_TEXT; 3]);
        assert!(!base.join("first.yaml").exists());
        ws.read_with(cx, |ws, _| assert!(ws.state.collection.is_none()));
    }

    #[gpui_kit::test]
    fn duplicate_never_lands_in_shared_folders(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let ws = s.ws.clone();
        let other = s.state.path().join("shared/someone-else");
        std::fs::create_dir_all(&other).unwrap();
        // A folder holding the copy, the shared folder itself, one beside it.
        for dest in [
            s.state.path().to_path_buf(),
            s.state.path().join("shared"),
            other.clone(),
        ] {
            cx.update_window(s.window, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.duplicate_to(&dest, window, cx));
            })
            .unwrap();
            s.ws.read_with(cx, |ws, _| {
                assert_eq!(
                    ws.state.collection.as_ref().unwrap().root,
                    s.root(),
                    "{dest:?}"
                );
            });
        }
        assert!(std::fs::read_dir(&other).unwrap().next().is_none());
        assert!(!s.state.path().join("rates").exists());
        assert!(!s.state.path().join("shared/rates").exists());
        assert!(!toasts(cx, &s).iter().any(|t| t == DUPLICATED));
    }

    #[gpui_kit::test]
    fn check_for_updates_without_a_share_says_so(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        // A folder under shared/ with no share record, opened by hand.
        let orphan = s.state.path().join("shared/orphan");
        std::fs::create_dir_all(&orphan).unwrap();
        std::fs::write(orphan.join("a.yaml"), "request:\n  url: https://a\n").unwrap();
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                assert!(ws.open_collection(&orphan, window, cx));
                assert_eq!(ws.read_only_org(), Some("its owner"));
                ws.check_for_updates(window, cx);
            });
        })
        .unwrap();
        cx.run_until_parked();
        assert!(!dialog_open(cx, &s));
        assert_eq!(
            toasts(cx, &s).last().map(String::as_str),
            Some("Couldn't check for updates: this copy has no share to ask.")
        );
    }

    #[gpui_kit::test]
    fn exports_from_a_copy_start_outside_it(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let dir = s.ws.read_with(cx, |ws, _| ws.export_dir()).unwrap();
        assert!(!dir.starts_with(s.state.path()), "{dir:?}");
        let mine = tempfile::tempdir().unwrap();
        let path = mine.path().to_path_buf();
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                assert!(ws.open_collection(&path, window, cx));
                assert_eq!(
                    ws.export_dir(),
                    None,
                    "an own collection exports beside itself"
                );
            });
        })
        .unwrap();
    }
}
