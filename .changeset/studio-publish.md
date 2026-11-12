---
"@curl-runner/cli": minor
---

**Publish to curl-runner Cloud from the studio**

The studio publishes a collection folder the way `curl-runner publish` does, signed in as you, and shows where it stands at the foot of the sidebar.

- **New:** Publish at the foot of the sidebar and in the palette. The first time, pick the team and the collection's name, suggested from the folder.
- **New:** the same checks as the CLI run on your Mac first: YAML errors and secret values in the files stop the publish, each with a link to the file.
- **New:** optionally saves the collection's name to `curl-runner.yaml`, so CI publishes to the same collection.
- **New:** the sidebar foot shows the published version, who published it and when, and how many files changed since.
- **New:** clear refusals for a collection that syncs from the studio, a plan limit (with Upgrade), and an account without a team.
