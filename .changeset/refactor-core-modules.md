---
"@curl-runner/cli": patch
---

Refactor: modularize core functionality for better testability and maintainability

- Extract config utilities to `core/config` (CLI parser, env loader)
- Extract curl utilities to `core/curl` (args builder, response parser)
- Extract interpolation to `core/interpolator` (variable resolution)
- Extract retry logic to `core/execution` (backoff, retry strategies)
- Extract validation to `core/validation` (response validation, body diffing)
- Extract formatting to `core/format` (duration, size, JSON, colors)
- Add 389 new unit tests for core modules
