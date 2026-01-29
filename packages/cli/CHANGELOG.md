# @curl-runner/cli

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
