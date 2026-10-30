# TreeRow

One row of the collections tree: folder or request, 28px (`row-nav`), `radius-md`.

- Folder rows: chevron, name, request count in `text-tertiary`. Request rows: 16px indent per level, `MethodBadge`, name, `StatusDot` for the last run.
- Selected = `surface-200` + `text-primary`; hover = `surface-100`.
- Shared collections you can't edit show a lock and "Read only".
- Names truncate with an ellipsis; the full path shows in a tooltip.
- GPUI-kit: `Tree` / `List` with custom `ListItem` rendering.
