# CommandPalette

The ⌘K palette and every menu share one item shape: 28px row, `radius-md`, `surface-300` for the active item.

- Group results under uppercase `caption-11` labels: Requests, Collections, Environments, Actions.
- Fuzzy matches are shown with bold, underlined characters rather than a colour.
- Shortcuts sit at the right in a `Kbd`; secondary context (folder name) in `text-secondary`.
- The palette is 560px wide, on `surface-raised` with `shadow-dialog`, over the `overlay` scrim, and always has the keyboard legend in its footer.
- GPUI-kit: `PopupMenu` for menus; the palette is a custom `List` inside a `Modal`.
