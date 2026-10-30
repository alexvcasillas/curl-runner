# Toast

Short-lived confirmation or a background failure, bottom-right, 360px, on `surface-raised` with `shadow-popover`.

- Confirmations name what happened in the past tense ("Link copied", "Collection shared") and leave after 4s.
- Failures stay until dismissed, say what failed and what happens next, and offer one action.
- Never use a toast for validation errors; those belong in the editor.
- GPUI-kit: `Notification`.
