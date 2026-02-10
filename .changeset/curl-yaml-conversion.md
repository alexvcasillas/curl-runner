---
"@curl-runner/cli": minor
---

feat: add bidirectional curl ⇄ YAML conversion engine

- Shell tokenizer with quote/escape/continuation handling
- Curl semantic parser supporting 40+ flags
- Normalization layer with method inference, body detection, auth detection, query param extraction
- YAML serializer with stable key ordering and loss-aware comments
- Curl generator producing canonical, shell-safe commands
- Batch script parser for converting entire shell scripts
- CLI commands: convert curl, convert file, convert yaml
- Debug mode exposing token stream, AST, and IR
