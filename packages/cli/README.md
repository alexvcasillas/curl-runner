# @curl-runner/cli

[![TypeScript](https://img.shields.io/badge/typescript-%23007ACC.svg?style=flat-square&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-%23000000.svg?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![MIT License](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)

A powerful CLI tool for HTTP request management using YAML configuration files. A single native Rust binary (libcurl statically linked; no Bun, Node or curl needed at runtime) featuring variable interpolation, parallel execution, and response validation.

## 🚀 Features

- **📝 YAML Configuration**: Define HTTP requests in simple, readable YAML files
- **📁 Directory Support**: Execute multiple requests from entire directories  
- **🔄 Execution Modes**: Sequential and parallel execution for optimal performance
- **🔧 Variable Interpolation**: Dynamic values using environment variables and inline substitutions
- **✅ Response Validation**: Built-in assertions for status codes, headers, and response bodies
- **📊 Beautiful Output**: Clean, colorized console output with detailed metrics
- **🔄 Retry Logic**: Automatic retry mechanisms with configurable delays
- **🔐 Authentication**: Support for Basic Auth and Bearer token authentication
- **⚡ Performance Metrics**: Detailed timing statistics for each request
- **🎯 Glob Patterns**: Flexible file matching with glob support

## 📦 Installation

### npm / Bun / pnpm

```bash
bun i -g @curl-runner/cli    # no Node needed, no postinstall to trust
npm i -g @curl-runner/cli    # no Bun needed; the runtime needs neither Node nor Bun
pnpm add -g @curl-runner/cli
```

### How the npm package works

`@curl-runner/cli` contains no JavaScript. It ships a native binary through
one optional dependency per platform, versioned in lockstep with it:

| Package | `os` / `cpu` | Binary |
| --- | --- | --- |
| `@curl-runner/cli-linux-x64` | linux / x64 | static musl |
| `@curl-runner/cli-linux-arm64` | linux / arm64 | static musl |
| `@curl-runner/cli-darwin-x64` | darwin / x64 | Mach-O |
| `@curl-runner/cli-darwin-arm64` | darwin / arm64 | Mach-O |
| `@curl-runner/cli-win32-x64` | win32 / x64 | `curl-runner.exe` |

npm, Bun, pnpm and yarn download only the package whose `os`/`cpu` match.
The `curl-runner` bin is `bin/curl-runner`, a POSIX `sh` launcher
(`#!/usr/bin/env sh`): it maps `uname` to the platform, finds
`node_modules/@curl-runner/cli-<platform>/bin/curl-runner` by walking up
`node_modules` directories like Node's resolver (npm nested/hoisted, Bun
global, pnpm `.pnpm` siblings, yarn), and `exec`s it. If the platform is
unsupported or its package is missing (for example after `--omit=optional`),
it exits 1 with a message saying so.

Why this design:

- **No postinstall.** Bun does not run lifecycle scripts of untrusted
  dependencies, so the esbuild-style "postinstall copies the binary over the
  bin" approach breaks `bun i -g`. Optional dependencies plus a launcher need
  no scripts at all.
- **No Node at runtime.** A JS launcher (`#!/usr/bin/env node`) fails on a
  machine with only Bun. `sh` is always present on Linux and macOS.
- **Lockstep versions.** The platform packages live in `npm/cli-*` with
  version `0.0.0`. At release, `scripts/npm-stage.sh` stamps them with the
  `@curl-runner/cli` version, copies in the CI-built binaries and adds the
  exact-version `optionalDependencies` to this package (kept out of git so
  `bun.lock` never points at unpublished versions). `scripts/npm-publish.sh`
  then publishes the five platform packages before `changeset publish`
  publishes `@curl-runner/cli`.

Windows: package managers wrap bins in shims (npm's cmd-shim writes
`curl-runner.cmd`/`.ps1`; Bun writes a `.exe` shim). Both read the shebang and
run `sh <launcher>`, so `curl-runner` works from cmd and PowerShell only when
a `sh` is on `PATH` (Git for Windows with its Unix tools on `PATH`, MSYS2,
Cygwin), and always from Git Bash. Without `sh`, install the native binary
with `install.ps1` from the docs, or run
`node_modules/@curl-runner/cli-win32-x64/bin/curl-runner.exe` directly.
Mapping a single `bin` entry to a native `.exe` on Windows would require a
postinstall, which is what this design avoids.

Testing: `scripts/npm-smoke.sh <bin-dir> [linux/arm64 linux/amd64]` packs
everything, publishes to a throwaway local verdaccio and installs it with
`bun i -g` (no node), `npm i -g` (no bun) and `pnpm add -g` in Docker.

### From Source

Needs [Rust](https://rustup.rs) (toolchain pinned in `rust-toolchain.toml`); Bun is not required.

```bash
git clone https://github.com/alexvcasillas/curl-runner.git
cd curl-runner

# Build the CLI
cargo build --release -p curl-runner

# The binary is at ./target/release/curl-runner
```

### Development Mode

```bash
# Run from source
cargo run -p curl-runner -- [files...] [options]

# Or use the workspace script
bun run cli [files...] [options]
```

## 🚀 Quick Start

### Basic Usage

```bash
# Run a single YAML file
curl-runner api-test.yaml

# Run all YAML files in current directory
curl-runner

# Run all files in a directory
curl-runner examples/

# Run with options
curl-runner -pv examples/  # Parallel + Verbose mode
```

### Your First Request

Create `my-first-request.yaml`:

```yaml
request:
  name: Get User Data
  url: https://jsonplaceholder.typicode.com/users/1
  method: GET
  expect:
    status: 200
```

Run it:

```bash
curl-runner my-first-request.yaml
```

## 📝 YAML Configuration

### Single Request Format

```yaml
request:
  name: Request Name                    # Optional: Human-readable name
  url: https://api.example.com/users    # Required: Target URL
  method: GET                           # HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
  headers:                              # Optional: Custom headers
    Authorization: Bearer ${TOKEN}
    Content-Type: application/json
  params:                               # Optional: Query parameters
    page: 1
    limit: 10
  body:                                 # Optional: Request body (JSON, string, etc.)
    name: John Doe
    email: john@example.com
  timeout: 5000                         # Optional: Request timeout in ms (default: 30000)
  followRedirects: true                 # Optional: Follow HTTP redirects (default: true)
  maxRedirects: 5                       # Optional: Max redirect count (default: 5)
  auth:                                 # Optional: Authentication
    type: basic|bearer
    username: user                      # For basic auth
    password: pass                      # For basic auth
    token: your-token                   # For bearer auth
  retry:                                # Optional: Retry configuration
    count: 3                            # Number of retry attempts
    delay: 1000                         # Delay between retries in ms
  expect:                               # Optional: Response validation
    status: 200                         # Expected status code(s)
    headers:                            # Expected headers
      Content-Type: application/json
    body:                               # Expected body content
      contains: "success"
```

### Multiple Requests Format

```yaml
requests:
  - name: Get Users
    url: https://api.example.com/users
    method: GET
  
  - name: Create User
    url: https://api.example.com/users
    method: POST
    headers:
      Content-Type: application/json
    body:
      name: Jane Doe
      email: jane@example.com
```

### Collection Format

```yaml
global:
  execution: sequential|parallel        # Execution mode
  continueOnError: true|false          # Continue on request failures
  variables:                           # Global variables
    BASE_URL: https://api.example.com
    API_KEY: ${API_KEY}                # Environment variable
  output:                              # Output configuration
    verbose: true                      # Show detailed output
    showHeaders: true                  # Display response headers
    showBody: true                     # Display response body
    showMetrics: true                  # Display performance metrics
    format: pretty|json|raw            # Output format
    saveToFile: results.json           # Save results to file
  defaults:                            # Default request settings
    headers:
      User-Agent: curl-runner/1.0.0
    timeout: 10000

collection:
  name: API Integration Tests
  description: Complete test suite for our API
  variables:                           # Collection-specific variables
    USER_ID: 123
  defaults:                            # Collection-specific defaults
    headers:
      Accept: application/json
  requests:
    - name: Get User Profile
      url: ${BASE_URL}/users/${USER_ID}
      method: GET
      expect:
        status: 200
        body:
          contains: "email"
    
    - name: Update Profile
      url: ${BASE_URL}/users/${USER_ID}
      method: PUT
      headers:
        Content-Type: application/json
      body:
        name: Updated Name
      expect:
        status: [200, 204]
```

## 🔧 Variable Interpolation

Variables can be defined at multiple levels and are resolved in this order:

1. **Environment variables**: `${ENV_VAR}`
2. **Global variables**: Defined in `global.variables`
3. **Collection variables**: Defined in `collection.variables`
4. **Request variables**: Defined in `request.variables`

```yaml
global:
  variables:
    BASE_URL: https://api.staging.com
    VERSION: v1

collection:
  variables:
    RESOURCE: users
    USER_ID: 123

requests:
  - name: Get User
    url: ${BASE_URL}/${VERSION}/${RESOURCE}/${USER_ID}
    method: GET
    variables:
      USER_ID: 456  # Overrides collection variable
```

## 📊 CLI Options

```bash
Usage: curl-runner [files...] [options]

Files:
  files...                  YAML files or directories to process
                           (default: current directory *.yaml, *.yml)

Options:
  -h, --help               Show help message
  -v, --verbose            Enable verbose output
  -p, --execution parallel Execute requests in parallel (default: sequential)
  -c, --continue-on-error  Continue execution even if requests fail
  --all                    Find all YAML files recursively in directories
  --output <file>          Save execution results to JSON file
  --version                Show version information

Examples:
  curl-runner                          # Run all .yaml/.yml files in current directory
  curl-runner api-test.yaml           # Run specific file
  curl-runner tests/ examples/        # Run all files in multiple directories
  curl-runner --all -p                # Run all files recursively in parallel
  curl-runner tests/*.yaml -vc        # Run with verbose output, continue on errors
```

## 🎯 Advanced Features

### Authentication

```yaml
# Basic Authentication
request:
  url: https://api.example.com/protected
  method: GET
  auth:
    type: basic
    username: myuser
    password: mypass

# Bearer Token Authentication
request:
  url: https://api.example.com/protected
  method: GET
  auth:
    type: bearer
    token: ${ACCESS_TOKEN}
```

### Response Validation

```yaml
request:
  url: https://api.example.com/health
  method: GET
  expect:
    status: 200                        # Single status code
    # status: [200, 201, 204]         # Multiple acceptable status codes
    headers:
      Content-Type: application/json
      X-Rate-Limit-Remaining: "*"     # Use "*" for any value
    body:
      contains: "healthy"              # Body must contain this text
      # For JSON responses, you can validate structure:
      # json:
      #   status: "ok"
      #   version: "1.0.0"
```

### Retry Configuration

```yaml
request:
  url: https://api.example.com/flaky-endpoint
  method: GET
  retry:
    count: 3                          # Retry up to 3 times
    delay: 1000                       # Wait 1 second between retries
  timeout: 5000                       # 5 second timeout per attempt
```

### Performance Monitoring

```yaml
global:
  output:
    showMetrics: true                 # Enable performance metrics
    format: pretty                    # Human-readable output

request:
  url: https://api.example.com/data
  method: GET
```

Example output with metrics:
```
✅ Get API Data (200 OK)
   📊 Duration: 245ms
   📈 DNS Lookup: 12ms
   🔗 TCP Connection: 45ms
   🔐 TLS Handshake: 89ms
   ⏱️  Time to First Byte: 156ms
   📥 Download: 34ms
   📦 Response Size: 1.2KB
```

## 📁 File Organization

### Directory Structure Example

```
api-tests/
├── auth/
│   ├── login.yaml
│   └── refresh-token.yaml
├── users/
│   ├── crud-operations.yaml
│   └── profile-management.yaml
├── orders/
│   └── order-lifecycle.yaml
└── global-config.yaml
```

### Running Directory Tests

```bash
# Run all tests in auth directory
curl-runner api-tests/auth/

# Run all tests recursively
curl-runner --all api-tests/

# Run specific test suites in parallel
curl-runner -p api-tests/users/ api-tests/orders/
```

## 🔍 Example YAML Files

The `examples/` directory contains sample configurations:

- **`simple.yaml`**: Basic GET request
- **`collection.yaml`**: Multiple requests with global configuration
- **`parallel.yaml`**: Parallel execution with various HTTP methods
- **`test-with-retry.yaml`**: Retry logic and error handling

### Running Examples

```bash
# Run a specific example
curl-runner examples/simple.yaml

# Run all examples
curl-runner examples/

# Run examples with verbose output
curl-runner -v examples/
```

## 🧪 Testing & Development

### Running Tests

```bash
# Run unit and integration tests
cargo test --workspace

# Run the example files
curl-runner examples/

# Test CLI functionality
cargo run -p curl-runner -- --help
```

### Development Commands

```bash
# Build the CLI binary
cargo build --release -p curl-runner

# Check code quality
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

## 🎨 Output Formats

### Pretty Format (Default)

```
🚀 CURL RUNNER

✅ Get User Data (200 OK)
   📊 Duration: 234ms
   📦 Size: 856 bytes

❌ Create Invalid User (422 Unprocessable Entity)
   📊 Duration: 145ms
   ❗ Validation failed: email is required

📈 Summary: 1 successful, 1 failed (379ms total)
```

### JSON Format

```bash
curl-runner --output results.json tests/
```

```json
{
  "summary": {
    "total": 2,
    "successful": 1,
    "failed": 1,
    "duration": 379
  },
  "results": [
    {
      "request": {
        "name": "Get User Data",
        "url": "https://api.example.com/users/1",
        "method": "GET"
      },
      "success": true,
      "status": 200,
      "metrics": {
        "duration": 234,
        "size": 856
      }
    }
  ]
}
```

## 🐛 Troubleshooting

### Common Issues

1. **No YAML files found**
   ```bash
   # Make sure you're in the right directory or specify files explicitly
   curl-runner path/to/your/files.yaml
   ```

2. **Request timeout**
   ```yaml
   request:
     url: https://slow-api.example.com
     timeout: 10000  # Increase timeout to 10 seconds
   ```

3. **SSL certificate issues**
   ```yaml
   request:
     url: https://self-signed-cert.example.com
     insecure: true  # Skip SSL verification (not recommended for production)
   ```

4. **Variable interpolation not working**
   ```bash
   # Make sure environment variables are exported
   export API_KEY=your-key-here
   curl-runner config.yaml
   ```

### Debug Mode

```bash
# Run with maximum verbosity to debug issues
curl-runner -v your-config.yaml
```

## 🤝 Contributing

We welcome contributions! Please see the main [CONTRIBUTE.md](../../CONTRIBUTE.md) for guidelines.

### CLI-Specific Development

When contributing to the CLI:

1. **Test with examples**: Always test your changes with the provided examples
2. **Update TypeScript types**: Keep `src/types/config.ts` up to date
3. **Add tests**: Include unit tests for new functionality
4. **Update this README**: Document any new features or configuration options

## 📄 License

MIT License - see [LICENSE.md](../../LICENSE.md) for details.

## 🔗 Related

- **[Main Repository](../../@README.md)**: Full project documentation
- **[Documentation Website](../docs/)**: Interactive documentation and examples
- **[Contributing Guide](../../CONTRIBUTE.md)**: How to contribute to the project

---

Built with ❤️ using Rust and libcurl