# EditorDiagnostics

How the YAML editor shows validation: squiggles, gutter icons, line tints, ghost text and the Problems list.

- **Errors** (`danger`) block running: unsupported values, wrong types, invalid YAML. **Warnings** (`warning`) don't: unknown properties are ignored at run time.
- On the line: a 1px wavy underline under the exact range, the icon in the gutter's fold column, the line number in the severity colour and a faint `diag-*-line` tint.
- One short ghost message at the end of the line in `text-tertiary` italic; the full message, source code and quick fix live in the hover `Popover`.
- The Problems panel lists every diagnostic with icon, message, code (`text-tertiary` mono) and `line:col`; clicking jumps to it, F8 moves to the next.
- Scrollbar ticks mark each diagnostic in its severity colour.
- GPUI-kit: feed curl-runner's validator output as `highlighter::Diagnostic` (range, severity, code, source, message); quick fixes come from a code action provider.
