---
"@curl-runner/cli": patch
---

fix: spawn curl directly instead of shell execution

Removes shell interpretation of ${...} variables in curl commands. Previously, unresolved variables would cause errors on macOS bash 3.2 and Debian dash. Now spawns curl directly with args array, uses --form-string for text form fields to prevent @/< interpretation.
