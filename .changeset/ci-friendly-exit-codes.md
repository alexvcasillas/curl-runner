---
"@curl-runner/cli": minor
---

Add CI-friendly exit codes for pipeline integration

New CLI options for controlling exit code behavior in CI/CD pipelines:

- `--strict-exit`: Exit with code 1 if any validation fails, regardless of `--continue-on-error`
- `--fail-on <count>`: Exit with code 1 only if failures exceed the specified threshold
- `--fail-on-percentage <pct>`: Exit with code 1 only if failure percentage exceeds the threshold

New environment variables:

- `CURL_RUNNER_STRICT_EXIT`: Enable strict exit mode
- `CURL_RUNNER_FAIL_ON`: Set maximum allowed failures
- `CURL_RUNNER_FAIL_ON_PERCENTAGE`: Set maximum allowed failure percentage

This enables cleaner CI/CD integration across platforms (GitHub Actions, GitLab CI, CircleCI, Jenkins) without requiring additional parsing logic to detect validation failures.
