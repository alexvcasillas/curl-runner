---
"@curl-runner/cli": minor
---

Add snapshot testing for API regression testing

- New `-s, --snapshot` flag to enable snapshot testing
- New `-u, --update-snapshots` flag to update all snapshots
- New `--update-failing` flag to update only failing snapshots
- New `--snapshot-dir <dir>` option for custom snapshot directory
- New `--ci-snapshot` flag to fail on missing snapshots (CI mode)
- Support via environment variables: `CURL_RUNNER_SNAPSHOT`, `CURL_RUNNER_SNAPSHOT_UPDATE`, etc.
- Support via YAML config: `snapshot: true` or detailed config with `include`, `exclude`, `match`
- Exclusion patterns: exact paths, wildcards (`*.timestamp`), array wildcards (`body[*].id`)
- Match rules: wildcards (`*`) and regex patterns (`regex:^v\d+`)
- Snapshots stored in `__snapshots__/<filename>.snap.json`
