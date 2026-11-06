---
"@curl-runner/cli": patch
---

**Clearer errors for suspended accounts, teams and links**

When a Cloud admin suspends an account or a team, or turns off a share link, the CLI now says so.

- **Improved:** `login` says your account is suspended, so you know to contact support instead of trying again.
- **Improved:** `publish` to a suspended team says the team is suspended and exits with `2`, like other access problems.
- **Improved:** `pull` of a link that no longer works says it may have been turned off by an admin, as well as reset or expired.
