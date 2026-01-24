---
"@curl-runner/cli": minor
---

Add watch mode for automatic re-execution on file changes

- New `-w, --watch` flag to enable watch mode
- `--watch-debounce <ms>` option to customize debounce delay (default: 300ms)
- `--no-watch-clear` option to disable screen clearing between runs
- Support via environment variables: `CURL_RUNNER_WATCH`, `CURL_RUNNER_WATCH_DEBOUNCE`, `CURL_RUNNER_WATCH_CLEAR`
- Support via YAML config under `global.watch`
