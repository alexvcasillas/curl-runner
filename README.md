# 🚀 curl-runner

[![Bun](https://img.shields.io/badge/Bun-%23000000.svg?style=for-the-badge&logo=bun&logoColor=white)](https://bun.sh)
[![TypeScript](https://img.shields.io/badge/typescript-%23007ACC.svg?style=for-the-badge&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Next.js](https://img.shields.io/badge/Next-black?style=for-the-badge&logo=next.js&logoColor=white)](https://nextjs.org/)
[![YAML](https://img.shields.io/badge/yaml-%23ffffff.svg?style=for-the-badge&logo=yaml&logoColor=151515)](https://yaml.org/)
[![MIT License](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)

A powerful CLI tool and documentation website for HTTP request management using YAML configuration files. Built with [Bun](https://bun.sh) for blazing-fast performance and featuring a modern documentation site powered by Next.js.

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

### Prerequisites

- **[Bun](https://bun.sh)** v1.2.21 or higher

### Quick Install

```bash
# Clone the repository
git clone https://github.com/yourusername/curl-runner.git
cd curl-runner

# Install all dependencies
bun install

# Build the CLI tool
bun run build:cli
```

## 🚀 Quick Start

### Basic Usage

```bash
# Run a single YAML file
bun run cli examples/simple.yaml

# Run all YAML files in a directory
bun run cli examples/

# Run with options
bun run cli -pv examples/  # Parallel + Verbose mode
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
bun run cli my-request.yaml
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
│   ├── cli/                 # CLI tool package
│   │   ├── src/            # Source code
│   │   ├── examples/       # Example YAML files
│   │   └── package.json
│   └── docs/               # Documentation website
│       ├── app/            # Next.js app directory
│       ├── components/     # React components
│       └── package.json
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

# Development mode
bun run dev              # Run all packages in dev mode
bun run dev:cli          # CLI only (watch mode)
bun run dev:docs         # Documentation site only

# Building
bun run build            # Build all packages
bun run build:cli        # Build CLI to binary
bun run build:docs       # Build documentation site

# Code quality
bun run format           # Format code with Biome
bun run lint             # Lint code with Biome
bun run check            # Format and fix all issues
bun run check:ci         # CI-friendly check

# Testing
bun test                 # Run all tests
bun test:cli             # Run CLI tests only
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
bun run cli examples/api-auth.yaml

# Load test with parallel requests
bun run cli -p examples/load-test/

# Integration test suite
bun run cli --verbose examples/integration/
```

### Environment-Specific Testing

```bash
# Set environment variables
export API_KEY=your-api-key
export BASE_URL=https://api.staging.example.com

# Run tests with environment variables
bun run cli examples/staging-tests.yaml
```

### Continuous Integration

```bash
# CI-friendly execution with JSON output
bun run cli --quiet --output results.json --continue examples/

# Validate API contract
bun run cli --fail-fast examples/contract-tests/
```

## 🏗️ Architecture

### CLI Package (`@curl-runner/cli`)

- **TypeScript** for type safety
- **Bun** runtime for performance
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

- **[Bun](https://bun.sh)** - The fast all-in-one JavaScript runtime
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
