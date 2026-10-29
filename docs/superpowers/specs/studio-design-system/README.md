curl-runner studio is a desktop app for writing, running and sharing collections of HTTP requests defined in YAML. It should feel like a calm, dense developer dashboard: dark by default, thin hairlines, one bright button, and color only when something has a state.

## Principles

- **Dense and calm.** Fit a whole collection, its YAML and its last result on one screen. Use `body-13` as the base size, 28px rows (`row-nav`) and 18px editor lines (`row-editor`). Separate areas with `border-100` hairlines, not boxes or shadows.
- **One primary button per view.** `primary` on `on-primary` is the brightest thing on screen and marks the one action the view is for (Run, Share, Import). Everything else is secondary (`border-200` outline) or ghost.
- **Color means state.** Neutrals carry the interface. `success`, `warning`, `danger` and `accent-blue` appear only for pass/fail, validation, running and links/focus. Every state color is paired with an icon or a word, never hue alone.
- **Mono for what gets sent.** Anything the user types into a request or receives from a server is set in Geist Mono: YAML, URLs, headers, bodies, variable names, method badges. Interface text is Geist.
- **Secrets stay local.** Every place that shows a credential says where it lives (the OS keychain) and that it is never synced or shared.

## Color

- Ground: `bg-100` for every column, the title bar and the status bar. `surface-100` is the editor's current line and row hover; `surface-200` is the selected row; `surface-300` fills chips, kbd keys and counters.
- Floating things (popovers, menus, palette, dialogs, toasts) sit on `surface-raised` with `border-200` and a shadow token; nothing else gets a shadow.
- Text: `text-primary` for content, `text-secondary` for labels and metadata, `text-tertiary` for hints and codes, `text-gutter` for line numbers, `text-disabled` only for disabled controls.
- Status fills: callouts and pills use the matching `*-subtle` fill and `*-border` with the status color for the icon or label.
- Editor: syntax tokens alias the palette: `syntax-key` = text, `syntax-string` = green, `syntax-number` = purple, `syntax-variable` = amber, `syntax-comment` = tertiary. Diagnostics tint the whole line with `diag-error-line` or `diag-warning-line`.
- HTTP methods: `method-get` blue, `method-post` green, `method-put` amber, `method-patch` purple, `method-delete` red, `method-other` neutral.
- Dark is the primary theme. Light mirrors it with Geist's light neutrals and darker status hues so every text pair stays at 4.5:1 or better.
- Known gap: `border-200` control outlines are below 3:1 against `bg-100` in both themes, on purpose, to keep the dashboard restraint. Controls always have a visible label, and keyboard focus uses `focus-ring`, which passes 3:1 on every surface.

## Type

- Geist (sans) and Geist Mono, both from Google Fonts.
- Interface: `heading-24` page titles, `heading-20` run and empty-state titles, `heading-16` dialog titles, `heading-14` panel headings, `body-13` default, `label-13` buttons and selected tabs, `small-12` metadata and callouts, `caption-11` counters and uppercase section labels.
- Code: `code-13` in the editor, `code-12` for responses, headers and inline code, `method-10` for method badges.
- Sentence case everywhere. Uppercase only for `caption-11` group labels, with 0.02em tracking.
- Use tabular figures for durations, sizes and counts.

## Layout

- Window: title bar `titlebar-height` (48px), status bar `statusbar-height` (28px).
- Three columns between them: collections `sidebar-width` (264px, resizable 220–360), editor (flexible, min 560px), inspector `inspector-width` (400px, resizable 320–560). Columns are separated by `border-100` hairlines.
- Controls: `control-md` (32px) by default, `control-sm` (24px) inline. File tabs `tab-height` (40px), panel tabs 34px.
- Spacing on a 4px scale: `space-2` inside rows, `space-3` button padding, `space-4` panel padding, `space-6` between sections.
- Radius: `radius-sm` badges and kbd, `radius-md` buttons, inputs and rows, `radius-lg` floating surfaces, `radius-xl` dialogs, `radius-full` pills and dots.

## States and motion

- Hover lightens the ground one step (`surface-100` or `surface-200`); selected uses `surface-200` plus `text-primary`.
- Focus is always `focus-ring` (1px `bg-100` gap, then 2px `accent-blue`), on every focusable element.
- Disabled controls use `surface-300` and `text-disabled` and explain themselves in a tooltip.
- Motion is short and functional: 120ms for hover and press, 160ms for popovers and dialogs (fade + 4px rise). Spinners only while a request is in flight. Respect reduced-motion settings.

## Iconography

- Line icons on a 16px grid with a 1.5px stroke, round caps and joins, drawn in `currentColor`. 13px inside dense rows, callouts and status labels.
- Core set: play (run), plus, chevron, search, folder, check, cross, triangle-! (warning), circle-! (error), circle-i (info), lock (read only), spinner.
- No emoji, no filled illustration. There is no logo yet: set "curl-runner studio" in Geist 600 wherever a mark would go.

## Writing

- Speak to the user as "you"; the app doesn't say "I" or "we".
- Name things the way people see them: collections, requests, environments, variables, checks, runs.
- Buttons are verbs in sentence case ("Run collection", "Copy link", "Keep mine").
- Errors say what is wrong and how to fix it in one sentence: "`FETCH` isn't a supported method. Use GET, POST, PUT, PATCH, DELETE, HEAD or OPTIONS."
- Use real HTTP language verbatim: "201 Created", "404 Not Found".
- Placeholder content in designs uses Acme, Payments API and api.example.com.

## Building it in GPUI-kit

The studio is built in Rust with GPUI-kit. Map these tokens onto its theme: `bg-100` → background, `surface-raised` → popover, `border-200` → border, `text-primary`/`text-secondary` → foreground/muted foreground, `primary`/`on-primary` → primary/primary foreground, `accent-blue` → ring and link, and the status colors → its success, warning, danger and info. Use its `Button`, `Input`, `Select`, `Switch`, `Checkbox`, `TabBar`, `Tree`/`List`, `Table`, `Popover`, `Tooltip`, `Modal` and `Notification` with the variants each component page names. The YAML editor is its code editor with line numbers, folding and search on, fed curl-runner's validator output as diagnostics.
