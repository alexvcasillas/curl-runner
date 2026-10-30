# Tabs

Two sizes of underline tabs: **file tabs** (40px strip, 13px) above the editor and **panel tabs** (34px strip, 12px) in the inspector and problems panel.

- The selected tab is `text-primary` with a 1px `primary` underline; the rest are `text-secondary`.
- File tabs lead with the request's `MethodBadge`. A 6px dot shows unsaved changes, a `danger` dot shows errors.
- Counts sit in a `cr-count` chip after the label.
- GPUI-kit: `TabBar` + `Tab` (underline variant).
