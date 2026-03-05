---
"@curl-runner/cli": patch
---

fix: reconfigure CLI logger after config resolution so --quiet flag suppresses output

- Fix URL typo detection false positives on valid https:// URLs
- Show warnings/info for valid files instead of silently swallowing them
- Apply auto-fixes for warnings on valid files
- Include "did you mean" suggestion inline in unknown key messages
- Detect incomplete when conditions (missing left with operator/right present)
- Fix glob file discovery for absolute paths
- Validate JSON body when Content-Type is application/json
