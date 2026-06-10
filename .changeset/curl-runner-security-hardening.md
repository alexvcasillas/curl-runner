---
"@curl-runner/cli": minor
---

Security hardening for curl execution, SSRF/file-access, and secret handling.

This release closes several attack vectors that arise when curl-runner processes
untrusted server responses (via `store` → `${store.x}` chaining) or shared YAML
configs. Secure defaults are enabled out of the box, each with an explicit opt-out.

**Hardening**

- **curl argument injection**: a `--` end-of-options separator is now emitted before
  the request URL, so a URL beginning with `-` can no longer be interpreted as a curl
  flag. Raw request bodies are sent with `--data-raw` instead of `-d`, preventing
  `body: "@/etc/passwd"` from reading local files.
- **URL protocol allow-list (SSRF / local file disclosure)**: only `http`/`https` are
  permitted by default. `file://`, `gopher://`, `ftp://`, etc. are rejected before
  curl runs, and `--proto`/`--proto-redir` prevent redirects from escaping into other
  protocols. Opt in with `--allow-protocol <proto>` or `CURL_RUNNER_ALLOWED_PROTOCOLS`.
- **Filesystem path confinement**: `output`, `saveToFile`, and `formData` file paths
  are confined to the working directory by default. Opt out with `--allow-path` or
  `CURL_RUNNER_ALLOW_PATHS`. (SSL `ca`/`cert`/`key` paths are exempt — they are TLS
  handshake material and are never transmitted to the server.)
- **Secret redaction**: inline credentials (`auth.token`, `auth.password`, and
  sensitive request headers) are now auto-registered for redaction, the saved results
  summary is redacted before being written to disk, and detection patterns were
  extended (JWT, Google API keys, `api_key=`/`token=`/`secret=` assignments).
- **ReDoS & prototype pollution**: user/server-controlled regex matching now caps input
  length, and `__proto__`/`prototype`/`constructor` keys are rejected during variable
  interpolation and response-path traversal.

**Potentially breaking**: configs that rely on non-`http(s)` URLs, or that read/write
files outside the working directory, must now opt in via the flags above.
