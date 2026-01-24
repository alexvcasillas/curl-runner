---
"@curl-runner/cli": minor
---

Add maxConcurrency option for parallel execution

- New `maxConcurrency` setting in global config to limit simultaneous requests in parallel mode
- New `--max-concurrent <n>` CLI flag
- New `CURL_RUNNER_MAX_CONCURRENCY` environment variable support

This helps avoid rate limiting from APIs and prevents overwhelming target servers when running many requests in parallel.
