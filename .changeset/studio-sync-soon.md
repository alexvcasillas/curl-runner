---
"@curl-runner/cli": patch
---

**Sync now no longer looks like it works when it can't**

Collections don't sync with curl-runner Cloud yet, so the studio says that instead of showing a button that does nothing.

- **Fixed:** on a server that can't sync collections yet, Settings › Accounts & sync reads "Sync is coming soon" and turns Sync now off.
