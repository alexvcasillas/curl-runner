# StatusPill

Pills and dots show the state of a request, check or run.

- **Pill** (`radius-full`, 20px): HTTP status in the inspector header and run tables. 2xx = `success` on `success-subtle`, 4xx/5xx = `danger` on `danger-subtle`, retries = `warning`, running = `accent-blue` with a spinner, not run = neutral.
- **Dot** (6px): last-run state in the tree and sync state in the status bar. A dot never stands alone where the state matters: pair it with a label, tooltip or `aria-label`.
- Use the status text verbatim ("201 Created"), not a paraphrase.
