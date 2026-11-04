---
"@curl-runner/cli": minor
---

**Cloud publishing from the CLI**

Publish a collection to curl-runner Cloud from CI, and pull a shared one onto your machine. Cloud is in private preview.

- **New:** `curl-runner publish` sends a folder to Cloud as a new version. It checks every file first, and `--dry-run` shows what would change.
- **New:** `curl-runner pull <link>` downloads a shared collection, and `curl-runner pull <dir>` updates it.
- **New:** `curl-runner login` and `curl-runner logout` sign in with an emailed code and sign out.
- **New:** a GitHub Action, `alexvcasillas/curl-runner/actions/publish`, installs the CLI and publishes on every push.
