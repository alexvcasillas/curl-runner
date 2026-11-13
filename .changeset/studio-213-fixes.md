---
"@curl-runner/cli": patch
---

**Studio fixes after 2.13.0**

- **Fixed:** `curl-runner.yaml` is checked as a config file: only YAML syntax errors show, not "Missing request(s)".
- **Fixed:** adding a shared collection from a link (the website's Open in curl-runner studio) closes the dialog. Before, it could stay open for good with Close doing nothing, and each retry added another copy.
- **Fixed:** a one-line toast's icon lines up with its text.
