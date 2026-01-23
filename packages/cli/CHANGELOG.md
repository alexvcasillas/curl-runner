# @curl-runner/cli

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
