# Input

Single-line text inputs and selects, 32px tall (`control-md`), `radius-md`, `border-200` outline on `bg-100`.

- Always pair with a visible label above (`small-12`, `text-secondary`). Placeholders are examples, never labels.
- Use `cr-input-mono` (Geist Mono 12px) for anything the user types that gets sent: URLs, headers, tokens, variable names.
- States: focus = `accent-blue` border + `focus-ring`; error = `danger` border plus a hint below that says how to fix it; disabled = `surface-100` fill and `text-disabled`.
- Selects use the same box with a 16px chevron icon on the right; native menus are fine.
- Secrets are masked by default with a reveal button and come from the OS keychain.
- GPUI-kit: `Input` / `InputState`, `Select`; code-heavy fields can use `InputState::code_editor` in single-line mode.
