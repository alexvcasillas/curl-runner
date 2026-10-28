---
'@curl-runner/cli': major
---

curl-runner 2.0 is a rewrite of the CLI in Rust. YAML files, flags, subcommands, environment variables, config discovery and exit codes stay the same, with the bug fixes and behavior changes listed below.

### Runtime

- The CLI is now a native Rust binary with libcurl linked statically (rustls for TLS). No Bun, Node.js or `curl` binary is needed at runtime. HTTPS uses the operating system trust store (on Debian/Ubuntu images keep `ca-certificates` installed) unless `ssl.ca` is set.
- `curl-runner upgrade` replaces the binary in-process (checksum verified); npm and bun installs still run `npm|bun install -g @curl-runner/cli@latest`.

### Install changes

- `@curl-runner/cli` has no JavaScript and no postinstall. It ships the binary through one optional dependency per platform (`@curl-runner/cli-linux-x64`, `-linux-arm64`, `-darwin-x64`, `-darwin-arm64`, `-win32-x64`), downloaded by npm, bun, pnpm and yarn according to `os`/`cpu`. Installing with `--omit=optional` leaves no binary.
- The `curl-runner` bin is a POSIX `sh` launcher. On Windows the npm/bun/pnpm/yarn shims therefore need `sh` on `PATH` (Git for Windows, MSYS2 or Cygwin); without `sh`, install with `install.ps1` or run the `.exe` from the platform package directly.
- Linux binaries are static musl builds.

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
- `interpolate` failures (a value that no longer fits a typed field, for example `timeout: "${store.name}"`) fail the request with `invalid value at \`timeout\`: ...` instead of running it with the raw value. An explicit `null` in a request replaces the default, like a JS spread. `RANDOM:string:N` with an unrepresentable `N` is left unresolved instead of aborting.
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
