# 🚀 curl-runner

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/typescript-%23007ACC.svg?style=for-the-badge&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Next.js](https://img.shields.io/badge/Next-black?style=for-the-badge&logo=next.js&logoColor=white)](https://nextjs.org/)
[![YAML](https://img.shields.io/badge/yaml-%23ffffff.svg?style=for-the-badge&logo=yaml&logoColor=151515)](https://yaml.org/)
[![MIT License](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)

A powerful CLI tool and documentation website for HTTP request management using YAML configuration files. The CLI is a single native Rust binary (libcurl statically linked, no Bun, Node or curl needed at runtime), and the documentation site is powered by Next.js.

## 🏢 Companies using `curl-runner`

<div align="center">
  <a href="https://tiptap.dev"><img src="https://avatars.githubusercontent.com/u/16939337?v=4" alt="Tiptap" height="40"></a>
  &nbsp;&nbsp;&nbsp;&nbsp;
  <a href="https://pulsora.co"><img src="https://pulsora.co/favicon.png" alt="Pulsora" height="40"></a>
</div>

## ✨ Features

### 🎯 Core CLI Features

- **📝 YAML Configuration**: Define HTTP requests in simple, readable YAML files
- **📁 Directory Support**: Execute multiple requests from entire directories
- **🔄 Execution Modes**: Sequential and parallel execution for optimal performance
- **🔧 Variable Interpolation**: Dynamic values using environment variables and inline substitutions
- **✅ Response Validation**: Built-in assertions for status codes, headers, and response bodies
- **📊 Beautiful Output**: Clean, colorized console output with detailed metrics
- **🔄 Retry Logic**: Automatic retry mechanisms for failed requests
- **📈 Performance Metrics**: Detailed timing and performance statistics

### 🌐 Documentation Website

- **📖 Comprehensive Docs**: Complete documentation with examples and tutorials
- **🎨 Modern Design**: Beautiful, responsive UI built with Next.js and Tailwind CSS
- **🌙 Dark Mode**: Built-in dark mode support
- **🔍 Interactive Examples**: Live code examples and playground
- **📱 Mobile Friendly**: Optimized for all screen sizes

## 📦 Installation

### Install the CLI

```bash
npm i -g @curl-runner/cli     # or: bun i -g @curl-runner/cli, pnpm add -g @curl-runner/cli
curl-runner --version
```

On macOS or Linux with Homebrew: `brew install alexvcasillas/tap/curl-runner`.

Or build it from crates.io with `cargo install curl-runner --locked` (needs a C compiler
and `cmake`).

The npm package ships a prebuilt native binary per platform; no Node, Bun or curl is
needed to run it. See [packages/cli/README.md](packages/cli/README.md) for the install
script, Windows notes and how the npm package works.

### Build from source

Prerequisites: [Rust](https://rustup.rs) (the toolchain is pinned in `rust-toolchain.toml`).
[Bun](https://bun.sh) is only needed for the documentation site.

```bash
git clone https://github.com/alexvcasillas/curl-runner.git
cd curl-runner

# Build the CLI (target/release/curl-runner)
cargo build --release -p curl-runner

# Documentation site only
bun install
```

## 🚀 Quick Start

### Basic Usage

```bash
# Run a single YAML file
curl-runner examples/simple.yaml

# Run all YAML files in a directory
curl-runner examples/

# Run with options
curl-runner -pv examples/  # Parallel + Verbose mode
```

### Your First Request

Create a file called `my-request.yaml`:

```yaml
# Simple GET request
request:
  name: Get User Data
  url: https://jsonplaceholder.typicode.com/users/1
  method: GET
  expect:
    status: 200
```

Run it:

```bash
curl-runner my-request.yaml
```

### Multiple Requests (Collection)

Create `api-tests.yaml`:

```yaml
global:
  execution: sequential
  variables:
    BASE_URL: https://jsonplaceholder.typicode.com

collection:
  name: API Integration Tests
  description: Testing user management endpoints
  requests:
    - name: List All Users
      url: ${BASE_URL}/users
      method: GET
      expect:
        status: 200

    - name: Get Specific User
      url: ${BASE_URL}/users/1
      method: GET
      expect:
        status: 200

    - name: Create New User
      url: ${BASE_URL}/users
      method: POST
      headers:
        Content-Type: application/json
      body:
        name: John Doe
        email: john@example.com
      expect:
        status: 201
```

## 📁 Project Structure

```
curl-runner/
├── packages/
│   ├── cli/                 # npm wrapper (launcher + platform packages)
│   │   ├── bin/            # curl-runner launcher
│   │   ├── examples/       # Example YAML files
│   │   └── package.json
│   └── docs/               # Documentation website
│       ├── app/            # Next.js app directory
│       ├── components/     # React components
│       └── package.json
├── crates/                 # Rust CLI (curl-runner, curl-runner-core, testkit)
├── parity/                 # TS vs Rust parity harness
├── package.json            # Root workspace configuration
├── biome.json             # Code formatting and linting
├── @README.md             # This file
├── CONTRIBUTE.md          # Contribution guidelines
└── LICENSE.md             # MIT License
```

## 🛠️ Development

### Workspace Commands

```bash
# Install dependencies for all packages
bun install

# CLI (Rust)
cargo run -p curl-runner -- packages/cli/examples/simple.yaml
cargo build --release -p curl-runner               # or: bun run build:cli
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                             # or: bun run test

# Documentation site
bun run dev:docs         # Development server
bun run build:docs       # Build documentation site

# Code quality (TypeScript: docs site and tooling)
bun run format           # Format code with Biome
bun run lint             # Lint code with Biome
bun run check            # Format and fix all issues
bun run check:ci         # CI-friendly check
```

### Running the Documentation Site

```bash
# Start development server
bun run dev:docs

# Build and start production server
bun run build:docs
bun run start:docs
```

Visit [http://localhost:3000](http://localhost:3000) to view the documentation.

## 📝 YAML Configuration Reference

### Single Request

```yaml
request:
  name: Request Name
  url: https://api.example.com/endpoint
  method: GET|POST|PUT|DELETE|PATCH
  headers:
    Authorization: Bearer ${TOKEN}
    Content-Type: application/json
  body: |
    {
      "key": "value"
    }
  expect:
    status: 200
    headers:
      Content-Type: application/json
    body:
      contains: "success"
```

### Collection (Multiple Requests)

```yaml
global:
  execution: sequential|parallel
  continueOnError: true|false
  variables:
    API_KEY: ${API_KEY}
    BASE_URL: https://api.example.com
  output:
    verbose: true
    showHeaders: true
    showMetrics: true

collection:
  name: Collection Name
  description: Collection description
  requests:
    - name: Request 1
      url: ${BASE_URL}/endpoint1
      method: GET
    - name: Request 2
      url: ${BASE_URL}/endpoint2
      method: POST
      body:
        data: value
```

### Advanced Features

#### Variable Interpolation

```yaml
global:
  variables:
    API_VERSION: v1
    USER_ID: 123

request:
  name: Get User Profile
  url: https://api.example.com/${API_VERSION}/users/${USER_ID}
  method: GET
```

#### Response Validation

```yaml
request:
  name: API Health Check
  url: https://api.example.com/health
  method: GET
  expect:
    status: 200
    headers:
      Content-Type: application/json
    body:
      contains: "healthy"
      json:
        status: "ok"
        version: "1.0.0"
```

#### Retry Configuration

```yaml
request:
  name: Flaky Endpoint
  url: https://api.example.com/flaky
  method: GET
  retry:
    attempts: 3
    delay: 1000
    exponential: true
  expect:
    status: 200
```

## 📚 CLI Options

```bash
# Execution modes
-p, --parallel          Execute requests in parallel
-s, --sequential        Execute requests sequentially (default)

# Output control
-v, --verbose           Show detailed output
-q, --quiet             Suppress non-essential output
--show-headers          Display response headers
--show-metrics          Display timing metrics

# Error handling
-c, --continue          Continue execution on errors
--fail-fast             Stop on first error (default)

# Output formats
-o, --output <file>     Save results to JSON file
--format json|yaml      Output format for results

# Other options
-h, --help              Show help information
--version               Show version information
-w, --watch             Watch files for changes
```

## 🧪 Examples

### API Testing

```bash
# Test a REST API with authentication
curl-runner examples/api-auth.yaml

# Load test with parallel requests
curl-runner -p examples/load-test/

# Integration test suite
curl-runner --verbose examples/integration/
```

### Environment-Specific Testing

```bash
# Set environment variables
export API_KEY=your-api-key
export BASE_URL=https://api.staging.example.com

# Run tests with environment variables
curl-runner examples/staging-tests.yaml
```

### Continuous Integration

```bash
# CI-friendly execution with JSON output
curl-runner --quiet --output results.json --continue examples/

# Validate API contract
curl-runner --fail-fast examples/contract-tests/
```

## 🏗️ Architecture

### CLI (`crates/`, published as `@curl-runner/cli`)

- **Rust** with libcurl linked statically (rustls); one native binary per platform
- **YAML parsing** for configuration
- **HTTP client** with retry logic
- **Validation engine** for assertions
- **Pretty printing** for console output

### Documentation Package (`@curl-runner/docs`)

- **Next.js 15** with App Router
- **React 19** with TypeScript
- **Tailwind CSS** for styling
- **Radix UI** for accessible components
- **Shiki** for syntax highlighting
- **GSAP** for animations

## 🤝 Contributing

We welcome contributions! Please see our [CONTRIBUTE.md](CONTRIBUTE.md) for guidelines on:

- Setting up the development environment
- Code style and conventions
- Submitting pull requests
- Reporting bugs and feature requests

## 📄 License

This project is licensed under the MIT License. See [LICENSE.md](LICENSE.md) for details.

## 🙏 Acknowledgments

- **[Rust](https://www.rust-lang.org/)** and **[libcurl](https://curl.se/libcurl/)** - The CLI runtime
- **[Bun](https://bun.sh)** - JavaScript runtime and package manager for the docs site
- **[Next.js](https://nextjs.org/)** - The React framework for production
- **[Biome](https://biomejs.dev/)** - Fast formatter and linter
- **[Tailwind CSS](https://tailwindcss.com/)** - Utility-first CSS framework
- **[Radix UI](https://www.radix-ui.com/)** - Low-level UI primitives

## 📞 Support

- 🐛 [Report bugs](https://github.com/alexvcasillas/curl-runner/issues)
- 💡 [Request features](https://github.com/alexvcasillas/curl-runner/issues)
- 📖 [View documentation](https://alexvcasillas.github.io/curl-runner)
- 💬 [Discussions](https://github.com/alexvcasillas/curl-runner/discussions)

---

<div align="center">
  <strong>curl-runner</strong> - Making HTTP request management simple and powerful
  <br><br>
  Made with ❤️ by the community
</div>
