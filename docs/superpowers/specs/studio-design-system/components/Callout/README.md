# Callout

Inline messages inside a panel or the editor: info, warning or error, each with its icon.

- Box: `radius-md`, 10px × 12px padding, `small-12` text. Fill and border from the matching `*-subtle` and `*-border` tokens; the icon takes `accent-blue`, `warning` or `danger`; the text stays `text-primary`.
- Say what happened and how to fix it in one sentence; put the fix in a small button at the right ("Fix", "Next error").
- Use a banner (same component, full panel width) for state that blocks the whole view, like a run blocked by errors.
