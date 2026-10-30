# Button

Buttons start actions; one **primary** button per view, for the thing the view is for (Run in the workspace, Share in the share dialog, Import in the import dialog).

- Variants: `cr-btn` (secondary, the default), `cr-btn-primary`, `cr-btn-ghost` (toolbars, dismiss), `cr-btn-danger` (destructive confirms only).
- Sizes: default `control-md` (32px) and `cr-btn-sm` (24px) for inline actions in callouts, popovers and problem rows. Icon-only buttons are square and need an `aria-label`.
- Labels are a verb in sentence case: "Run collection", "Copy link", "Keep mine". No trailing punctuation; an ellipsis only when another step follows ("Share collection…").
- Show the shortcut with a `Kbd` inside the button when one exists.
- Disabled buttons say why in a tooltip ("Fix 2 errors to run"); prefer explaining nearby over silently disabling.
- Focus: `focus-ring`. GPUI-kit: `Button` with `.primary()`, `.ghost()`, `.danger()`, `.small()`.
