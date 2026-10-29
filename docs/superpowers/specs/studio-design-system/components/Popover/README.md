# Popover

Floating surfaces anchored to something: the diagnostic hover in the editor, menus from buttons, and tooltips.

- Popover: 340px, `surface-raised`, `border-200`, `radius-lg`, `shadow-popover`. Body uses `small-12`; actions sit in a footer row separated by a hairline.
- The diagnostic popover shows icon, message, and a `caption-11` source line (`curl-runner · <code> · <path>`), then the quick fix (with ⌘.) and a docs link.
- Tooltip: one line, `radius-sm`, appears after 500ms hover; used to explain disabled controls and truncated names.
- GPUI-kit: `Popover`, `Tooltip`; the editor's hover provider renders the diagnostic popover.
