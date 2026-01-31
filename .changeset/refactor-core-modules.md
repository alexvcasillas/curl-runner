---
"@curl-runner/cli": patch
---

Refactor: modularize core functionality for better testability and maintainability

- Extract config utilities to `core/config` (CLI parser, env loader)
- Extract curl utilities to `core/curl` (args builder, response parser, body parser)
- Extract interpolation to `core/interpolator` (variable resolution)
- Extract retry logic to `core/execution` (backoff, retry strategies, post-processor)
- Extract validation to `core/validation` (response validation, body diffing)
- Extract formatting to `core/format` (duration, size, JSON, colors, tree renderer)
- Split types/config.ts into domain-specific modules (json, request, execution, snapshot, diff, profile, watch, global)
- Add 400+ new unit tests for core modules
