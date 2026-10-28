# @curl-runner/cli

## 2.1.0

### Minor Changes

- [#133](https://github.com/alexvcasillas/curl-runner/pull/133) [`b5f26d3`](https://github.com/alexvcasillas/curl-runner/commit/b5f26d392216c7794ab3854b11c6d7c111230e56) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Install with Homebrew**

  Homebrew: install with `brew install alexvcasillas/tap/curl-runner` (prebuilt binaries for macOS and Linux, arm64 and x64). `curl-runner upgrade` now detects Homebrew installs and runs `brew upgrade curl-runner` instead of refusing to upgrade them.

## 2.0.0

### Major Changes

- [#127](https://github.com/alexvcasillas/curl-runner/pull/127) [`bec01c6`](https://github.com/alexvcasillas/curl-runner/commit/bec01c646216e99a5b7618cf75e42da6d072d91a) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Rewritten in Rust, on libcurl**

  curl-runner 2.0 is a rewrite of the CLI in Rust. YAML files, flags, subcommands, environment variables, config discovery and exit codes stay the same, with the bug fixes and behavior changes listed below.

  ### Runtime

  - The CLI is now a native Rust binary with libcurl linked statically (rustls for TLS). No Bun, Node.js or `curl` binary is needed at runtime. HTTPS uses the operating system trust store (on Debian/Ubuntu images keep `ca-certificates` installed) unless `ssl.ca` is set.
  - `curl-runner upgrade` replaces the binary in-process (checksum verified); npm and bun installs still run `npm|bun install -g @curl-runner/cli@latest`.

  ### Install changes

  - `@curl-runner/cli` has no JavaScript and no postinstall. It ships the binary through one optional dependency per platform (`@curl-runner/cli-linux-x64`, `-linux-arm64`, `-darwin-x64`, `-darwin-arm64`, `-win32-x64`), downloaded by npm, bun, pnpm and yarn according to `os`/`cpu`. Installing with `--omit=optional` leaves no binary.
  - The `curl-runner` bin is a POSIX `sh` launcher. On Windows the npm/bun/pnpm/yarn shims therefore need `sh` on `PATH` (Git for Windows, MSYS2 or Cygwin); without `sh`, install with `install.ps1` or run the `.exe` from the platform package directly.
  - Linux binaries are static musl builds.
  - New: `cargo install curl-runner --locked` installs from crates.io (`curl-runner` and its engine `curl-runner-core`); `curl-runner upgrade` detects cargo installs and reruns that command.

  ### Migration: timeouts are now milliseconds

  `timeout` and `--timeout` are documented in milliseconds, but 1.x passed the value to curl as seconds (`timeout: 5000` waited 5000 s). 2.0 honors milliseconds. Multiply any value you wrote for the old behavior by 1000: `--timeout 30` becomes `--timeout 30000`, `timeout: 2` becomes `timeout: 2000`. Values you already wrote in milliseconds, as the docs said, need no change (under 1.x those were read as seconds, so `timeout: 5000` meant about 83 minutes). `connectionPool.keepaliveTime` (seconds), `connectionPool.connectTimeout`, `retry.delay`, `--retry-delay` and `--watch-debounce` (milliseconds) are unchanged.

  ### Fixed

  Timeouts and transport failures:

  - Request `timeout` is applied in milliseconds (see Migration above); `--dry-run` prints `--max-time` in fractional seconds (`5000` is `5`, `1500` is `1.5`).
  - Transport failures (connection refused, DNS failure, timeout, max redirects exceeded) now fail the request and are retried. In 1.x they were reported as success with status 0 and never retried. Errors read `Request timeout after {timeout}ms`, `Connection timeout after {ms}ms` (when the pool's connect timeout is the smaller one still running), or `curl: (N) <libcurl message>`. Exhausted retries after a transport failure report that error; exhausted retriable statuses keep `Operation returned retriable result`.
  - `method: HEAD` no longer waits for a body that never comes.
  - Profile mode (`--profile`): a failed iteration (transport failure) is counted in `failures` and excluded from the timings instead of counting as a successful sample. `failureRate` is measured failures divided by measured iterations (1.x divided by successful samples, which could exceed 100%). `--profile-concurrency 0` runs sequentially (1.x looped forever). A `--profile-export` write error prints `Cannot export profile results: ...` and exits 1 instead of crashing.

  Precedence:

  - Settings now resolve as defaults < `curl-runner.yaml` < test file `global` < environment variables < CLI flags, as documented. In 1.x a test file's `global` beat CLI flags and environment variables.
  - Per request: base < file `global.defaults` < `collection.defaults` < CLI/env-derived defaults < the request itself. `retry` merges field by field (`--retries` and `--retry-delay` no longer wipe each other or the file's other retry fields), and CLI/env output flags beat the file's `global.output`.
  - A `curl-runner.yaml` `global` with a wrongly typed field (for example `maxConcurrency: abc`) is skipped with a warning.

  Results file (`--output`):

  - With several test files, `--output` now holds the aggregate of all files (`total`, `successful`, `failed`, `skipped` summed, `results` concatenated in run order, `duration` the sum of the per-file durations). In 1.x only the last file's summary was saved. The file is also written on Ctrl-C with the completed results, and atomically (temp file, then rename), so an interrupted run never leaves a truncated file.
  - Numbers are written like `JSON.stringify` on the wire and in files (`2`, `100000000000000000000`, not `2.0` or `1e20`).

  Parallel runs and connection pool:

  - Parallel runs keep a sliding window of `maxConcurrency` live requests instead of waiting for the slowest of each chunk. Without a limit (unset, 0 or at least the request count) every request runs and is reported.
  - On a failure without `continueOnError`, a parallel run stops starting new requests and reports every request already started (1.x reported every request of the chunks run so far).
  - With `connectionPool.enabled` in parallel mode, 4xx/5xx statuses are judged only through `expect` (1.x marked them failed before `expect` ran, so `expect.status: 404` or `expect.failure` could never pass), `retry` and `maxConcurrency` apply, results are no longer truncated at the first failure, and `maxStreamsPerHost` (default 10) limits connections per host. Pool options now apply per handle in sequential runs too (1.x ignored them outside parallel mode).

  Validation, interpolation and parsing:

  - `expect.headers` with non-string values (`x-count: 5`, `x-flag: true`) are stringified and compared; 1.x always failed them.
  - Body, store and condition path lookups read own keys (plus array `length` and numeric indices), not JavaScript prototype members (`toString`, `hasOwnProperty`, ...). `${constructor}` and `${toString}` in interpolation no longer resolve.
  - Environment variables with non-numeric, negative (for unsigned fields) or out-of-range numeric values, and CLI numeric flags such as `--timeout abc`, are ignored instead of stored as `NaN` or negative numbers. `--keepalive-time abc` no longer force-enables the pool.
  - `interpolate` failures (a value that no longer fits a typed field, for example `timeout: "${store.name}"`) fail the request with `invalid value at \`timeout\`: ...`instead of running it with the raw value. An explicit`null`in a request replaces the default, like a JS spread.`RANDOM:string:N`with an unrepresentable`N` is left unresolved instead of aborting.
  - Duplicate mapping keys: the last value wins (they were a parse error). YAML scalars resolve like `Bun.YAML.parse` 1.3 (`yes/no/on/off` are booleans, `010` is 10, ...), so `insecure: yes` loads. Integers beyond the i64/u64 range are read as floats. Nesting deeper than 128 levels is a clean parse error.
  - Absolute paths now work for files, directories and globs (`curl-runner /abs/path/test.yaml` used to find nothing).
  - Dry run also checks that formData files exist. URL globbing (`{a,b}`, `[1-3]`) is no longer applied; URLs are sent literally.

  Secrets and snapshots:

  - Redaction: explicit secrets are keyed by value (two tokens sharing their first 8 characters no longer overwrite each other), `api_key=`/`token=`/`secret=` values are replaced by span, and known secrets are also masked in every other printed string (bodies, header values, errors, stored values, names, JSON/raw output), and in snapshot and diff output.
  - Snapshots: parallel requests no longer race on the snapshot file; a snapshot or baseline file that fails to parse is never overwritten (the comparison fails with an error naming the file) so stored snapshots are not lost; write failures are reported as a failed comparison instead of aborting the request. `--diff-compare <label>` saves the run only when the baseline file does not exist (1.x also replaced the file when no request name matched or nothing succeeded).

  Other commands:

  - `validate --fix` no longer writes YAML that fails to read back or changes meaning (empty `{}`/`[]`, strings such as `*/*`, `@x`, `yes`, `-1`); on a file whose root is not one mapping it leaves the file untouched and prints a warning instead of erasing it.
  - `edit` updates only the edited request inside the full document; 1.x rewrote the file with the first request only and dropped everything else (other requests, `global`, `collection`, `store`, `when`, `snapshot`, ...).
  - Wizard: the advanced options can switch `followRedirects`/`insecure`/`http2` back off, status codes must be whole numbers (`200,abc` used to write `NaN`), and `init -o <file>` is the default of quick init's `Save to` prompt. A `number` field in the interactive JSON builder that is not a number is kept as a string with a warning (1.x stored `NaN`). Wizard writes are atomic. Without a terminal the wizard reads one answer per stdin line and treats end of input as a cancel (exit 0) instead of hanging.
  - Watch mode watches each file's directory, so editors that save by rename-over keep working.
  - Install detection: `~/.local/bin` is checked before the npm environment variables; Homebrew `Cellar`, `~/.cargo/bin` and development builds are never self-replaced. `upgrade` verifies the SHA-256 before extracting, leaves the binary untouched on any failure, and a GitHub release tag without `x.y.z` is a failed lookup.

  ### Behavior changes

  Logged deviations that are not bug fixes (full list with reasons in `crates/CHANGES-FROM-TS.md`):

  - Colors are printed unless `NO_COLOR` is set to a non-empty value.
  - Ctrl-C cancels the run, prints the summary of completed requests and `✗ Interrupted`, writes `--output` (and `--profile-export`) and exits 130; a second Ctrl-C exits 130 at once. Watch mode exits 0.
  - Sequential requests share one connection loop, so connections stay alive between requests; DNS/TCP times print as missing for reused connections and profile latencies exclude process start-up.
  - Without the connection pool the curl tool defaults are reproduced (TCP keepalive 60 s, max 50 redirects, `User-Agent: curl/<libcurl version>`). Retry waits are clamped to 24 h.
  - Dry run with `store`: `${store.x}` stays literal in the printed command and no `Stored:` lines are printed (1.x interpolated an empty string).
  - User regexes run on `fancy-regex` (lookaround and backreferences work) with `\d \w \s \b` and `.` translated to JavaScript meanings; JS-only syntax (`\cX`, `[^]`) counts as an invalid pattern, and a pattern that exceeds the backtracking limit counts as no match.
  - The `--output` JSON keeps the same shape but uses a fixed key order. YAML parse error messages of `validate` and `convert yaml` come from a different parser and can read differently (exit codes are unchanged). Tab-indented YAML is rejected on every command.
  - Update notice: runs on a thread, prints after the command, only when stdout is a terminal, skipped when `CI` is set and for `--help`/`--version`/`upgrade`.
  - The protocol allow-list is intersected with the protocols the static libcurl supports (an unsupported entry stays blocked). `.env` values for `${VAR}` come from the process environment first, then the `.env` files chosen by the final `env.environment`.
  - `convert` quirks of the TypeScript implementation are preserved; `convert yaml` YAML parse errors read `YAML Parse error: <parser message>`. Key sorting in `convert` matches ICU `localeCompare` for ASCII and single-accent Latin letters only.

## 1.25.0

### Minor Changes

- [#124](https://github.com/alexvcasillas/curl-runner/pull/124) [`c1ce517`](https://github.com/alexvcasillas/curl-runner/commit/c1ce517195ee50c4298e55441de8225cdcf1a7d0) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **Security hardening**

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

## 1.24.0

### Minor Changes

- [#122](https://github.com/alexvcasillas/curl-runner/pull/122) [`7ea38aa`](https://github.com/alexvcasillas/curl-runner/commit/7ea38aa4e6861f543b23b12cac3a2fa40f2f9cea) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - **`$absent` and `$exists` assertions**

  Add `$absent` and `$exists` body-assertion keywords for response validation. `$absent` asserts a key is not present in the response (fails even when the value is `null`); `$exists` asserts a key is present with any value (including `null`), stricter than `*`.

## 1.23.5

### Patch Changes

- [#119](https://github.com/alexvcasillas/curl-runner/pull/119) [`078487f`](https://github.com/alexvcasillas/curl-runner/commit/078487fcd469be9e1a426a3873d6baee321fc846) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - fix(cli): capture response headers via `-D -` so `expect.headers` validation works; expose `headerHistory` and multi-value header support

## 1.23.4

### Patch Changes

- [#115](https://github.com/alexvcasillas/curl-runner/pull/115) [`388cb1c`](https://github.com/alexvcasillas/curl-runner/commit/388cb1cdc55a9fc300fd0ec9107e9caf344ca2e5) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - fix: reconfigure CLI logger after config resolution so --quiet flag suppresses output

  - Fix URL typo detection false positives on valid https:// URLs
  - Show warnings/info for valid files instead of silently swallowing them
  - Apply auto-fixes for warnings on valid files
  - Include "did you mean" suggestion inline in unknown key messages
  - Detect incomplete when conditions (missing left with operator/right present)
  - Fix glob file discovery for absolute paths
  - Validate JSON body when Content-Type is application/json

## 1.23.3

### Patch Changes

- [#113](https://github.com/alexvcasillas/curl-runner/pull/113) [`b3c4051`](https://github.com/alexvcasillas/curl-runner/commit/b3c4051376c1fe173089fc6def2306473077305c) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Fix retry mechanism not triggering on HTTP 429/5xx status codes. Adds Retry-After header support, configurable retryable status codes, and proper CLI defaults propagation.

## 1.23.2

### Patch Changes

- [#111](https://github.com/alexvcasillas/curl-runner/pull/111) [`1df80e6`](https://github.com/alexvcasillas/curl-runner/commit/1df80e67facf1d8a9daa0f6cfeb917961084f18a) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - fix: remove loose char checks from isRegexPattern to prevent false positives on plain strings like "C++", "really?", "\*-", "[]"

## 1.23.1

### Patch Changes

- [#108](https://github.com/alexvcasillas/curl-runner/pull/108) [`fa60a11`](https://github.com/alexvcasillas/curl-runner/commit/fa60a11c2f1b6a9ab0311de89444e4a9d552ebb0) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Fix array body validation to use element-wise matching when both expected and actual values are arrays, instead of treating all arrays as "one of" enum validators

## 1.23.0

### Minor Changes

- [#102](https://github.com/alexvcasillas/curl-runner/pull/102) [`cbd556f`](https://github.com/alexvcasillas/curl-runner/commit/cbd556fb767d227c663650b9d67c077d73e4a2dd) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - feat: add bidirectional curl ⇄ YAML conversion engine

  - Shell tokenizer with quote/escape/continuation handling
  - Curl semantic parser supporting 40+ flags
  - Normalization layer with method inference, body detection, auth detection, query param extraction
  - YAML serializer with stable key ordering and loss-aware comments
  - Curl generator producing canonical, shell-safe commands
  - Batch script parser for converting entire shell scripts
  - CLI commands: convert curl, convert file, convert yaml
  - Debug mode exposing token stream, AST, and IR

## 1.22.0

### Minor Changes

- [#100](https://github.com/alexvcasillas/curl-runner/pull/100) [`9b6f0a7`](https://github.com/alexvcasillas/curl-runner/commit/9b6f0a799b562f67cc59c0fbd49b8de8e8a4c94a) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - feat: add .env file support with secret redaction

  - Load variables from .env, .env.local, .env.{env}, .env.{env}.local
  - `--env` / `-e` flag to select environment
  - `SECRET_*` prefixed variables automatically redacted in output
  - Pattern-based detection for common API keys (Stripe, AWS, GitHub, NPM, Slack, Paddle, OpenAI, Anthropic)
  - `--no-redact` flag to disable redaction
  - Config option `env.environment` and `env.redactSecrets`

## 1.21.0

### Minor Changes

- [#98](https://github.com/alexvcasillas/curl-runner/pull/98) [`c99f88c`](https://github.com/alexvcasillas/curl-runner/commit/c99f88c00ec64d55e9ededd6408959e7311552d4) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - feat: add validate command for YAML config validation

  New `curl-runner validate` command that:

  - Accepts file paths or glob patterns
  - Validates YAML syntax and structure
  - Checks curl-runner API correctness (methods, auth types, operators, etc.)
  - Validates curl options (SSL, retry, timeout configs)
  - Reports issues with severity (error/warning)
  - Proposes fixes for common issues
  - Auto-fixes with `--fix` flag (uppercase methods, https:// prefix)

## 1.20.0

### Minor Changes

- [#96](https://github.com/alexvcasillas/curl-runner/pull/96) [`022c831`](https://github.com/alexvcasillas/curl-runner/commit/022c831e2509ee23926090aa9c04b183631f1513) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - feat: add YAML wizard for interactive config creation

  - Add `curl-runner init` for quick YAML file creation
  - Add `curl-runner init --wizard` for full interactive wizard
  - Add `curl-runner edit <file>` to modify existing YAML files
  - Support templates: basic-get, basic-post, api-test, file-upload, auth-flow
  - Preview YAML before saving with option to run immediately
  - Add 51 new tests for wizard module

## 1.19.1

### Patch Changes

- [#94](https://github.com/alexvcasillas/curl-runner/pull/94) [`e5a4f15`](https://github.com/alexvcasillas/curl-runner/commit/e5a4f1561ddd311d4118b1f77d19d02bbc034772) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Refactor: modularize core functionality for better testability and maintainability

  - Extract config utilities to `core/config` (CLI parser, env loader)
  - Extract curl utilities to `core/curl` (args builder, response parser, body parser)
  - Extract interpolation to `core/interpolator` (variable resolution)
  - Extract retry logic to `core/execution` (backoff, retry strategies, post-processor)
  - Extract validation to `core/validation` (response validation, body diffing)
  - Extract formatting to `core/format` (duration, size, JSON, colors, tree renderer)
  - Split types/config.ts into domain-specific modules (json, request, execution, snapshot, diff, profile, watch, global)
  - Add 400+ new unit tests for core modules

## 1.19.0

### Minor Changes

- [#92](https://github.com/alexvcasillas/curl-runner/pull/92) [`f8b7744`](https://github.com/alexvcasillas/curl-runner/commit/f8b77447206083b2e91bd7d45fd54ff453b0a3ec) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Add TCP connection pooling with HTTP/2 multiplexing

  - New `connectionPool` global configuration for request batching
  - Groups requests by host and executes in single curl process
  - Uses `curl -Z --parallel --http2` for HTTP/2 stream multiplexing
  - Dramatically reduces TCP/TLS handshake overhead for parallel requests
  - Configuration options: `enabled`, `maxStreamsPerHost`, `keepaliveTime`, `connectTimeout`

## 1.18.0

### Minor Changes

- [#89](https://github.com/alexvcasillas/curl-runner/pull/89) [`6b862c3`](https://github.com/alexvcasillas/curl-runner/commit/6b862c3a57360a068979d0c5a265bc4787b0a500) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Add HTTP/2 support with multiplexing

  - New `--http2` flag to use HTTP/2 protocol for all requests
  - Per-request `http2: true` option in YAML config
  - Environment variable support: `CURL_RUNNER_HTTP2=true`
  - Enables HTTP/2 multiplexing for improved performance

## 1.17.0

### Minor Changes

- [#87](https://github.com/alexvcasillas/curl-runner/pull/87) [`3c23c8c`](https://github.com/alexvcasillas/curl-runner/commit/3c23c8c6856502b2f02fc312157fe22d415af22d) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Add dry-run mode to preview curl commands without executing them

  - New `--dry-run` / `-n` flag to show curl commands without making actual API calls
  - Status displays as "DRY-RUN" in cyan for all output formats
  - JSON output includes `dryRun: true` field instead of status code
  - Environment variable support: `CURL_RUNNER_DRY_RUN=true`
  - Useful for debugging configurations and validating request setups

## 1.16.5

### Patch Changes

- [#84](https://github.com/alexvcasillas/curl-runner/pull/84) [`14dc8aa`](https://github.com/alexvcasillas/curl-runner/commit/14dc8aa736074b1f80a9695da897670196deeedc) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - fix: add process.env fallback in resolveVariable()

## 1.16.4

### Patch Changes

- [#82](https://github.com/alexvcasillas/curl-runner/pull/82) [`2f329ca`](https://github.com/alexvcasillas/curl-runner/commit/2f329cadb4cce5183a325d0f5206179cff35ad40) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - fix: extract semver from monorepo GitHub tags in upgrade command

## 1.16.3

### Patch Changes

- [#80](https://github.com/alexvcasillas/curl-runner/pull/80) [`d398c44`](https://github.com/alexvcasillas/curl-runner/commit/d398c449ece2968d4df5365da14d6a25411ca95b) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - fix: spawn curl directly instead of shell execution

  Removes shell interpretation of ${...} variables in curl commands. Previously, unresolved variables would cause errors on macOS bash 3.2 and Debian dash. Now spawns curl directly with args array, uses --form-string for text form fields to prevent @/< interpretation.

## 1.16.2

### Patch Changes

- [#78](https://github.com/alexvcasillas/curl-runner/pull/78) [`f093b13`](https://github.com/alexvcasillas/curl-runner/commit/f093b13e07830ef2abe98d9d97d281238b472a4f) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - fix: install script download URL and binary name extraction

## 1.16.1

### Patch Changes

- [#75](https://github.com/alexvcasillas/curl-runner/pull/75) [`e0dc09f`](https://github.com/alexvcasillas/curl-runner/commit/e0dc09f0a90ea761aa62036f93d9848d9f139a76) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - fix: include dist folder in npm package and fix install script version parsing

## 1.16.0

### Minor Changes

- [#70](https://github.com/alexvcasillas/curl-runner/pull/70) [`c7ed9b3`](https://github.com/alexvcasillas/curl-runner/commit/c7ed9b3a7eda5a3a6943ca0f6f89caa1d1ff01d1) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - feat: add `curl-runner upgrade` command with auto-detection of installation source

## 1.15.0

### Minor Changes

- [#65](https://github.com/alexvcasillas/curl-runner/pull/65) [`084a3c3`](https://github.com/alexvcasillas/curl-runner/commit/084a3c37030920fc5398cbc178fa8c899a370a05) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - feat: add response diffing for environment/run comparison

  - Compare API responses between environments (staging/prod)
  - Detect API drift between runs over time
  - Exclude dynamic paths (timestamps, IDs)
  - Multiple output formats: terminal, JSON, markdown
  - CI-friendly exit codes
  - Diff subcommand for offline baseline comparison

## 1.14.0

### Minor Changes

- [#59](https://github.com/alexvcasillas/curl-runner/pull/59) [`27e6cc2`](https://github.com/alexvcasillas/curl-runner/commit/27e6cc2ba79bc08ba579f938f63f231ea9e3fbe0) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - feat: add performance profiling mode with p50/p95/p99 latency stats

## 1.13.0

### Minor Changes

- [#61](https://github.com/alexvcasillas/curl-runner/pull/61) [`66b4e8f`](https://github.com/alexvcasillas/curl-runner/commit/66b4e8fd96a10f6659c6e1be844dc510b31e0f56) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - feat: add binary download links to landing and installation pages

## 1.12.0

### Minor Changes

- [#60](https://github.com/alexvcasillas/curl-runner/pull/60) [`3476345`](https://github.com/alexvcasillas/curl-runner/commit/34763453df1f5329dd4dc81414c6f965e000962d) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - feat: add conditional request chaining with `when` field for skip/run logic based on previous results

## 1.11.0

### Minor Changes

- [#57](https://github.com/alexvcasillas/curl-runner/pull/57) [`1ab2e0e`](https://github.com/alexvcasillas/curl-runner/commit/1ab2e0e5af2cb891dde29f0193fefe0e14cdaa24) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Add snapshot testing for API regression testing

  - New `-s, --snapshot` flag to enable snapshot testing
  - New `-u, --update-snapshots` flag to update all snapshots
  - New `--update-failing` flag to update only failing snapshots
  - New `--snapshot-dir <dir>` option for custom snapshot directory
  - New `--ci-snapshot` flag to fail on missing snapshots (CI mode)
  - Support via environment variables: `CURL_RUNNER_SNAPSHOT`, `CURL_RUNNER_SNAPSHOT_UPDATE`, etc.
  - Support via YAML config: `snapshot: true` or detailed config with `include`, `exclude`, `match`
  - Exclusion patterns: exact paths, wildcards (`*.timestamp`), array wildcards (`body[*].id`)
  - Match rules: wildcards (`*`) and regex patterns (`regex:^v\d+`)
  - Snapshots stored in `__snapshots__/<filename>.snap.json`

## 1.10.0

### Minor Changes

- [#55](https://github.com/alexvcasillas/curl-runner/pull/55) [`5f675de`](https://github.com/alexvcasillas/curl-runner/commit/5f675de3a576ec45c522695e8a3542246d9b2438) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Add watch mode for automatic re-execution on file changes

  - New `-w, --watch` flag to enable watch mode
  - `--watch-debounce <ms>` option to customize debounce delay (default: 300ms)
  - `--no-watch-clear` option to disable screen clearing between runs
  - Support via environment variables: `CURL_RUNNER_WATCH`, `CURL_RUNNER_WATCH_DEBOUNCE`, `CURL_RUNNER_WATCH_CLEAR`
  - Support via YAML config under `global.watch`

## 1.9.0

### Minor Changes

- [#37](https://github.com/alexvcasillas/curl-runner/pull/37) [`9968188`](https://github.com/alexvcasillas/curl-runner/commit/996818855042d12b2dce364b66972bca92add697) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Add SSL/TLS certificate configuration support

  - New `ssl` configuration option for requests and global settings
  - Support for custom CA certificates (`ssl.ca`)
  - Support for mutual TLS (mTLS) with client certificates (`ssl.cert`, `ssl.key`)
  - Support for disabling SSL verification (`ssl.verify: false`)

## 1.8.0

### Minor Changes

- [#36](https://github.com/alexvcasillas/curl-runner/pull/36) [`8722304`](https://github.com/alexvcasillas/curl-runner/commit/8722304aab156442e4e015acc8a9c046a5a029c5) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Add exponential backoff support for retry mechanism

  The retry configuration now supports a `backoff` multiplier for exponential backoff between retries. When set, the delay increases exponentially with each retry attempt using the formula: `delay * backoff^(attempt-1)`.

  Example usage:

  ```yaml
  retry:
    count: 3
    delay: 1000 # Initial delay: 1 second
    backoff: 2.0 # Multiplier
  # Delays: 1000ms, 2000ms, 4000ms
  ```

  The backoff defaults to 1 (no backoff) to maintain backward compatibility.

## 1.7.0

### Minor Changes

- [#35](https://github.com/alexvcasillas/curl-runner/pull/35) [`c398f6b`](https://github.com/alexvcasillas/curl-runner/commit/c398f6bed818a43a62c42b23aa41d875bdc2a354) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Add maxConcurrency option for parallel execution

  - New `maxConcurrency` setting in global config to limit simultaneous requests in parallel mode
  - New `--max-concurrent <n>` CLI flag
  - New `CURL_RUNNER_MAX_CONCURRENCY` environment variable support

  This helps avoid rate limiting from APIs and prevents overwhelming target servers when running many requests in parallel.

## 1.6.0

### Minor Changes

- [#34](https://github.com/alexvcasillas/curl-runner/pull/34) [`c9487a3`](https://github.com/alexvcasillas/curl-runner/commit/c9487a30245ae9cee02affb015ab3f0a8758f6fe) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Add string transformation support for variables

  New `:upper` and `:lower` transforms for case manipulation:

  - `${VAR:upper}` converts variable value to uppercase
  - `${VAR:lower}` converts variable value to lowercase
  - Works with both static variables and environment variables

  Example usage:

  ```yaml
  global:
    variables:
      ENV: "production"
      RESOURCE: "Users"

  collection:
    requests:
      - name: API Request
        url: "https://api.example.com/${RESOURCE:lower}"
        headers:
          X-Environment: "${ENV:upper}"
  ```

  This resolves to `https://api.example.com/users` with header `X-Environment: PRODUCTION`.

## 1.5.0

### Minor Changes

- [#33](https://github.com/alexvcasillas/curl-runner/pull/33) [`fd9eb74`](https://github.com/alexvcasillas/curl-runner/commit/fd9eb74d0006a6da27e9e3b8942eb1f4257bdb7e) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Add default value syntax for variables (`${VAR:default}`)

  Variables can now specify fallback values that are used when the variable is not set:

  - Basic defaults: `${API_TIMEOUT:5000}` - uses "5000" if API_TIMEOUT is not defined
  - Nested defaults: `${DATABASE_HOST:${DB_HOST:localhost}}` - tries DATABASE_HOST, then DB_HOST, then falls back to "localhost"

  This syntax does not conflict with existing dynamic variable patterns like `${DATE:YYYY-MM-DD}` or `${TIME:HH:mm:ss}`.

## 1.4.0

### Minor Changes

- [#31](https://github.com/alexvcasillas/curl-runner/pull/31) [`3a9f252`](https://github.com/alexvcasillas/curl-runner/commit/3a9f2525788a8193bc6947c3d6b32379b14d480c) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Add support for additional dynamic variable generators:

  - `${UUID:short}` - Returns the first 8 characters of a UUID
  - `${RANDOM:min-max}` - Generates a random integer in the specified range
  - `${RANDOM:string:length}` - Generates a random alphanumeric string of specified length

  These variables are useful for API testing scenarios where shorter IDs or random test data are needed.

## 1.3.0

### Minor Changes

- [#28](https://github.com/alexvcasillas/curl-runner/pull/28) [`e35a89d`](https://github.com/alexvcasillas/curl-runner/commit/e35a89ddcf23a03a4d170057490ab6e783271fb2) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Add file attachment support for multipart/form-data requests

  New `formData` property for uploading files and sending form data:

  - Send files using the `file` property with path to the file
  - Customize the filename sent to the server with `filename`
  - Specify explicit MIME type with `contentType`
  - Mix file attachments with regular form fields
  - Automatic file existence validation before request execution

  Example usage:

  ```yaml
  request:
    name: Upload Document
    url: https://api.example.com/upload
    method: POST
    formData:
      title: "My Document"
      document:
        file: "./report.pdf"
        filename: "quarterly-report.pdf"
        contentType: "application/pdf"
  ```

## 1.2.0

### Minor Changes

- [#25](https://github.com/alexvcasillas/curl-runner/pull/25) [`468fadc`](https://github.com/alexvcasillas/curl-runner/commit/468fadc12e25553c74cabdf782098ee52d994723) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Add CI-friendly exit codes for pipeline integration

  New CLI options for controlling exit code behavior in CI/CD pipelines:

  - `--strict-exit`: Exit with code 1 if any validation fails, regardless of `--continue-on-error`
  - `--fail-on <count>`: Exit with code 1 only if failures exceed the specified threshold
  - `--fail-on-percentage <pct>`: Exit with code 1 only if failure percentage exceeds the threshold

  New environment variables:

  - `CURL_RUNNER_STRICT_EXIT`: Enable strict exit mode
  - `CURL_RUNNER_FAIL_ON`: Set maximum allowed failures
  - `CURL_RUNNER_FAIL_ON_PERCENTAGE`: Set maximum allowed failure percentage

  This enables cleaner CI/CD integration across platforms (GitHub Actions, GitLab CI, CircleCI, Jenkins) without requiring additional parsing logic to detect validation failures.

## 1.1.0

### Minor Changes

- [#22](https://github.com/alexvcasillas/curl-runner/pull/22) [`102be9d`](https://github.com/alexvcasillas/curl-runner/commit/102be9d86e6b11ef92657134c405954bf923c9f3) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - feat: Add response storage for sequential request chaining

  Store response values from one request and use them in subsequent requests using the new `store` property and `${store.variableName}` syntax.

  **New Features:**

  - Add `store` property to request configuration for extracting response values
  - Support JSON path syntax for extracting nested values (e.g., `body.data.token`, `headers.content-type`)
  - Reference stored values in subsequent requests using `${store.variableName}`
  - Works seamlessly with existing static and dynamic variables

  **Example:**

  ```yaml
  requests:
    - name: Login
      url: https://api.example.com/auth/login
      method: POST
      body:
        username: "admin"
        password: "secret"
      store:
        authToken: body.accessToken
        userId: body.user.id

    - name: Get Profile
      url: https://api.example.com/users/${store.userId}
      headers:
        Authorization: Bearer ${store.authToken}
  ```

  **Supported paths:**

  - `body.field` - Response body fields
  - `body.nested.field` - Nested fields
  - `body.array.0.field` - Array elements by index
  - `headers.name` - Response headers
  - `status` - HTTP status code
  - `metrics.duration` - Response metrics

  **Note:** Response storage requires sequential execution mode (the default).

## 1.0.3

### Patch Changes

- [#11](https://github.com/alexvcasillas/curl-runner/pull/11) [`3fd1c66`](https://github.com/alexvcasillas/curl-runner/commit/3fd1c6640dda618ead28bee21491db28bf9be519) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - A major redesign of the output formatting for the CLI tool, focusing on a new "minimal" pretty format and a tree-based rendering for request results. It simplifies the output structure, improves readability, and updates documentation to clarify feature availability

## 1.0.2

### Patch Changes

- [#8](https://github.com/alexvcasillas/curl-runner/pull/8) [`e5d29ba`](https://github.com/alexvcasillas/curl-runner/commit/e5d29ba10590072ff94bcae031552ad772e1d318) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Improvements over the release process and improvements over the version checking

- [#7](https://github.com/alexvcasillas/curl-runner/pull/7) [`0f1d1b5`](https://github.com/alexvcasillas/curl-runner/commit/0f1d1b5609a4848dd2da9482ef219ceee4812229) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Improve version checking, version releases and perform platform builds for release

## 1.0.1

### Patch Changes

- [#1](https://github.com/alexvcasillas/curl-runner/pull/1) [`41bfb79`](https://github.com/alexvcasillas/curl-runner/commit/41bfb79d64b3229d811b41ab3e84da7961164310) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - Setup automated release process with Changesets

  - Added Changesets configuration for automated versioning and publishing
  - Created GitHub Actions workflow for CI/CD pipeline
  - Configured NPM publishing with proper access controls
