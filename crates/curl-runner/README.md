# curl-runner

Run HTTP request suites defined in YAML, on libcurl: response validation, retries,
sequential or parallel runs, stored values between requests, snapshots and response diffs.

## Install

```bash
cargo install curl-runner --locked
```

Building from source compiles libcurl and the TLS library, so it needs a C compiler and
`cmake`. Prebuilt binaries are also available:

```bash
curl -fsSL https://www.curl-runner.com/install.sh | bash   # macOS, Linux
npm i -g @curl-runner/cli                                   # or bun / pnpm
```

## Usage

```yaml
# api.yaml
request:
  name: Get a post
  url: https://jsonplaceholder.typicode.com/posts/1
  expect:
    status: 200
    body:
      id: 1
```

```bash
curl-runner api.yaml
curl-runner tests/ --execution parallel --output results.json
curl-runner upgrade
```

Documentation: <https://www.curl-runner.com/docs>

## License

MIT
