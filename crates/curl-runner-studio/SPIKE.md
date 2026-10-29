# GPUI-kit spike (gpui-kit 0.8.1)

Findings from a throwaway window (YAML editor, one diagnostic, one hover, one
code action), run on macOS with toolchain 1.99.1. Every item below was built
and seen working; nothing needed the fallback.

## Dependencies

- `gpui-kit = { version = "=0.8.1", features = ["tree-sitter-yaml"] }`. Default
  features (`component`, `assets`) stay on. The YAML feature is named
  `tree-sitter-yaml` and enables `gpui-component/tree-sitter-yaml`.
- gpui-kit 0.8.1 pins `gpui-pre =0.3.10` (and `gpui-pre-platform`,
  `gpui-pre-macros` at the same version). Our `Cargo.toml` pins `gpui-pre`
  to the same version so an upgrade has to move both.
- GPUI is used through the Kit only: `use gpui_kit::*;` is GPUI (`App`,
  `Window`, `Entity`, `Task`, `Result`, `div`, `px`, `Rgba`...). Do not add a
  dependency renamed to `gpui`: gpui-kit re-exports itself as `gpui`, and a
  second `gpui` makes glob imports ambiguous (E0659).
- Add `lsp-types = "0.97.0"` (feature `proposed`, macOS-only) when wiring providers: the
  provider traits take and return `lsp_types` values and the Kit does not
  re-export the crate (only `lsp_types::Position`).
- `gpui_kit::component` = `gpui-component`; the editor engine lives in
  `gpui-base` and is re-exported through `gpui_kit::component::input`.

## App and window

```rust
use gpui_kit::*;
application().run(|cx| {
    gpui_kit::init(cx); // gpui-component + gpui-base, key bindings included
    let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
    let opts = WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() };
    gpui_kit::open_window(opts, cx, |window, cx| cx.new(|cx| View::new(window, cx)))?;
    cx.activate(true);
});
```

`gpui_kit::open_window` wraps the view in `gpui_kit::base::Root` (needed for
popovers and menus). The component theme starts in light mode
(`Theme::change(ThemeMode::Light, ..)` in `init`); switch with
`gpui_kit::component::Theme::change(ThemeMode::Dark, None, cx)` and override
colors on `Theme::global_mut(cx)`. The editor paints its own background from
the theme, so it showed white over our `bg-100` until the theme is mapped to
our tokens (Task 7).

## 1. YAML code editor: line numbers, folding, find bar

- State: `gpui_kit::component::input::EditorState` (alias of
  `InputBaseState<EditorMode>`), an entity:
  `cx.new(|cx| EditorState::new(window, cx).language("yaml").line_number(true).folding(true))`.
  `EditorState::new` already turns on line numbers, indent guides, tab size 2
  and search; `folding` defaults to true. Runtime setters:
  `set_line_number`, `set_folding`, `set_searchable`, `set_highlighter(lang, cx)`.
- Content: `state.set_value(text, window, cx)` (no undo entry, no
  `InputEvent::Change`), `state.text() -> &Rope`, `state.value() -> SharedString`.
- Element: `gpui_kit::component::input::Editor::new(&state)` with `.h_full()`,
  `.bordered(false)`, `.readonly(..)`.
- YAML highlighting (tree-sitter) worked: strings, numbers, keys in distinct
  colors. Syntax colors come from `Theme::highlight_theme`.
- Find bar: bound to `cmd-f` (`Search` action) in the `Input` key context;
  programmatic: `state.open_search(replace_mode, cx)`, `close_search`,
  `set_search_query`, `next_search_match`. It renders above the text.
- Changes: subscribe to `InputEvent::Change` (`cx.subscribe(&state, ..)`).

## 2. Diagnostics (severity, range, message)

- Types: `gpui_kit::component::highlighter::{Diagnostic, DiagnosticSeverity, DiagnosticSet}`.
  In 0.8.1 these are re-exported from `highlighter` only; `component::input`
  does not re-export them (checked in `gpui-component-0.8.1/src/input/mod.rs`).
- `Diagnostic::new(start..end, message).with_severity(DiagnosticSeverity::Warning).with_code("unknown-property").with_source("curl-runner")`;
  `range: Range<Position>`, `severity` is `Error | Warning | Info | Hint`.
- Set: `state.diagnostics_mut()` returns `Option<&mut DiagnosticSet>` (Some for
  the editor) with `push`, `extend`, `clear`; then `cx.notify()`.
- Rendered as a wavy underline in the severity color (seen). The editor also
  has a built-in diagnostic popover (`present_diagnostic`, shown on hover over
  the range); not checked by hand in the spike.
- **The set is reset on every text change.** The host must re-lint and
  re-push after each `InputEvent::Change`.
- Whole-line tints (`diag-error-line`, `diag-warning-line`) are not built in:
  use range decorations, `state.create_range_decorations_collection(vec![RangeDecoration::new(line_range).with_style(RangeDecorationStyle::Fill).with_color(hsla)], cx)`,
  which returns a `RangeDecorationCollection` with `set`, `clear`, `dispose`.
  Ranges are UTF-8 byte offsets and follow edits.

## 3. Hover provider

```rust
impl HoverProvider for Hovers {
    fn hover(&self, text: &Rope, offset: usize, window: &mut Window, cx: &mut App)
        -> Task<Result<Option<lsp_types::Hover>>>;
}
state.lsp_mut().hover_provider = Some(Rc::new(Hovers));
```

`offset` is a byte offset. Called after a 150 ms pointer rest; the returned
contents showed in a popover under the pointer. Whether markdown is styled
was not settled by the spike (Task 8 should check bold/code spans).

## 4. Code-action provider that applies a text edit

```rust
impl CodeActionProvider for Fixes {
    fn id(&self) -> SharedString;
    fn code_actions(&self, state: Entity<EditorState>, range: Range<usize>, window, cx: &mut App)
        -> Task<Result<Vec<lsp_types::CodeAction>>>;
    fn perform_code_action(&self, state: Entity<EditorState>, action: CodeAction,
        push_to_history: bool, window: &mut Window, cx: &mut App) -> Task<Result<()>>;
}
state.lsp_mut().code_action_providers = vec![Rc::new(Fixes)];
```

- The menu opens on `cmd-.` (`ToggleCodeActions`) with the selection's byte
  range; choosing an item calls `perform_code_action`.
- Carry the edit in `CodeAction.edit: Some(WorkspaceEdit { changes: Some({uri: vec![TextEdit]}) })`
  (any URI) and apply with `state.apply_lsp_edits(&edits, window, cx)`.
- **`perform_code_action` runs while the editor entity is being updated.**
  Calling `state.update(..)` directly panics ("already being updated").
  Defer it: `window.defer(cx, move |window, cx| state.update(cx, |s, cx| s.apply_lsp_edits(&edits, window, cx)))`.
- `apply_lsp_edits` applies edits one by one in order, so several edits must
  be sorted from last to first (or be non-overlapping and already adjusted).
  It uses `replace_text_in_range_silent`; Task 8 should confirm undo and
  `InputEvent::Change` behavior.

## 5. Byte offset to editor position

`gpui_kit::component::input::RopeExt` on `state.text()`:
`offset_to_position(byte) -> Position` and `position_to_offset(&Position) -> usize`.
`Position` is `lsp_types::Position` (`gpui_kit::component::input::Position`,
`Position::new(line, character)`, both zero-based). **`character` counts
chars (Unicode scalar values), not UTF-16 units**, despite the LSP type.
Checked: byte 30 of the sample is `Position { line: 2, character: 4 }`.
Diagnostics and `TextEdit` ranges use this same `Position`.

## 6. Loading Geist and Geist Mono

Neither is on macOS by default, and GPUI panics on the first text laid out in
a family it cannot find. Embed the font files and register them before any
window opens:

```rust
cx.text_system().add_fonts(vec![
    Cow::Borrowed(include_bytes!("../assets/fonts/Geist-Regular.ttf")),
    Cow::Borrowed(include_bytes!("../assets/fonts/GeistMono-Regular.ttf")),
    // one entry per weight used (400, 500, 600)
])?;
let theme = gpui_kit::component::Theme::global_mut(cx);
theme.font_family = "Geist".into();
theme.mono_font_family = "Geist Mono".into();
```

`TextSystem::add_fonts(Vec<Cow<'static, [u8]>>) -> Result<()>`. The theme
uses an explicitly set family as-is (no fallback probe), so the bytes must be
registered first. Font files are not vendored yet (license: SIL OFL, from the
Geist repository); system families (`.SystemUIFont`, `Menlo`) work without
embedding.

## Import paths

- `gpui_kit::component::input::{EditorState, Editor, InputState, Input, InputEvent,
  HoverProvider, CodeActionProvider, Rope, RopeExt, Position, RangeDecoration,
  RangeDecorationStyle, RangeDecorationCollection}`.
- `gpui_kit::component::highlighter::{Diagnostic, DiagnosticSeverity, DiagnosticSet}`.
- `gpui_kit::component::{Theme, ThemeConfig, TitleBar, WindowExt, Icon, IconName}`;
  dialogs `component::dialog::{Dialog, DialogFooter, AlertDialog}`, menus
  `component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem}`.
- `use gpui_kit::prelude::*` for `FluentBuilder` (`when`, `map`).
- Test modules must not glob `gpui_kit::*`: with `test-support` it brings
  GPUI's `test` macro, which shadows `#[test]` (recursion-limit error).

## Theme and fonts (Task 7)

- Theme: build a `ThemeConfig` (JSON: `colors`, `highlight` with
  `editor.*` and `syntax`) from our tokens and install it with
  `Theme::update(cx, |t| t.apply_config(&Rc::new(config)))`; that also
  switches to dark and repaints the editor background.
- Fonts: `cx.text_system().all_font_names()` tells whether Geist / Geist Mono
  are installed; otherwise `.SystemUIFont` and `Menlo`.
- `Editor::new(&state).text_size(px(13.)).line_height(px(18.))` overrides the
  editor's default 1.5 line height.

## Test support

`gpui-kit` `test-support` (dev-dependency) resolves the weak optional
`gpui-fast*` crates into `Cargo.lock` (not compiled). That needed
`taffy 0.15.0` and `unicode-properties 0.1.3` (from 0.15.1 / 0.1.4).
`#[gpui_kit::test]` plus `gpui_kit::test::TestWindowExt`: `.role(..)` then
`.test_support()` on an element makes it queryable with
`window.within(id).find_all_by_role(role)`.

## Fallbacks

None needed: all six items exist in 0.8.1. If a later version drops one, the
fallback is range decorations (`RangeDecoration` fill/frame) for marks plus a
`gpui_kit::component::popover` anchored at the range for messages.
