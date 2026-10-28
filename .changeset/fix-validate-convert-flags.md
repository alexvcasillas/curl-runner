---
"@curl-runner/cli": patch
---

**Validator, convert and flag fixes**

- `validate` accepts every key the config model supports, such as `retry.retryableStatuses`, `profile.histogram`, snapshot and diff fields, `global.env` and `security` (#136)
- Numeric request fields (`timeout`, `retry.count`, `retry.delay`, `maxRedirects` and others) accept `${VAR}` and `${VAR:default}` in `validate`; global settings are not interpolated by the runner, so `validate` still reports `${...}` there (#137)
- `validate` now warns about keys the runner ignores, such as `profile.percentiles`, snapshot and diff `ignoreFields` and `ignoreHeaders`, and `diff.baselineUrl`
- `convert` writes JSON bodies as plain `body` values, quotes YAML safely and keeps curl's form content type (#138)
- `convert` maps `--retry` and `--retry-delay` to a matching `retry` block (#139)
- `global.http2` now applies to every request (#140)
- `-c` works as the short form of `--continue-on-error` (#141)
