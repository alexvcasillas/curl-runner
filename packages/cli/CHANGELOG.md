# @curl-runner/cli

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
