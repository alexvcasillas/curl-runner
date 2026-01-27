# Feature Ideas for curl-runner

## Existing capabilities (from repo docs)
- YAML config, collections, variables + response store
- Conditional execution (when), retries, snapshots, response diffing
- Performance profiling + watch mode
- Auth: basic/bearer, SSL/mTLS support
- Output: pretty/json/raw + metrics

## Feature ideas (high leverage)
- OpenAPI import to YAML suites
- Postman/Insomnia collection import
- `.env` + per-env overrides
- Secret masking in output + redaction rules
- Data-driven runs (CSV/JSON matrix paramization)
- Assertions: JSONPath, JSON Schema, regex on headers/body
- Hooks: pre/post scripts (Bun TS) with safe sandbox
- Global rate limit/throttle (req/s) + adaptive backoff
- Baseline compare across envs -> Markdown/HTML report
- JUnit/CI report output
- Cache/etag support (If-None-Match/If-Modified-Since)
- Auth: OAuth2 client creds + refresh token flow
- Plugin system for custom validators/formatters
- CLI filters: `--tag`/`--only` on requests
- Docs: interactive YAML playground + share links

## Plan
- Confirm target users + top pain points
- Score 5-8 candidates by impact/effort/risk
- Draft MVP specs (CLI flags + YAML schema)
- Build roadmap: quick wins vs big bets

## Unresolved questions
- Primary target: CI pipelines or local dev?
- Prefer CLI features, docs features, or both?
- Top 3 pain points today?
