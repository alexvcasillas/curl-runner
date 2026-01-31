---
"@curl-runner/cli": minor
---

feat: add YAML wizard for interactive config creation

- Add `curl-runner init` for quick YAML file creation
- Add `curl-runner init --wizard` for full interactive wizard
- Add `curl-runner edit <file>` to modify existing YAML files
- Support templates: basic-get, basic-post, api-test, file-upload, auth-flow
- Preview YAML before saving with option to run immediately
- Add 51 new tests for wizard module
