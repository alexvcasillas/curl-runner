---
"@curl-runner/cli": patch
---

Fix array body validation to use element-wise matching when both expected and actual values are arrays, instead of treating all arrays as "one of" enum validators
