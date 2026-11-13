---
"@curl-runner/cli": patch
---

**Publishing from a git folder records where it came from**

The studio sends the folder's git details with each version, as CI does, so Cloud can say where a collection comes from.

- **Improved:** a publish from a git repository carries its commit, branch and repository. The sidebar foot and the dashboard show them (`main@4f2a9c1`).
- **Improved:** uncommitted changes leave the commit out, and credentials in the remote URL are never sent.
- **Improved:** with a commit to show, the sidebar foot reads "2 hours ago · main@4f2a9c1", without who published it.
