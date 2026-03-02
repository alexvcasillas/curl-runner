---
"@curl-runner/cli": patch
---

Fix retry mechanism not triggering on HTTP 429/5xx status codes. Adds Retry-After header support, configurable retryable status codes, and proper CLI defaults propagation.
