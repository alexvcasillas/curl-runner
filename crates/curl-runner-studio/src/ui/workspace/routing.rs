//! Which screen the main area shows: the collection run screen, History,
//! the collection settings page, the app's Settings and the sign-in
//! screen, each opened and closed here.

use super::*;

impl Workspace {
    // The collection run screen.

    /// Whether the run screen takes the main area.
    pub(in crate::ui) fn run_shown(&self) -> bool {
        self.show_run && self.run_view.is_some()
    }

    /// Opens a fresh run screen on the collection run that just started.
    pub(in crate::ui) fn open_run_view(
        &mut self,
        target: RunTarget,
        ctx: EnvCtx,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let labels = self.run_labels();
        // Display only: the Keychain service keeps `ctx.collection`.
        let name = self
            .collection_title()
            .unwrap_or_else(|| ctx.collection.clone());
        let export_dir = self.export_dir();
        let view = cx.new(|cx| {
            let mut view = RunView::new(name, ctx, target, labels, cx);
            view.set_export_dir(export_dir);
            view
        });
        self._run_view_events = Some(cx.subscribe_in(&view, window, Self::on_run_view_event));
        self.run_view = Some(view);
        self.run_view_live = true;
        self.show_run_view(window, cx);
    }

    /// Each file's requests as the tree lists them: `(method, name)`.
    pub(super) fn run_labels(&self) -> Labels {
        fn walk(nodes: &[TreeNode], out: &mut Labels) {
            for n in nodes {
                match n {
                    TreeNode::Folder { children, .. } => walk(children, out),
                    TreeNode::File {
                        name,
                        rel_path,
                        method,
                        requests,
                        ..
                    } => {
                        let rows = if requests.is_empty() {
                            vec![(
                                method.clone().unwrap_or_else(|| "GET".into()),
                                request_label(name).to_string(),
                            )]
                        } else {
                            requests
                                .iter()
                                .map(|r| (r.method.clone(), r.name.clone()))
                                .collect()
                        };
                        out.insert(rel_path.clone(), rows);
                    }
                }
            }
        }
        let mut out = Labels::new();
        if let Some(c) = &self.state.collection {
            walk(&c.tree, &mut out);
        }
        out
    }

    pub(in crate::ui) fn show_run_view(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(view) = self.run_view.clone() else {
            return;
        };
        self.show_run = true;
        self.show_history = false;
        self.show_conflicts = false;
        self.settings_view = None;
        self.close_app_settings();
        self.close_hover(cx);
        view.read(cx).focus_handle(cx).focus(window, cx);
        self.sync_chrome(window, cx);
    }

    /// Back to the editor; the run goes on.
    pub(in crate::ui) fn hide_run_view(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_run = false;
        if self.state.active.is_some() {
            self.focus_active_editor(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
        self.sync_chrome(window, cx);
    }

    pub(super) fn on_run_view_event(
        &mut self,
        _: &Entity<RunView>,
        event: &RunViewEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            RunViewEvent::Open { file, index } => {
                self.open_run_result(file.clone(), *index, window, cx)
            }
            RunViewEvent::Stop => self.stop_run(),
            RunViewEvent::RerunFailed(files) => {
                self.run(RunTarget::Files(files.clone()), window, cx)
            }
            RunViewEvent::Close => self.hide_run_view(window, cx),
        }
    }

    /// Stops the run in flight, if any (Stop, ⌘.).
    pub(in crate::ui) fn stop_run(&mut self) {
        if let Some(handle) = &self.runs.active {
            handle.stop();
        }
    }

    /// A run-table row was clicked: its file, with the cursor (and so the
    /// inspector) on the request; a failed result opens on its checks.
    pub(super) fn open_run_result(
        &mut self,
        file: PathBuf,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_run = false;
        self.open_file(&file, window, cx);
        let Some(ix) = self.state.position(&file) else {
            return;
        };
        if let Some(line) = self.tabs[ix].request_lines.get(index).copied().flatten() {
            self.tabs[ix].editor.update(cx, |e, cx| {
                e.set_cursor_position(
                    gpui_kit::component::input::Position::new(line, 0),
                    window,
                    cx,
                );
            });
        }
        self.tabs[ix].cursor_request = index;
        if self.tabs[ix].request_lines.len() > 1 {
            self.expanded.insert(file.clone());
            self.selected_request = Some((file.clone(), index));
        }
        let view = self.runs.last.get(&file).and_then(|r| r.views.get(&index));
        self.inspector_tab = match view {
            Some(v) if !v.ok || v.failed() > 0 => InspectorTab::Checks,
            _ => InspectorTab::Response,
        };
        self.sync_chrome(window, cx);
    }

    // The History screen.

    pub(in crate::ui) fn history_shown(&self) -> bool {
        self.show_history && self.history_view.is_some()
    }

    /// Shows the History screen (⌘Y), reading the stored runs afresh.
    pub(in crate::ui) fn open_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(c) = &self.state.collection else {
            return;
        };
        let view = match &self.history_view {
            Some(v) => v.clone(),
            None => {
                let (root, name) = (c.root.clone(), c.name.clone());
                let meta = self.notes.read(cx);
                let (id, marked) = (meta.id.clone(), meta.secret_names.clone());
                let shared = meta.shared_defaults();
                let export_dir = self.export_dir();
                let view = cx.new(|cx| {
                    let mut view = HistoryView::new(root, name, id, marked, cx);
                    view.set_export_dir(export_dir);
                    view.set_shared(shared);
                    view
                });
                self._history_events = Some(cx.subscribe_in(&view, window, Self::on_history_event));
                self.history_view = Some(view.clone());
                view
            }
        };
        self.show_history = true;
        self.show_run = false;
        self.show_conflicts = false;
        self.settings_view = None;
        self.close_app_settings();
        self.close_hover(cx);
        view.read(cx).focus_handle(cx).focus(window, cx);
        self.reload_history(cx);
        self.sync_chrome(window, cx);
    }

    /// ⌘Y again leaves it.
    pub(super) fn toggle_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.history_shown() {
            self.hide_history(window, cx);
        } else {
            self.open_history(window, cx);
        }
    }

    pub(in crate::ui) fn hide_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_history = false;
        if self.state.active.is_some() {
            self.focus_active_editor(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
        self.sync_chrome(window, cx);
    }

    /// Reads the stored runs off the main thread into the shown screen.
    pub(super) fn reload_history(&mut self, cx: &mut Context<Self>) {
        let (Some(view), Some(base), Some(c)) = (
            self.history_view.clone().filter(|_| self.show_history),
            self.store.base(),
            &self.state.collection,
        ) else {
            return;
        };
        let history = History::open(base, &c.root);
        let read = cx.background_spawn(async move { (history.list(), history.usage()) });
        cx.spawn(async move |this, cx| {
            let (entries, usage) = read.await;
            view.update(cx, |v, cx| v.set_entries(entries, usage, cx));
            this.update(cx, |this, cx| {
                let usage = this.history_shown().then(|| usage_text(usage).into());
                this.status.update(cx, |s, cx| s.set_history(usage, cx));
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn on_history_event(
        &mut self,
        _: &Entity<HistoryView>,
        event: &HistoryViewEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            HistoryViewEvent::Close => self.hide_history(window, cx),
            HistoryViewEvent::Rerun(e) => self.rerun_history(e, window, cx),
        }
    }

    /// Runs a stored run's target again, in the environment it ran in:
    /// the picker switches to it (a normal pick), and a gone environment
    /// gets a notice instead of a run. A request is found by its file and
    /// name, else by its stored position; a file whose requests can't be
    /// told apart runs whole. Only a gone file (or every file of a re-run)
    /// gets a notice; a re-run with some files gone says how many it skips.
    pub(in crate::ui) fn rerun_history(
        &mut self,
        e: &HistoryEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.runs.active.is_some() {
            window.push_notification(
                Notification::warning("A run is in flight. Stop it first."),
                cx,
            );
            return;
        }
        let Some(c) = &mut self.state.collection else {
            return;
        };
        c.refresh();
        let root = c.root.clone();
        let resolved = rerun_target(e, &c.tree);
        if list_environments(&root) != self.environments {
            self.load_environments(window, cx);
        }
        if !self.environments.contains(&e.environment) {
            window.push_notification(
                Notification::warning(format!(
                    "Environment \"{}\" no longer exists",
                    env_label(&e.environment)
                )),
                cx,
            );
            return;
        }
        let Some((target, skipped)) = resolved else {
            let (title, _) = super::super::history_view::entry_title(e);
            window.push_notification(
                Notification::warning(format!("{title} is no longer in the collection.")),
                cx,
            );
            return;
        };
        self.pick_environment(&e.environment.clone(), window, cx);
        self.show_history = false;
        if let RunTarget::File(f) | RunTarget::Request { file: f, .. } = &target {
            self.open_file(f, window, cx);
        }
        self.run(target, window, cx);
        if skipped > 0 {
            let files = if skipped == 1 { "file" } else { "files" };
            window.push_notification(
                Notification::warning(format!(
                    "Skipped {skipped} {files} no longer in the collection."
                )),
                cx,
            );
        }
        self.sync_chrome(window, cx);
    }

    // The collection settings page.

    pub(in crate::ui) fn settings_shown(&self) -> bool {
        self.settings_view.is_some()
    }

    /// Opens the collection settings on Environments, on the picked
    /// environment, read afresh.
    pub(in crate::ui) fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(c) = &self.state.collection else {
            return;
        };
        let (root, name, notes) = (c.root.clone(), c.name.clone(), self.notes.clone());
        let (secrets, env, captured) = (
            self.secret_cache.clone(),
            self.environment.clone(),
            self.captured_vars(),
        );
        let sharing = self.share_sync(window, cx).map(|sync| Sharing {
            backend: self.backend.clone(),
            sync,
            people: self.sync.people,
        });
        let view = cx.new(|cx| {
            let mut view = CollectionSettings::new(root, name, notes, secrets, env, captured, cx);
            if let Some(sharing) = sharing {
                view.set_sharing(sharing, cx);
            }
            view
        });
        self._settings_events = Some(cx.subscribe_in(&view, window, Self::on_settings_event));
        view.read(cx).focus_handle(cx).focus(window, cx);
        self.settings_view = Some(view);
        self.settings_origin = None;
        self.close_app_settings();
        self.show_run = false;
        self.show_history = false;
        self.close_hover(cx);
        self.sync_chrome(window, cx);
    }

    pub(in crate::ui) fn hide_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_view = None;
        self._settings_events = None;
        if self.state.active.is_some() {
            self.focus_active_editor(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
        self.sync_chrome(window, cx);
    }

    pub(super) fn on_settings_event(
        &mut self,
        _: &Entity<CollectionSettings>,
        event: &SettingsEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            SettingsEvent::Close => match self.settings_origin.take() {
                Some(page) => self.open_app_settings(page, window, cx),
                None => self.hide_settings(window, cx),
            },
            SettingsEvent::Status => self.sync_chrome(window, cx),
            SettingsEvent::OpenApp(page) => self.open_app_settings(*page, window, cx),
            // The notes are shared: what the page wrote there is saved and
            // passed on by `notes_changed`.
            SettingsEvent::Changed => {
                self.invalidate_secrets();
                // A Default value may be a shared default: it goes again.
                self.variables_changed(window, cx);
                if let Some(c) = &self.state.collection
                    && list_environments(&c.root) != self.environments
                {
                    self.load_environments(window, cx);
                }
                self.sync_chrome(window, cx);
            }
        }
    }

    // The app's Settings.

    pub(in crate::ui) fn app_settings_shown(&self) -> bool {
        self.app_settings.is_some()
    }

    pub(in crate::ui) fn close_app_settings(&mut self) {
        self.app_settings = None;
        self._app_settings_events = None;
    }

    /// Opens the app's Settings on `page` (⌘,), in place of whatever the
    /// window shows.
    pub(in crate::ui) fn open_app_settings(
        &mut self,
        page: AppPage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(view) = &self.app_settings {
            view.update(cx, |v, cx| v.set_page(page, cx));
            return;
        }
        let meta = self.notes.read(cx);
        let collection = self.state.collection.as_ref().map(|c| CollectionInfo {
            root: c.root.clone(),
            name: c.name.clone(),
            id: meta.id.clone(),
            marked: meta.secret_names.clone(),
            secrets: self.secret_cache.clone(),
        });
        let (settings, base) = (
            self.settings.clone(),
            self.store.base().map(Path::to_path_buf),
        );
        let accounts = Accounts {
            backend: self.backend.clone(),
            key: self.collection_key(cx),
            offline: self.connection() == Some(Connection::Offline),
            sync: self.sync_line(),
        };
        let view =
            cx.new(|cx| SettingsView::new(page, settings, base, collection, accounts, window, cx));
        self._app_settings_events =
            Some(cx.subscribe_in(&view, window, Self::on_app_settings_event));
        view.read(cx).focus_handle(cx).focus(window, cx);
        self.app_settings = Some(view);
        self.settings_view = None;
        self.settings_origin = None;
        self._settings_events = None;
        self.close_hover(cx);
        self.sync_chrome(window, cx);
    }

    pub(in crate::ui) fn hide_app_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_app_settings();
        if self.state.active.is_some() && !self.run_shown() && !self.history_shown() {
            self.focus_active_editor(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
        self.sync_chrome(window, cx);
    }

    pub(super) fn on_app_settings_event(
        &mut self,
        view: &Entity<SettingsView>,
        event: &AppSettingsEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            AppSettingsEvent::Close => self.hide_app_settings(window, cx),
            AppSettingsEvent::Changed => {
                self.settings = view.read(cx).settings().clone();
                self.apply_editor_settings(window, cx);
            }
            AppSettingsEvent::EditVariable { environment, name } => {
                let (environment, name) = (environment.clone(), name.clone());
                let page = view.read(cx).page();
                self.open_settings(window, cx);
                self.return_to(page, cx);
                self.settings_on(Some(&environment), Some(&name), window, cx);
            }
            AppSettingsEvent::OpenSignIn => {
                let page = view.read(cx).page();
                self.open_sign_in(Some(page), window, cx);
            }
            AppSettingsEvent::AccountChanged => self.start_sync(window, cx),
            AppSettingsEvent::SyncNow => self.sync_now(window, cx),
            AppSettingsEvent::AddVariable => {
                let page = view.read(cx).page();
                self.open_settings(window, cx);
                self.return_to(page, cx);
                if let Some(view) = self.settings_view.clone() {
                    view.update(cx, |v, cx| v.open_variable(None, window, cx));
                }
            }
        }
    }

    // Sign-in and the server.

    /// The open collection as the server knows it.
    pub(in crate::ui) fn collection_key(&self, cx: &App) -> Option<CollectionKey> {
        let c = self.state.collection.as_ref()?;
        Some(CollectionKey {
            id: self.notes.read(cx).id.clone(),
            name: c.name.clone(),
        })
    }

    /// The sign-in screen's answer: stored, else this session's.
    pub(in crate::ui) fn connection(&self) -> Option<Connection> {
        self.store.connection().or(self.session_connection)
    }

    /// Shows the sign-in screen on first launch: when it was never
    /// answered (no stored [`Connection`]).
    pub(in crate::ui) fn first_launch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.connection().is_none() {
            self.open_sign_in(None, window, cx);
        }
    }

    /// The sign-in screen, in place of everything under the title bar.
    /// From Settings (`origin`) it returns there; Esc leaves it unless it
    /// is the first launch's.
    pub(in crate::ui) fn open_sign_in(
        &mut self,
        origin: Option<AppPage>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<SignInView> {
        let can_go_back = self.connection().is_some();
        let backend = self.backend.clone();
        let view = cx.new(|cx| SignInView::new(backend, can_go_back, window, cx));
        self._sign_in_events = Some(cx.subscribe_in(&view, window, Self::on_sign_in_event));
        view.read(cx).focus_handle(cx).focus(window, cx);
        self.sign_in = Some(view.clone());
        self.sign_in_origin = origin;
        self.close_app_settings();
        self.close_hover(cx);
        self.sync_chrome(window, cx);
        view
    }

    pub(in crate::ui) fn sign_in_shown(&self) -> bool {
        self.sign_in.is_some()
    }

    pub(in crate::ui) fn close_sign_in(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sign_in = None;
        self._sign_in_events = None;
        match self.sign_in_origin.take() {
            Some(page) => self.open_app_settings(page, window, cx),
            None => {
                self.focus.focus(window, cx);
                self.sync_chrome(window, cx);
            }
        }
        if let Some(link) = self.pending_link.take() {
            self.open_shared(Some(link), window, cx);
        }
    }

    pub(super) fn on_sign_in_event(
        &mut self,
        _: &Entity<SignInView>,
        event: &SignInEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let SignInEvent::Done(connection) = event {
            self.store.set_connection(Some(*connection));
            self.session_connection = Some(*connection);
        }
        self.close_sign_in(window, cx);
        self.start_sync(window, cx);
    }

    /// Welcome's cloud chip: its dot's token and its text.
    pub(in crate::ui) fn connection_text(&self) -> (&'static str, String) {
        if self.connection() == Some(Connection::Offline) {
            return ("text-tertiary", "Offline".into());
        }
        if self.backend.account().is_none() {
            return ("text-tertiary", "Not signed in".into());
        }
        match self.backend.server() {
            Server::Cloud => ("success", "Connected to curl-runner Cloud".into()),
            Server::SelfHosted { url } => ("success", format!("Self-hosted · {}", host_of(&url))),
        }
    }

    /// The open collection settings return to Settings on `page`.
    pub(super) fn return_to(&mut self, page: AppPage, cx: &mut Context<Self>) {
        if let Some(view) = &self.settings_view {
            self.settings_origin = Some(page);
            view.update(cx, |v, cx| v.set_back_to_settings(true, cx));
        }
    }
}
