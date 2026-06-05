---
"@curl-runner/cli": minor
---

Add `$absent` and `$exists` body-assertion keywords for response validation. `$absent` asserts a key is not present in the response (fails even when the value is `null`); `$exists` asserts a key is present with any value (including `null`), stricter than `*`.
