# Dialog

Modal dialogs for focused tasks: new request, share, import, open shared link, sync conflicts.

- 480px (forms) or 640–720px (diffs, import previews); `surface-raised`, `radius-xl`, `shadow-dialog`, over the `overlay` scrim.
- Head: `heading-16` title that names the task, one line of `text-secondary` context. Body: 20px padding, 16px between fields. Footer on `bg-100`: shortcut hint left, ghost Cancel and one primary button right.
- The primary label repeats the task ("Create request", "Share", "Keep mine"). Esc and Cancel close without changes.
- GPUI-kit: `Modal` / `Dialog`.
