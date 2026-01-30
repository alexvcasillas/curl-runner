---
"@curl-runner/cli": minor
---

Add HTTP/2 support with multiplexing

- New `--http2` flag to use HTTP/2 protocol for all requests
- Per-request `http2: true` option in YAML config
- Environment variable support: `CURL_RUNNER_HTTP2=true`
- Enables HTTP/2 multiplexing for improved performance
