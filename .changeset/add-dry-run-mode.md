---
"@curl-runner/cli": minor
---

Add dry-run mode to preview curl commands without executing them

- New `--dry-run` / `-n` flag to show curl commands without making actual API calls
- Status displays as "DRY-RUN" in cyan for all output formats
- JSON output includes `dryRun: true` field instead of status code
- Environment variable support: `CURL_RUNNER_DRY_RUN=true`
- Useful for debugging configurations and validating request setups
