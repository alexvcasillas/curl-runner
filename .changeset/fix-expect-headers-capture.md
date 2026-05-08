---
"@curl-runner/cli": patch
---

fix(cli): capture response headers via `-D -` so `expect.headers` validation works; expose `headerHistory` and multi-value header support
