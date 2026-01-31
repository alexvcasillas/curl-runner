---
"@curl-runner/cli": minor
---

feat: add validate command for YAML config validation

New `curl-runner validate` command that:
- Accepts file paths or glob patterns
- Validates YAML syntax and structure
- Checks curl-runner API correctness (methods, auth types, operators, etc.)
- Validates curl options (SSL, retry, timeout configs)
- Reports issues with severity (error/warning)
- Proposes fixes for common issues
- Auto-fixes with `--fix` flag (uppercase methods, https:// prefix)
