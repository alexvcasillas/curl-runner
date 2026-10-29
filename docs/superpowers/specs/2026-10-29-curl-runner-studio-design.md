# curl-runner studio — v1 design (local app, mocked server)

Status: ready for owner review · 2026-10-29

Mockups (16 screens): https://claude.ai/artifact/EeLYjq1iRGw7SwAWSbvnps
Design system: https://claude.ai/artifact/UU6nU4BeijNvfgMiN6jBQU

Both links are private to the owner. Share them with anyone building from this document.

## Why

curl-runner is a CLI: you write YAML request files and run them as tests. The studio is a desktop app for the same files. You browse collections, edit YAML with live validation, run requests or whole collections, and read the results.

Later, a team can share a collection read-only with its customers, the way Postman collections are shared today. The customer follows a link, the app opens and pulls the collection, and the customer runs it with their own credentials.

The business model follows Supabase: everything is open source, and people either use the managed server (curl-runner Cloud) or host it themselves.

This document covers the first version only: the desktop app with the server mocked. The sync and sharing server is a separate project that comes next.

## Goals (v1)

1. A macOS desktop app, built in Rust with GPUI-kit, that looks and behaves like the mockups and follows the design system.
2. Work with real curl-runner YAML on disk.
   - Open a folder as a collection.
   - Browse it as a tree of folders and requests.
   - Edit, create, rename and delete files.
3. Live validation in the editor, using the same rules as `curl-runner validate`.
   - Squiggles, gutter icons, hover popups and a Problems panel.
   - Quick fixes where the validator offers one, such as renaming `backof` to `backoff`.
4. Real runs through `curl-runner-core`, for a single request or a whole collection.
   - Show the real response, the checks, the headers, the timing and the retry attempts.
   - Stop a run, re-run the failed requests, and export a report.
5. Environments and variables.
   - Non-secret values live where the CLI already reads them.
   - Secrets live in the macOS Keychain.
6. Every server-backed flow from the mockups works end to end against a mocked backend, with believable fake data:
   - sign-in and the server choice;
   - sync status;
   - sharing and share links;
   - opening a shared collection as a customer;
   - the read-only customer view;
   - offline mode and sync conflicts.
7. A clean seam between the app and the mocked backend, so the real server can replace the mock later without UI changes.

## Non-goals (v1)

- The real server: no accounts, database, hosting, auth provider or sync protocol.
- Linux and Windows. GPUI-kit supports them, but packaging and keychain work wait.
- Code signing, notarization, auto-update, and distribution through Homebrew or npm. A locally built `.app` is enough.
- Light theme in the app. The design system defines it; v1 ships dark only.
- Feature parity with every CLI flag. The studio runs what the YAML says, with the environment you picked.

## Anti-goals (a failure even if it technically works)

- **The studio and the CLI disagree.**
  - A file the studio calls valid fails `curl-runner validate`, or the reverse.
  - A run gives a different result than `curl-runner` on the same file with the same variable values.
- **A secret leaves the machine.**
  - It is written into a YAML or `.env` file the user didn't choose.
  - It shows up in logs, exported reports, or anything the mock "shares" or "syncs".
- **The mock leaks into the UI code.** If replacing it with a real server means touching views, the seam failed.
- **The app drifts from the design system:** ad-hoc colors, sizes or components that aren't in it.
- **Anything private about the owner's employer or customers gets into this public repo.** Use placeholders such as Acme, Payments API and api.example.com.

## Constraints

- **Platform:** macOS only.
- **Language and toolchain:** Rust, inside the existing Cargo workspace, which uses edition 2024 and toolchain 1.99.1 per `rust-toolchain.toml`. Add the app as a new member crate.
- **UI:** GPUI-kit, formerly gpui-component (https://gpui-kit.com). Use its code editor with:
  - tree-sitter YAML;
  - line numbers and folding;
  - the find bar;
  - its `Diagnostic`, hover and code-action hooks for validation.
- **Engine:** `curl-runner-core` for parsing, config and env loading, and execution.
  - The executor already takes an env lookup (`Executor::with_env`).
  - It streams events to an `EventSink`, which drives live run progress.
  - It exposes a cancel handle, which drives Stop.
- **Look:** the design system tokens and components, in dark theme. Read its README first. Typography is Geist and Geist Mono, with the editor at 13px on 18px rows.
- **Public repo:** follow `AGENTS.md`.

## Decided

### Real vs mocked

| Area | v1 |
| --- | --- |
| Files, tree, editor, save | Real (disk) |
| Validation | Real, same rules as `curl-runner validate` |
| Running requests and collections | Real, through `curl-runner-core` |
| Environments and `.env` values | Real (disk, CLI format) |
| Secrets | Real (macOS Keychain) |
| Run history | Real, stored locally |
| Sign-in, server choice | Mocked |
| Sync status, offline, conflicts | Mocked |
| Share dialog, share links, invites | Mocked |
| Opening a shared link, customer read-only view | Mocked (fixture collections) |

### Validation severity

The studio follows the CLI.

- **Errors block a run:** an unsupported value, a wrong type or invalid YAML. Run is disabled, and the inspector shows "Can't run until N errors are fixed".
- **Warnings don't block a run:** for example an unknown property. The CLI ignores unknown keys at run time (`model/mod.rs`: no `deny_unknown_fields`) and `validate` reports them as warnings.

### Environments and secrets

An environment in the studio is the CLI's `environment`.

- Its non-secret values come from the files the CLI already loads: `.env`, `.env.local`, `.env.{environment}` and `.env.{environment}.local` in the collection folder (`env.rs`). That way the same folder and environment behave the same in the CLI.
- Secrets are stored in the macOS Keychain, keyed by collection, environment and variable name.
- At run time, values come from three places, highest priority first:
  1. values captured during the run (curl-runner's store);
  2. Keychain secrets;
  3. the environment's `.env` files.

  These are passed to the executor through `with_env`.
- **The owner decided that the CLI does not read the Keychain.**
  - Agreement between the studio and the CLI means this: the same file with the same variable values gives the same result. Secrets are simply supplied differently.
  - In the terminal, the user provides secrets themselves, for example in `.env.local` or as environment variables.
  - The studio offers "Copy as CLI command", which lists the variables the user needs to set.
  - The CLI is out of scope for v1, apart from moving the validator.
- A secret is never written to disk by the studio, never sent to the mock backend, and is masked in the UI unless revealed. Reports exported from the studio redact secrets through the core's redactor.

### Layout and screens

These are the 16 boards on the canvas. Each must exist in v1 and behave as described there.

1. **Workspace.** Three columns:
   - collections tree, 264px;
   - editor;
   - inspector, 400px.

   Plus a title bar and a status bar.
2. **YAML errors.** Diagnostics, the Problems panel and the blocked-run state.
3. **First launch.** Empty state with New collection, Open folder, Open shared link, and Import.
4. **Sign in and server choice.** Mocked.
5. **Command palette (⌘K).** Requests, collections, environments and actions.
6. **Settings.** General, Editor, Appearance, Accounts & sync (mocked), Keyboard shortcuts and Updates.
7. **Run collection.** Live progress and results, Stop, Re-run failed, and Export (JUnit, JSON, Markdown).
8. **Failed request.** Expected vs actual for each check, a body diff, and a retry timeline.
9. **Run history.** Stored locally, with comparison to an earlier run.
10. **Environments and variables.** Values from the `.env` files and secrets from the Keychain.
11. **Share dialog.** Mocked. Shows variable names only, never values.
12. **Opening a shared link.** Mocked: the `curl-runner://` deep link and the step where the customer enters credentials.
13. **Customer read-only view.** Mocked fixture collection. The editor is locked, Run works with the customer's own credentials, and "Duplicate to my workspace" is available.
14. **Import.** Real for curl, through core's `convert`. Postman and OpenAPI tabs exist but may show "Coming soon" in v1.
15. **New request / new collection.** Real.
16. **Offline and sync conflict.** Mocked.

### The backend seam

The app talks to "the server" only through a Rust trait boundary owned by the studio crate.

- v1 ships one implementation, an in-process mock with fixture data: an account, a workspace, people with access, a shared Shipping API collection, and sync events.
- A developer-only menu toggles mock states so every screen is reachable: offline, conflict on next sync, an expired share link, and sign-in failure.

## Prerequisite in the existing code

The YAML validator lives in the CLI binary crate (`crates/curl-runner/src/validate_cmd/`), not in `curl-runner-core`.

- Its issues are addressed by key path, such as `request.retry.backof`, not by line and column.
- They carry a severity (Error, Warning or Info) and an optional `Fix`.

The studio needs this as a library, so v1 starts with two changes:

1. Move the validator into a library crate the CLI and studio both use. That can be `curl-runner-core` or a new crate.
2. Map key paths to source ranges in the YAML text, so diagnostics can be underlined.

`curl-runner validate` output must not change. The existing tests and the `parity/` suite prove that.

## Calls made while writing this (review these)

These were settled by Claude, not by the owner. They can be overridden.

- **Which variables are secrets.**
  - Any variable whose name starts with `SECRET_` is a secret by default, matching the CLI's redaction rule.
  - The user can mark any other variable as secret in the Environments screen.
  - Every secret value is handed to the core `Redactor` (`add_secrets`) before a run. That covers exported reports and run history, so no core change is needed.
- **Run history** is stored locally and redacted. It keeps response bodies, but only up to a size cap the builder picks.
- **Where environments come from.**
  - The environment list is discovered from `.env.{name}` files in the collection folder, ignoring `.local` variants. A folder with only `.env` has one environment, Default.
  - Creating an environment writes an empty `.env.{name}`.
  - Edits to non-secret values are written to `.env.{name}`.
  - The studio reads `.env.local` and `.env.{name}.local` but never writes them.
  - The environment picked in the studio behaves like the CLI flag, so it wins over an `environment` set in YAML, per the config precedence in `config.rs`.
- **The collection model.**
  - A collection is a folder. It has no manifest.
  - Request files are `*.yaml` and `*.yml`, found recursively, ignoring hidden entries.
  - A `curl-runner.yaml` at the root is the project's global config, as in the CLI, not a request file.
  - File discovery should reuse the CLI's `discover.rs` logic, moved next to the validator.
- **The validator** moves into `curl-runner-core`, as a new module that sits beside the existing `validate.rs`, which checks responses.
  - YAML that fails to parse shows a single error at the parser's reported location.
  - Fixes are applied as text edits computed from the key-path fix.
- **How runs behave.**
  - Run saves the file first, so a run always uses what's on disk, as the CLI does.
  - A collection run uses the CLI's semantics: files in discovery order, and one captured-value store shared across the run.
  - Running one request from a multi-request file runs that request alone. It starts from the values captured in that file's last run, if there was one.
  - Re-run failed reuses the store from the run it repeats.
- **The mock backend.**
  - It is stateful: sign-in, share links, people with access and the dev-menu toggles persist in a JSON file under Application Support.
  - The trait is async, as the real server will be.
  - "Duplicate to my workspace" writes real files to a folder the user picks.
  - A simulated conflict compares the real local file against a modified copy that the mock produces.
- **Tests.**
  - Logic lives outside the views and gets unit tests: validator moves, range mapping, environment resolution, the mock, and report export.
  - Views get smoke tests where GPUI's test harness allows.
  - Visual match with the mockups is checked by a person, from screenshots attached to each PR.
- **Report export.**
  - JSON uses the same shape as the CLI's JSON output (`output/json.rs`).
  - Markdown and JUnit XML are new, written in the studio crate.

## Left to the builder

- Crate name and internal module layout. A suggestion: `crates/curl-runner-studio`.
- How GPUI-kit is pinned and which version, since GPUI tracks Zed.
  - The first task is a short spike. It confirms that the editor's diagnostics, hover, code actions, folding and find work as needed under the workspace toolchain.
  - Where a feature is missing, the builder draws it with the editor's range decorations and popovers instead.
- How the tree shows a file with several requests (`requests:` lists).
- The run-history storage format and location under Application Support.
- How the `curl-runner://` scheme is registered for local builds.
- Keyboard shortcuts not shown in the mockups.
- What Postman and OpenAPI import do in v1, if anything beyond "Coming soon".
