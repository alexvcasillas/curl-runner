# @curl-runner/cli

[![TypeScript](https://img.shields.io/badge/typescript-%23007ACC.svg?style=flat-square&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Bun](https://img.shields.io/badge/Bun-%23000000.svg?style=flat-square&logo=bun&logoColor=white)](https://bun.sh)
[![MIT License](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)

A powerful CLI tool for HTTP request management using YAML configuration files. Built with [Bun](https://bun.sh) for blazing-fast performance and featuring variable interpolation, parallel execution, and response validation.

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

### From Source

```bash
# Clone the monorepo
git clone https://github.com/alexvcasillas/curl-runner.git
cd curl-runner

# Install dependencies
bun install

# Build the CLI
bun run build:cli

# The binary will be available at ./curl-runner
```

### Development Mode

```bash
# Run from source
bun run packages/cli/src/cli.ts [files...] [options]

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
# Run unit tests
bun test

# Run integration tests with example files
curl-runner examples/

# Test CLI functionality
bun run packages/cli/src/cli.ts --help
```

### Development Commands

```bash
# Run in development mode with file watching
bun run dev:cli

# Build the CLI binary
bun run build:cli

# Check code quality
bun run check
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

Built with ❤️ using [Bun](https://bun.sh) and TypeScript