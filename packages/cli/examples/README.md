# Curl Runner Examples

This directory contains comprehensive examples showcasing all features of curl-runner CLI.

## Basic Request Examples

### HTTP Methods
- **`basic-get.yaml`** - Simple GET request
- **`basic-post.yaml`** - POST request with JSON body
- **`basic-put.yaml`** - PUT request for updates
- **`basic-delete.yaml`** - DELETE request
- **`multiple-requests.yaml`** - Multiple requests in sequence

### Usage
```bash
# Run individual examples
curl-runner basic-get.yaml
curl-runner basic-post.yaml

# Run multiple files
curl-runner basic-*.yaml
```

## Authentication Examples

### Types
- **`auth-basic.yaml`** - HTTP Basic authentication
- **`auth-bearer.yaml`** - Bearer token authentication
- **`auth-headers.yaml`** - Custom header authentication

### Usage
```bash
curl-runner auth-basic.yaml
curl-runner auth-bearer.yaml
curl-runner auth-headers.yaml
```

## File Upload Examples

### Types
- **`file-upload.yaml`** - Single file upload with various options
- **`file-upload-multi.yaml`** - Multiple file uploads with form fields
- **`sample-file.txt`** - Sample file for testing uploads

### Usage
```bash
# Basic file upload
curl-runner file-upload.yaml

# Multiple files
curl-runner file-upload-multi.yaml
```

### File Upload Configuration

Use `formData` instead of `body` to send multipart/form-data requests:

```yaml
request:
  name: Upload Document
  url: https://api.example.com/upload
  method: POST
  formData:
    # Simple form fields
    title: "My Document"
    description: "Test upload"

    # File attachment (basic)
    document:
      file: "./report.pdf"

    # File with custom filename
    avatar:
      file: "./photo.jpg"
      filename: "profile-picture.jpg"

    # File with explicit content type
    data:
      file: "./data.bin"
      filename: "export.json"
      contentType: "application/json"
```

### File Attachment Properties

| Property | Required | Description |
|----------|----------|-------------|
| `file` | Yes | Path to the file (relative or absolute) |
| `filename` | No | Custom filename sent to server |
| `contentType` | No | MIME type (auto-detected if not specified) |

## Retry and Timeout Examples

### Features
- **`retry-basic.yaml`** - Basic retry configuration
- **`timeout-basic.yaml`** - Request timeout settings
- **`timeout-with-retry.yaml`** - Combined timeout and retry
- **`test-with-retry.yaml`** - Advanced retry scenarios

### Usage
```bash
curl-runner retry-basic.yaml
curl-runner timeout-basic.yaml
curl-runner timeout-with-retry.yaml
```

## Validation Examples

### Types
- **`validation-status.yaml`** - HTTP status code validation
- **`validation-body.yaml`** - Response body field validation
- **`validation-headers.yaml`** - Response header validation
- **`validation-failing.yaml`** - Examples of validation failures
- **`expect-failure.yaml`** - Test expected failures (negative testing)
- **`expect-failure-mixed.yaml`** - Mixed success and failure expectations

### Usage
```bash
curl-runner validation-status.yaml
curl-runner validation-body.yaml
curl-runner validation-failing.yaml

# Test expected failures (negative testing)
curl-runner expect-failure.yaml
curl-runner expect-failure-mixed.yaml
```

### Failure Testing (Negative Testing)

The `expect.failure: true` option allows you to test that endpoints correctly fail in expected ways:

```yaml
request:
  name: Test 404 Error
  url: https://api.example.com/nonexistent
  method: GET
  expect:
    failure: true  # Expect this request to fail
    status: 404    # Should return 404 Not Found
    body:
      error: "Resource not found"
```

**How it works:**
- `expect.failure: true` + 4xx/5xx status + validations pass = ✅ **SUCCESS**
- `expect.failure: true` + 2xx/3xx status = ❌ **FAILED** (expected failure but got success)
- `expect.failure: true` + 4xx/5xx status + validations fail = ❌ **FAILED** (wrong error details)

## Output Format Examples

### Pretty Format Levels
- **`pretty-minimal.yaml`** - Minimal output (status only)
- **`pretty-standard.yaml`** - Standard output (body + metrics when enabled)
- **`pretty-detailed.yaml`** - Detailed output (all information)

### Format Types
- **`output-formats.yaml`** - Test different output formats
- **`save-to-file.yaml`** - Save results to JSON file

### Usage
```bash
# Different pretty levels
curl-runner pretty-minimal.yaml
curl-runner pretty-standard.yaml  
curl-runner pretty-detailed.yaml

# Different format types
curl-runner output-formats.yaml --output-format json
curl-runner output-formats.yaml --output-format raw
curl-runner output-formats.yaml --pretty-level minimal

# Save to file
curl-runner save-to-file.yaml
```

## Execution Mode Examples

### Types
- **`execution-sequential.yaml`** - Sequential execution (default)
- **`execution-parallel.yaml`** - Parallel execution
- **`parallel.yaml`** - Advanced parallel example
- **`execution-continue-on-error.yaml`** - Continue when requests fail

### Usage
```bash
# Sequential vs parallel
curl-runner execution-sequential.yaml
curl-runner execution-parallel.yaml

# CLI override
curl-runner execution-sequential.yaml --execution parallel

# Continue on error
curl-runner execution-continue-on-error.yaml
```

## Response Storage (Chaining Requests)

Store response values from one request to use in subsequent requests. This is useful for:
- Authentication flows (store tokens for later use)
- CRUD workflows (create resource, then use the ID to update/delete)
- Data pipelines (extract data from one API to send to another)

### Examples
- **`response-storage.yaml`** - Basic response storage example
- **`response-storage-auth-flow.yaml`** - Authentication workflow example

### Usage
```bash
# Basic storage example
curl-runner response-storage.yaml

# Authentication flow
curl-runner response-storage-auth-flow.yaml
```

### Store Configuration

Use the `store` property in a request to extract values from the response:

```yaml
requests:
  # First request: Create a resource
  - name: Create User
    url: https://api.example.com/users
    method: POST
    body:
      name: "John Doe"
      email: "john@example.com"
    store:
      userId: body.id              # Store response body's id field
      authToken: body.token        # Store nested field
      contentType: headers.content-type  # Store response header
      statusCode: status           # Store HTTP status code

  # Second request: Use stored values
  - name: Get User
    url: https://api.example.com/users/${store.userId}
    method: GET
    headers:
      Authorization: Bearer ${store.authToken}
```

### Supported Paths

The `store` configuration supports dot-notation paths to extract values:

| Path | Description | Example |
|------|-------------|---------|
| `body.field` | Top-level body field | `body.id` |
| `body.nested.field` | Nested body field | `body.data.token` |
| `body.array.0.field` | Array element by index | `body.items.0.id` |
| `headers.name` | Response header | `headers.content-type` |
| `status` | HTTP status code | `status` |
| `metrics.duration` | Response metrics | `metrics.duration` |

### Important Notes

- **Sequential execution required**: Response storage only works in sequential mode (the default). In parallel mode, request order is not guaranteed.
- **Stored values are strings**: All stored values are converted to strings. Objects/arrays are JSON stringified.
- **Scope**: Stored values persist for all subsequent requests in the same execution run.
- **Variable syntax**: Use `${store.variableName}` to reference stored values.

## Variables and Configuration

### Examples
- **`variables-basic.yaml`** - Variable substitution
- **`dynamic-variables.yaml`** - Dynamic variables (UUID, timestamp, etc.)
- **`comprehensive.yaml`** - All features combined
- **`cli-options.yaml`** - Testing CLI options
- **`simple.yaml`** - Simple test case

### Usage
```bash
# Variables
curl-runner variables-basic.yaml

# Dynamic variables
curl-runner dynamic-variables.yaml

# CLI options
curl-runner cli-options.yaml --verbose --show-headers
curl-runner cli-options.yaml --timeout 5000 --retries 3

# Comprehensive example
curl-runner comprehensive.yaml
```

## Global Configuration

The root `curl-runner.yaml` file sets global defaults that can be overridden by individual files.

## CLI Options Reference

### Output Control
```bash
--output-format <format>     # json, pretty, raw
--pretty-level <level>       # minimal, standard, detailed
--show-headers              # Include response headers
--show-body                 # Include response body  
--show-metrics             # Include timing metrics
--verbose                  # Enable verbose output
--quiet                   # Suppress output
--output <file>           # Save to file
```

### Execution Control
```bash
--execution parallel       # Run requests in parallel
--continue-on-error       # Don't stop on failures
--timeout <ms>           # Request timeout
--retries <count>        # Max retry attempts
--retry-delay <ms>       # Delay between retries
--no-retry              # Disable retries
```

### File Discovery
```bash
--all                   # Find files recursively
```

## Environment Variables

Set defaults via environment variables:
```bash
export CURL_RUNNER_OUTPUT_FORMAT=json
export CURL_RUNNER_PRETTY_LEVEL=detailed
export CURL_RUNNER_VERBOSE=true
export CURL_RUNNER_CONTINUE_ON_ERROR=true
```

## Configuration Precedence

Settings are applied in this order (later overrides earlier):
1. Default values
2. Environment variables
3. Global `curl-runner.yaml`
4. Individual YAML file
5. CLI arguments

## Testing All Examples

```bash
# Test basic functionality
curl-runner basic-*.yaml

# Test all features
curl-runner comprehensive.yaml

# Test output formats
curl-runner output-formats.yaml --output-format json
curl-runner pretty-*.yaml

# Test validation
curl-runner validation-*.yaml

# Test execution modes
curl-runner execution-*.yaml

# Test authentication
curl-runner auth-*.yaml

# Test response storage (chaining)
curl-runner response-storage.yaml
curl-runner response-storage-auth-flow.yaml
```