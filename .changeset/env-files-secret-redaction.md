---
"@curl-runner/cli": minor
---

feat: add .env file support with secret redaction

- Load variables from .env, .env.local, .env.{env}, .env.{env}.local
- `--env` / `-e` flag to select environment
- `SECRET_*` prefixed variables automatically redacted in output
- Pattern-based detection for common API keys (Stripe, AWS, GitHub, NPM, Slack, Paddle, OpenAI, Anthropic)
- `--no-redact` flag to disable redaction
- Config option `env.environment` and `env.redactSecrets`
