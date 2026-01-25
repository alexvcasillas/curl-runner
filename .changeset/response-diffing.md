---
"@curl-runner/cli": minor
---

feat: add response diffing for environment/run comparison

- Compare API responses between environments (staging/prod)
- Detect API drift between runs over time
- Exclude dynamic paths (timestamps, IDs)
- Multiple output formats: terminal, JSON, markdown
- CI-friendly exit codes
- Diff subcommand for offline baseline comparison
