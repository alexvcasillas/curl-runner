# curl-runner studio, plan 1 of 3: foundation (validator in core, app shell, editor with diagnostics)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A macOS app that opens a collection folder, shows it as a tree, and edits its YAML. Validation runs live with the same rules as `curl-runner validate`, showing squiggles, hover messages, quick fixes and a Problems panel.

**Architecture:**
- First, the YAML validator and file discovery move from the CLI binary into `curl-runner-core`. The CLI's output stays byte-for-byte the same.
- Core then gains:
  - source ranges for issues;
  - quick fixes expressed as text edits.
- A new workspace crate, `curl-runner-studio`, holds the app.
  - Pure state modules (collection, documents, diagnostics) are unit-tested.
  - GPUI-kit views render that state.
  - The crate compiles to a stub on non-macOS targets, so the existing Linux and Windows CI keeps passing.

**Tech Stack:**
- Rust: edition 2024, toolchain 1.99.1.
- `curl-runner-core`.
- `gpui-kit` 0.8.1 from crates.io, which brings `gpui-pre` 0.3.10, with its tree-sitter YAML feature.
- `notify` 8, already a CLI dependency.

**Spec:** `docs/superpowers/specs/2026-10-29-curl-runner-studio-design.md`
- Mockups: `docs/superpowers/specs/studio-mockups/*.dc.html`. Open them in a browser; the content renders as static HTML.
- Design system: `docs/superpowers/specs/studio-design-system/` — `README.md`, `tokens.json`, and `components/*/README.md`.

**Later plans (not this one):**
- Plan 2: runs, inspector, environments, Keychain, history and export.
- Plan 3: the mocked server, with sign-in, sharing, the shared link, the customer view, offline mode, conflicts and the developer menu.

## Global Constraints

- macOS only for the app. On other targets the studio crate must still compile, as a binary that prints `curl-runner studio is macOS only` and exits 1.
  - The CI matrix in `.github/workflows/tests.yml` runs `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` on ubuntu, macos and windows. It must stay green.
- Put `publish = false` in the studio crate's `Cargo.toml`, because the release job runs `cargo publish --workspace --dry-run`.
- `curl-runner validate` output and exit codes must not change. Every test in `crates/curl-runner/src/validate_cmd/tests.rs` passes unmodified, apart from `use` paths.
- Validation severity follows the CLI.
  - `Error` blocks a run.
  - `Warning` and `Info` don't.
- A collection is a folder. Its request files are what `find_yaml_files(&[], true, root)` returns, minus a root-level `curl-runner.yaml`.
- Look:
  - Colors come only from `studio-design-system/tokens.json`, dark theme.
  - Fonts are Geist for the UI and Geist Mono for code.
  - The editor is 13px text on 18px rows.
  - The window chrome follows `studio-mockups/Main.dc.html`: title bar 48, sidebar 264, inspector 400, status bar 28.
- Public repo: no employer or customer names. Use placeholders: Acme, Payments API, api.example.com.
- Every change ships with tests (`AGENTS.md`).

## Review Focus

1. **Multi-byte text before an issue.** An emoji or accented `description` on an earlier line must not shift squiggles. Ranges are byte offsets on UTF-8 boundaries, and the studio converts them to the editor's positions. Pinned in Task 3.
2. **Shapes that make the validator `Crash`.** A YAML file whose root is a list, a scalar or null produces one Error diagnostic on line 1 with the crash message. It must never panic. Pinned in Tasks 2 and 6.
3. **A file changed on disk while open**, for example by `curl-runner validate --fix` or `git checkout`.
   - If the buffer is clean, it reloads.
   - If the buffer is dirty, it keeps the user's text and shows "Changed on disk", with Reload and Keep mine.

   Pinned in Task 6.
4. **Closing a tab or quitting with unsaved changes** asks Save / Don't save / Cancel. Nothing is lost silently. Pinned in Task 6 (model) and Task 8 (view).
5. **Files the tree can't parse.** Non-UTF-8 files, empty files and invalid YAML still appear in the tree, with an error dot. Opening one shows the parse error, and the app keeps working. Pinned in Task 5.

---

### Task 1: Move YAML file discovery into core

**Files:**
- Move: `crates/curl-runner/src/discover.rs` → `crates/curl-runner-core/src/discover.rs`, with its tests.
- Modify:
  - `crates/curl-runner-core/src/lib.rs`: `pub mod discover;`
  - `crates/curl-runner-core/Cargo.toml`: add `globset = "0.4.21"`.
- Modify: `crates/curl-runner/src/main.rs` and every user of `crate::discover`. Import from `curl_runner_core::discover` and delete the CLI module.
- Modify: `crates/curl-runner/Cargo.toml`. Drop `globset` if nothing else in the CLI uses it.

**Interfaces:**
- Produces: `curl_runner_core::discover::find_yaml_files(patterns: &[String], all: bool, cwd: &Path) -> Vec<String>`. Same behavior and signature as today.

- [ ] **Step 1:** Move the file and its `#[cfg(test)] mod tests` unchanged, then fix the imports.
- [ ] **Step 2:** Run `cargo test -p curl-runner-core discover` and `cargo test -p curl-runner`. Expected: all pass, with the same test count as before the move.
- [ ] **Step 3:** Run `cargo clippy --workspace --all-targets -- -D warnings`. Expected: clean.
- [ ] **Step 4:** Commit: `refactor(core): move YAML file discovery into curl-runner-core`.

### Task 2: Move the validator rules into core as `curl_runner_core::lint`

**Files:**
- Move into `crates/curl-runner-core/src/lint/`: `conditions.rs`, `fix.rs`, `global.rs`, `issues.rs`, `request.rs`, `security.rs`, `structure.rs`, `url.rs`.
- Create `crates/curl-runner-core/src/lint/mod.rs`, holding `PASSES`, `lint_text`, `lint_document` and the parse-error message mapping now in `validate_cmd/mod.rs`.
- Modify `crates/curl-runner/src/output/format.rs`. Move `truthy` into `curl_runner_core::jsval`, and leave a `pub use` in `format.rs` so CLI callers don't change.
- Keep in the CLI (`crates/curl-runner/src/validate_cmd/`):
  - `mod.rs`: options, `run`, `run_in`, `validate_all`, and a `validate_file` that calls core;
  - `report.rs`;
  - `tests.rs`.

**Interfaces:**
- Produces, in `curl_runner_core::lint`. These are the public versions of today's types, unchanged in shape.
  - `pub enum Severity { Error, Warning, Info }`
  - `pub struct Issue { pub path: String, pub severity: Severity, pub message: String, pub fix: Option<Fix> }`
  - `pub struct Fix { pub description: String, pub value: FixValue }`
  - `pub enum FixValue { Set(Json), Delete, Rename { to: String, value: Option<Json> } }`
  - `pub struct Crash(pub String)`
  - `pub fn lint_document(content: &Json) -> Result<Vec<Issue>, Crash>`: runs `PASSES` in order.
  - `pub enum LintOutcome { ParseError(String), Linted { content: Json, issues: Vec<Issue> } }`
  - `pub fn lint_text(text: &str) -> Result<LintOutcome, Crash>`:
    - `ParseError` carries today's message text, for example `Failed to parse YAML: YAML Parse error: Unexpected token`.
  - `pub fn fix_document(content: &Json, issues: &[Issue]) -> Option<(String, usize)>`: today's function, unchanged.
  - `pub fn is_valid(issues: &[Issue]) -> bool`: true when no issue is `Severity::Error`.

- [ ] **Step 1: Write the failing core tests** in `crates/curl-runner-core/src/lint/tests.rs`:

```rust
#[test]
fn unknown_key_is_a_warning_with_rename_fix() {
    let LintOutcome::Linted { issues, .. } = lint_text("request:\n  url: https://api.example.com\n  retry:\n    count: 3\n    backof: 2\n").unwrap() else { panic!() };
    let i = issues.iter().find(|i| i.path == "request.retry.backof").unwrap();
    assert_eq!(i.severity, Severity::Warning);
    assert!(matches!(&i.fix, Some(Fix { value: FixValue::Rename { to, .. }, .. }) if to == "backoff"));
    assert!(is_valid(&issues));
}

#[test]
fn invalid_method_is_an_error() {
    let LintOutcome::Linted { issues, .. } = lint_text("request:\n  url: https://api.example.com\n  method: FETCH\n").unwrap() else { panic!() };
    assert!(issues.iter().any(|i| i.path == "request.method" && i.severity == Severity::Error));
    assert!(!is_valid(&issues));
}

#[test]
fn parse_error_is_reported_not_crashed() {
    assert!(matches!(lint_text("request: [\n"), Ok(LintOutcome::ParseError(m)) if m.starts_with("Failed to parse YAML")));
}

#[test]
fn list_root_crashes_with_message() {
    assert!(matches!(lint_text("- a\n- b\n"), Err(Crash(m)) if m.contains("is not an Object")));
}
```

  The last test pins the crash message `content is not an Object. (evaluating '"request" in content')`.

- [ ] **Step 2:** Run `cargo test -p curl-runner-core lint`. Expected: FAIL, because the module doesn't exist yet.
- [ ] **Step 3:** Move the rule files and build `lint/mod.rs` with the interfaces above.
  - The CLI's `validate_file` calls `lint_text` and `fix_document`.
  - Rule logic doesn't change.
- [ ] **Step 4:** Run `cargo test -p curl-runner-core lint` and `cargo test -p curl-runner`. Expected: PASS, with every existing `validate_cmd/tests.rs` test unmodified apart from `use` paths.
- [ ] **Step 5:** Run `cargo clippy --workspace --all-targets -- -D warnings`. Expected: clean.
- [ ] **Step 6:** Commit: `refactor(core): move validate rules into curl_runner_core::lint`.

### Task 3: Source ranges for issues

**Files:**
- Create: `crates/curl-runner-core/src/lint/spans.rs`, with tests in the same file.

**Interfaces:**
- Consumes: `Issue` and `Crash` from Task 2.
- Produces:
  - `pub struct NodeSpans { pub key: Option<Range<usize>>, pub value: Range<usize> }`. These are byte ranges into the text. `key` is `None` for sequence items and the root.
  - `pub fn locate(text: &str, path: &str) -> Option<NodeSpans>`. It resolves paths in the validator's syntax: `request.retry.backof`, `requests[2].url`, `collection.requests[0].expect.status`. The empty path is the root.
  - `pub fn issue_range(text: &str, issue: &Issue) -> Range<usize>`. Rules:
    - a `Rename` fix, or a message starting `Unknown key`, gives the key span;
    - otherwise the value span when the value is a scalar;
    - otherwise the key span;
    - an unresolvable path or the empty path gives the first line, trimmed;
    - an empty text gives `0..0`.
  - `pub fn parse_error_range(text: &str) -> Range<usize>`: the line libyaml reports for the error, or the first line.

  Build the spans from libyaml events using the `unsafe_libyaml` marks, the way `yaml_load.rs` drives the parser. Use the first document only.

- [ ] **Step 1: Write the failing tests:**

```rust
const DOC: &str = "# Créer 🚀\nrequest:\n  method: FETCH\n  retry:\n    backof: 2\nrequests:\n  - url: x\n";
fn s(r: Range<usize>) -> &'static str { &DOC[r] }

#[test] fn key_span_for_unknown_key() { assert_eq!(s(issue_range(DOC, &issue("request.retry.backof", "Unknown key \"backof\""))), "backof"); }
#[test] fn value_span_for_bad_value() { assert_eq!(s(issue_range(DOC, &issue("request.method", "Invalid method"))), "FETCH"); }
#[test] fn sequence_index_paths() { assert_eq!(s(locate(DOC, "requests[0].url").unwrap().value), "x"); }
#[test] fn multibyte_prefix_does_not_shift() { assert!(DOC.is_char_boundary(issue_range(DOC, &issue("request.method", "x")).start)); }
#[test] fn root_and_missing_paths_fall_back_to_first_line() { assert_eq!(s(issue_range(DOC, &issue("", "Missing request(s)"))), "# Créer 🚀"); assert_eq!(s(issue_range(DOC, &issue("nope.x", "m"))), "# Créer 🚀"); }
#[test] fn parse_error_points_at_bad_line() { let t = "request:\n  url: [\n"; assert!(t[parse_error_range(t)].contains("url")); }
```

  `issue(path, msg)` is a test helper that builds an `Error` issue with no fix.

- [ ] **Step 2:** Run `cargo test -p curl-runner-core spans`. Expected: FAIL.
- [ ] **Step 3:** Implement the three functions as specified.
- [ ] **Step 4:** Run `cargo test -p curl-runner-core spans`. Expected: PASS.
- [ ] **Step 5:** Commit: `feat(core): map lint issues to source ranges`.

### Task 4: Quick fixes as text edits

**Files:**
- Create: `crates/curl-runner-core/src/lint/edit.rs`, with tests.

**Interfaces:**
- Consumes: `locate` from Task 3, and `yaml_text::{yaml_str, map_key_text}`.
- Produces:
  - `pub struct TextEdit { pub range: Range<usize>, pub new_text: String }`
  - `pub fn fix_edit(text: &str, issue: &Issue) -> Option<TextEdit>`. It returns `None` without a fix, or when the path can't be located.
    - `Rename { to, value: None }` replaces the key span with `map_key_text(to)`.
    - `Rename { to, value: Some(v) }` also replaces the value span.
    - `Set(v)` replaces the value span with the scalar text `fix.rs` would emit for `v`.
    - `Delete` removes the entry's whole lines, including the newline.

- [ ] **Step 1: Write the failing tests.**

  Rename case: `backof: 2` becomes `backoff: 2`.

  For the Set and Delete cases, take one issue each from the existing fixable rules. The `fix.rs` tests show which rules produce `Set` and which produce `Delete`. Each case gets one assertion: after applying the edit, `lint_text` no longer reports that issue.

  Round-trip test: for every fixable issue in `validate_cmd/tests.rs` fixtures that `fix_document` fixes, applying `fix_edit` and then linting yields the same issue set as linting `fix_document`'s output. Compare paths and severities only.
- [ ] **Step 2:** Run `cargo test -p curl-runner-core edit`. Expected: FAIL.
- [ ] **Step 3:** Implement `fix_edit`.
- [ ] **Step 4:** Run `cargo test -p curl-runner-core`. Expected: PASS.
- [ ] **Step 5:** Commit: `feat(core): express lint fixes as text edits`.

### Task 5: Studio crate, GPUI-kit spike, theme

**Files:**
- Create: `crates/curl-runner-studio/Cargo.toml`.
  - Set `publish = false`.
  - Put `gpui-kit = { version = "=0.8.1", features = ["tree-sitter-yaml"] }` under `[target.'cfg(target_os = "macos")'.dependencies]`, using the YAML feature's real name from the crate. Put the matching `gpui-pre` pin there too.
  - Put `curl-runner-core` under plain `[dependencies]`.
- Create: `crates/curl-runner-studio/src/main.rs`.
  - On macOS it launches the app.
  - Otherwise it prints `curl-runner studio is macOS only` to stderr and exits 1.
- Create: `crates/curl-runner-studio/src/theme.rs` with an `include_str!` of `../../../docs/superpowers/specs/studio-design-system/tokens.json`.
- Create: `crates/curl-runner-studio/SPIKE.md`.
- Modify: root `Cargo.toml` `members`, to add `crates/curl-runner-studio`.

**Interfaces:**
- Produces:
  - `pub fn color(name: &str) -> Rgba` (dark theme, aliases like `{accent-blue}` resolved). It panics on an unknown name. That is fine, because names are compile-time literals that tests cover.
  - `pub fn method_color(method: &str) -> Rgba`:
    - GET → `method-get`;
    - POST → `method-post`;
    - PUT → `method-put`;
    - PATCH → `method-patch`;
    - DELETE → `method-delete`;
    - anything else → `method-other`.
  - `Rgba` is a plain `{ r, g, b, a: f32 }` struct in `theme.rs`, outside the macOS gate. The view layer converts it to GPUI's color type.
  - `SPIKE.md` records, for `gpui-kit` 0.8.1, the exact types and functions used for the items below. Tasks 7 and 8 rely on it.
    1. The YAML code editor with line numbers, folding and the find bar.
    2. Setting diagnostics with severity, range and message.
    3. A hover provider.
    4. A code-action provider that applies a text edit.
    5. Converting a byte offset to an editor position.
    6. Loading Geist and Geist Mono.

    For any item that doesn't exist, it records the fallback: range decorations plus a popover.

- [ ] **Step 1: Write the failing tests** in `theme.rs`:
  - `color("bg-100")` is `#0a0a0a`;
  - `color("text-gutter")` is `#808080`;
  - `method_color("GET") == color("accent-blue")`;
  - `method_color("TRACE") == color("method-other")`;
  - every token in `tokens.json` resolves without panicking.
- [ ] **Step 2:** Run `cargo test -p curl-runner-studio theme`. Expected: FAIL.
- [ ] **Step 3:** Implement `theme.rs`, and the spike: a throwaway window with a YAML editor, one diagnostic, one hover and one code action. Write `SPIKE.md` from what worked, then reduce `main.rs` to opening an empty 1440×900 window with background `bg-100`.
- [ ] **Step 4:** On macOS, run `cargo test -p curl-runner-studio` and `cargo clippy --workspace --all-targets -- -D warnings`. Then run `cargo check -p curl-runner-studio --target x86_64-unknown-linux-gnu`, or rely on CI. Expected: all clean.
- [ ] **Step 5:** Commit: `feat(studio): crate scaffold, design tokens, GPUI-kit spike notes`.

### Task 6: State model: collection, documents, diagnostics

**Files:**
- Create in `crates/curl-runner-studio/src/model/` (no GPUI imports, so it compiles on every target):
  - `collection.rs`
  - `document.rs`
  - `diagnostics.rs`
  - `mod.rs`

**Interfaces:**
- Consumes: `find_yaml_files`, `lint_text`, `issue_range`, `parse_error_range`, `fix_edit`, `TextEdit` and `Severity`.
- Produces:
  - **`collection.rs`**
    - `pub enum TreeNode { Folder { name: String, children: Vec<TreeNode> }, File { name: String, rel_path: PathBuf, method: Option<String>, parse_ok: bool } }`
    - `pub struct Collection { pub root: PathBuf, pub name: String, pub tree: Vec<TreeNode> }`
    - `pub fn Collection::open(root: &Path) -> io::Result<Collection>`:
      - the tree is built from `find_yaml_files(&[], true, root)` minus a root `curl-runner.yaml`;
      - within each level, folders come before files, and both are sorted by name, case-insensitive;
      - `method` is the `request.method` of single-request files (default `GET`). It is `None` for `requests:` files;
      - `parse_ok` is false when the file isn't UTF-8 or fails to parse.
    - `fn create_file(&mut self, rel: &Path, contents: &str) -> io::Result<()>`
    - `fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()>`
    - `fn delete(&mut self, rel: &Path) -> io::Result<()>`

    Each of these refreshes `tree`.
  - **`diagnostics.rs`**
    - `pub struct StudioDiagnostic { pub range: Range<usize>, pub severity: Severity, pub message: String, pub fix: Option<(String, TextEdit)> }`. The `String` in `fix` is the fix description.
    - `pub fn diagnose(text: &str) -> Vec<StudioDiagnostic>`:
      - a parse error is one Error at `parse_error_range`;
      - a `Crash` is one Error on the first line, carrying the crash message;
      - otherwise there is one diagnostic per issue, at `issue_range`, with `fix_edit`.
    - `pub struct Counts { pub errors: usize, pub warnings: usize }`
    - `pub fn counts(d: &[StudioDiagnostic]) -> Counts`
    - `pub fn can_run(d: &[StudioDiagnostic]) -> bool`
  - **`document.rs`**
    - `pub struct Document { pub rel_path: PathBuf, pub text: String, pub saved_text: String, pub disk_conflict: bool, pub diagnostics: Vec<StudioDiagnostic> }`
    - `fn open(root, rel) -> io::Result<Document>`
    - `fn set_text(&mut self, text: String)`, which re-diagnoses;
    - `fn is_dirty(&self) -> bool`
    - `fn save(&mut self, root) -> io::Result<()>`
    - `fn on_disk_changed(&mut self, disk: String)`:
      - if the buffer is clean, it reloads;
      - if it is dirty, it sets `disk_conflict` and keeps `text`.
    - `fn resolve_conflict(&mut self, keep_mine: bool)`
    - `fn apply_fix(&mut self, idx: usize)`
    - `pub enum CloseDecision { Close, Ask }`, and `fn close_decision(&self) -> CloseDecision`, which returns `Ask` when the document is dirty.

- [ ] **Step 1: Write the failing tests**, using `tempfile = "3.27.0"` as a dev-dependency (the version core and the CLI use) for folders. Names and assertions:
  - `tree_orders_folders_first_and_skips_root_config`: `curl-runner.yaml` absent, `auth/` before `a.yaml`.
  - `method_read_from_single_request_files`: `POST`, and `None` for a `requests:` file.
  - `unparseable_and_non_utf8_files_listed_with_parse_ok_false`
  - `crash_becomes_single_error_on_first_line`: input `- a\n`, giving one Error at `0..3`.
  - `unknown_key_warning_does_not_block_run`: `can_run` is true and `counts` is `{0,1}`.
  - `invalid_value_blocks_run`
  - `apply_fix_renames_key_and_clears_issue`
  - `disk_change_reloads_clean_buffer`
  - `disk_change_on_dirty_buffer_sets_conflict_and_keeps_text`
  - `close_dirty_document_asks`
  - `create_rename_delete_refresh_tree`
- [ ] **Step 2:** Run `cargo test -p curl-runner-studio model`. Expected: FAIL.
- [ ] **Step 3:** Implement the three modules as specified.
- [ ] **Step 4:** Run `cargo test -p curl-runner-studio model`. Expected: PASS on macOS. CI covers Linux and Windows.
- [ ] **Step 5:** Commit: `feat(studio): collection, document and diagnostics model`.

### Task 7: Workspace window: chrome, tree, tabs, editor

**Files:**
- Create in `crates/curl-runner-studio/src/ui/`, gated to macOS:
  - `app.rs`: app setup, fonts, the action and keymap table;
  - `title_bar.rs`
  - `sidebar.rs`: the collection tree;
  - `editor_pane.rs`: tabs, breadcrumb and editor;
  - `status_bar.rs`
  - `welcome.rs`: the empty state;
  - `state.rs`: last-opened folder.

**Interfaces:**
- Consumes:
  - the model from Task 6;
  - `theme::color` and `theme::method_color`;
  - the editor APIs listed in `SPIKE.md`.
- Produces:
  - `pub struct AppState { collection: Option<Collection>, open: Vec<Document>, active: Option<usize> }`, held by the root view.
  - `pub fn last_folder() -> Option<PathBuf>` and `pub fn remember_folder(p: &Path)`. These read and write `~/Library/Application Support/curl-runner-studio/state.json` as `{ "lastFolder": "<path>" }`.
  - Actions: `OpenFolder` (⌘O), `SaveFile` (⌘S), `CloseTab` (⌘W), `NewRequest` (⌘N) and `NewCollection` (⇧⌘N). `NewRequest` and `NewCollection` open the dialogs in Task 9.
- Behavior to match the mockups:
  - **Launch:** reopen `last_folder()` if it still exists. Otherwise show `welcome.rs` (`studio-mockups/Welcome.dc.html`). Of its actions, only New collection, Open folder and Import are active.
    - Import opens Task 9's New dialog on the From curl template. The full Import dialog arrives in plan 2.
    - Open shared link is disabled with the tooltip "Coming soon"; plan 3 enables it.
  - **Tree context menu:** New request here, Rename, and Delete. Delete asks for confirmation first. These call `Collection::rename` and `Collection::delete`, and update open tabs to match.
  - **Sidebar** (`Main.dc.html`):
    - collapsible folders with a chevron;
    - file rows with the method badge (`theme::method_color`) and name without the extension;
    - a red dot for `parse_ok == false`;
    - selecting a row opens it in a tab;
    - a Filter input that narrows rows by name, case-insensitive;
    - "Shared with me" and the sync footer are omitted until plan 3.
  - **Tabs:**
    - method badge and file name, plus a dot when the file is dirty;
    - closing a dirty tab shows a Save / Don't save / Cancel dialog (Review Focus 4);
    - the breadcrumb shows collection / folders / file.
  - **Editor:**
    - the GPUI-kit YAML code editor, 13px Geist Mono on 18px rows, line numbers, folding and find (⌘F);
    - every edit calls `Document::set_text`;
    - file changes on disk, through `notify` on the collection root, call `on_disk_changed`;
    - when `disk_conflict` is set, a callout with Reload and Keep mine appears above the editor, styled as the design system's `Callout`, warning variant.
  - **Status bar:** collection name, `YAML`, `Ln x, Col y`, and `curl-runner <core VERSION>`.
- Window title: `curl-runner studio — <collection name>`.

- [ ] **Step 1: Write the failing tests:**
  - `state_round_trips_last_folder`: write to a temp dir through an injectable base path.
  - A GPUI smoke test, `workspace_renders_tree_rows`. It uses `gpui-kit`'s `test-support` feature, or whatever `SPIKE.md` names. It opens a fixture collection of 2 folders and 3 files and asserts 5 rows exist.
- [ ] **Step 2:** Run `cargo test -p curl-runner-studio`. Expected: FAIL.
- [ ] **Step 3:** Implement the views as described.
- [ ] **Step 4:** Run `cargo test -p curl-runner-studio` and `cargo clippy --workspace --all-targets -- -D warnings`. Expected: PASS and clean. Then run `cargo run -p curl-runner-studio`, open `packages/cli/examples`, and compare against `Main.dc.html` by eye.
- [ ] **Step 5:** Commit: `feat(studio): workspace window with tree, tabs and YAML editor`. Attach screenshots of the welcome and workspace states to the PR.

### Task 8: Diagnostics in the editor, Problems panel, blocked Run

**Files:**
- Create in `crates/curl-runner-studio/src/ui/`:
  - `diagnostics_view.rs`: the editor diagnostics, hover and code actions;
  - `problems_panel.rs`
  - `run_button.rs`
- Modify `title_bar.rs` and `editor_pane.rs`.

**Interfaces:**
- Consumes: `Document::diagnostics`, `counts`, `can_run`, `Document::apply_fix`, and `SPIKE.md` items 2–5.
- Produces:
  - `pub struct RunRequested { pub rel_path: PathBuf }`, emitted by the Run button. In this plan it is dispatched and logged; plan 2 executes it.
  - `fn byte_to_position(text: &str, offset: usize) -> (u32, u32)`, in `diagnostics_view.rs`, in the unit the editor expects as recorded in `SPIKE.md`. It is unit-tested.
- Behavior (`studio-mockups/Errors.dc.html`, `studio-design-system/components/EditorDiagnostics/README.md`):
  - **Squiggles:** red for Error, amber for Warning, blue for Info. Plus a gutter icon and a line tint, using `diag-error-line` and `diag-warning-line`.
  - **Hover:** the message plus `curl-runner · <path>`.
  - **Quick fix:** shown as a code action, with the fix description as its label, for example `Rename to "backoff"`. ⌘. applies the first fix at the cursor.
  - **Breadcrumb counts:** "N errors · M warnings", hidden at zero, and "Unsaved" while the file is dirty.
  - **Scrollbar ticks** at each diagnostic's line. If GPUI-kit can't draw them, record that in `SPIKE.md` and skip them.
  - **Problems panel** below the editor:
    - 178px tall, toggled with ⇧⌘M;
    - one row per diagnostic, with icon, message, path and `line:col`;
    - clicking a row moves the cursor to the range;
    - it lists the active file only.
  - **Run button:**
    - disabled with the tooltip "Fix N errors to run" while `!can_run`;
    - enabled otherwise.
  - **Debounce:** re-diagnose 150 ms after the last keystroke.

- [ ] **Step 1: Write the failing tests:**
  - `byte_to_position_handles_multibyte`: an emoji on line 0 doesn't change line 1's column 2.
  - `run_disabled_reason_counts_errors`: a pure helper `run_state(&[StudioDiagnostic]) -> Result<(), String>` returns `Err("Fix 2 errors to run")` and `Err("Fix 1 error to run")`.
  - A GPUI smoke test, `problems_panel_lists_active_file_issues`.
- [ ] **Step 2:** Run `cargo test -p curl-runner-studio`. Expected: FAIL.
- [ ] **Step 3:** Implement the behavior as described.
- [ ] **Step 4:** Run `cargo test -p curl-runner-studio`. Expected: PASS. Then run the app manually on `method: FETCH`, `timout: 5000` and `count: three`, and compare against `Errors.dc.html`.
- [ ] **Step 5:** Commit: `feat(studio): live diagnostics, quick fixes and Problems panel`. Attach a screenshot.

### Task 9: New request / new collection dialogs and the command palette

**Files:**
- Create in `crates/curl-runner-studio/src/ui/`:
  - `new_dialog.rs`
  - `palette.rs`
- Create: `crates/curl-runner-studio/src/model/templates.rs`, which has no GPUI dependency.

**Interfaces:**
- Consumes:
  - `Collection::create_file`;
  - `curl_runner_core::convert` (the API the CLI's `convert_cmd.rs` calls) for "From curl";
  - the actions from Task 7.
- Produces:
  - `pub fn request_template(name: &str, method: &str, url: &str) -> String`. Its output must lint with zero issues:

```yaml
request:
  name: <name>
  url: <url>
  method: <METHOD>
```

  - `pub fn file_name_for(name: &str) -> String`: a kebab-case slug plus `.yaml`. For example, `Create charge` becomes `create-charge.yaml`.
  - `pub fn palette_matches(query: &str, items: &[PaletteItem]) -> Vec<(usize, Vec<usize>)>`: a fuzzy subsequence match, case-insensitive, returning the matched character indices for highlighting. Results are ranked by:
    1. a contiguous match first;
    2. then the earliest match;
    3. then the shorter label.

    `PaletteItem { group: Group, label: String, detail: String }`, where `Group` is `Requests`, `Collections` or `Actions`. Environments arrive in plan 2.
- Behavior:
  - **New dialog** (`NewRequest.dc.html`):
    - a Request / Collection switch;
    - templates: Blank, and From curl, which pastes a curl command and converts it through core;
    - name, method and URL fields, and a folder select;
    - Create (⌘↵) writes the file and opens it.
    - The `${` variable autocomplete in the URL field is deferred to plan 2, when variables exist.
  - **New collection:** pick an empty folder, then write a starter `requests.yaml` from `request_template("First request", "GET", "https://api.example.com")`.
  - **Palette (⌘K)** (`Palette.dc.html`):
    - groups: Requests (every file in the collection) and Actions (Open folder, New request, New collection, Toggle Problems);
    - ↵ opens or runs the selection;
    - Esc closes it.

- [ ] **Step 1: Write the failing tests:**
  - `template_lints_clean`
  - `file_name_slugs`: `"Create charge"` → `"create-charge.yaml"`, and `"  Ünïcode / x "` → `"unicode-x.yaml"`.
  - `palette_ranks_contiguous_first`
  - `palette_returns_highlight_indices`
  - `from_curl_creates_lintable_file`
- [ ] **Step 2:** Run `cargo test -p curl-runner-studio`. Expected: FAIL.
- [ ] **Step 3:** Implement the dialogs and the palette as described.
- [ ] **Step 4:** Run `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`. Expected: PASS and clean.
- [ ] **Step 5:** Commit: `feat(studio): new request/collection dialogs and command palette`. Attach screenshots.
