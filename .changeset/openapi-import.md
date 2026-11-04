---
"@curl-runner/cli": minor
---

`curl-runner convert openapi <spec.json|spec.yaml>` turns an OpenAPI 3.0, 3.1 or Swagger 2.0 document into YAML files and a `.env`. You get one file per operation, in a folder per tag, with sample values from the spec's examples and schemas and a check on the documented success status. Optional parameters come in commented out. Variables are prefixed per API, so two APIs can share one `.env`. Secrets never land in a file: their names are listed in `.env` and you set the values in `.env.local`. The command prints what it skipped or needs from you.
