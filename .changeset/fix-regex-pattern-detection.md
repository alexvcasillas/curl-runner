---
"@curl-runner/cli": patch
---

fix: remove loose char checks from isRegexPattern to prevent false positives on plain strings like "C++", "really?", "*-", "[]"
