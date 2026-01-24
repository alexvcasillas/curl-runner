---
title: "Output Formats"
description: "Control how curl-runner displays and saves request results."
category: "Documentation"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - output
  - formats
  - yaml
  - variables
  - sequential
  - headers
  - response
  - request
  - cli
  - environment
slug: "/docs/output-formats"
toc: true
date: "2026-01-24T16:04:59.526Z"
lastModified: "2026-01-24T16:04:59.526Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Output Formats"
  category: "Documentation"
tags:
  - documentation
  - documentation
og:
  title: "Output Formats - curl-runner Documentation"
  description: "Control how curl-runner displays and saves request results."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Output Formats"
  description: "Control how curl-runner displays and saves request results."
  datePublished: "2026-01-24T16:04:59.526Z"
  dateModified: "2026-01-24T16:04:59.526Z"
---

# Output Formats

Control how curl-runner displays and saves request results.

## Overview

`curl-runner` provides flexible output options to suit different use cases, from human-readable terminal output to machine-parseable JSON for CI/CD pipelines.

> **Structured Data**
> Machine-readable format for automation

## Output Configuration

Configure output settings in the global section of your YAML file.

### Configuration Options

| Option | Type | Default | Description |
| --- | --- | --- | --- |
| `format` | string | pretty | "json", "pretty", or "raw" |
| `verbose` | boolean | false | Show detailed output |
| `showHeaders` | boolean | false | Include response headers |
| `showBody` | boolean | true | Include response body |
| `showMetrics` | boolean | false | Include performance metrics |
| `prettyLevel` | string | minimal | "minimal", "standard", or "detailed" (for pretty format) |
| `saveToFile` | string | - | File path to save results |

**output-config.yaml**

```yaml
# Different output format configurations
global:
  output:
    format: pretty      # Output format (json, pretty, raw)
    prettyLevel: standard  # Pretty format detail level (minimal, standard, detailed)
    verbose: true       # Show detailed information
    showHeaders: true   # Include response headers
    showBody: true      # Include response body (default: true)
    showMetrics: true   # Include performance metrics

requests:
  - name: API Test
    url: https://api.example.com/data
    method: GET
```

## Format Types

### JSON Format

Structured JSON output ideal for parsing and automation.

**json-config.yaml**

```yaml
# JSON format configuration
global:
  output:
    format: json
    showHeaders: true
    showBody: true
    showMetrics: true

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET
```

### Pretty Format

Human-readable format with colors and tree-structured output for terminal viewing. Three detail levels available: minimal, standard, and detailed.

**pretty-minimal.yaml**

```yaml
# Pretty format - Minimal level
global:
  output:
    format: pretty
    prettyLevel: minimal  # Compact output

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET
```

### Raw Format

Unprocessed response body, useful for binary data or custom formats.

**raw-config.yaml**

```yaml
# Raw format configuration
global:
  output:
    format: raw
    showBody: true

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET
```

**json-config.yaml**

```yaml
# JSON format configuration
global:
  output:
    format: json
    showHeaders: true
    showBody: true
    showMetrics: true

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET
```

**output.json**

```json
{
  "summary": {
    "total": 3,
    "successful": 2,
    "failed": 1,
    "duration": 1234,
    "startTime": "2024-01-15T10:30:00Z",
    "endTime": "2024-01-15T10:30:01Z"
  },
  "results": [
    {
      "request": {
        "name": "Get Users",
        "url": "https://api.example.com/users",
        "method": "GET"
      },
      "success": true,
      "status": 200,
      "headers": {
        "content-type": "application/json",
        "x-api-version": "v1"
      },
      "body": {
        "users": [
          { "id": 1, "name": "John Doe" }
        ]
      },
      "metrics": {
        "duration": 125,
        "size": 256
      }
    },
    {
      "request": {
        "name": "Create User",
        "url": "https://api.example.com/users",
        "method": "POST"
      },
      "success": false,
      "status": 400,
      "error": "Validation failed",
      "metrics": {
        "duration": 89,
        "size": 128
      }
    }
  ]
}
```

**pretty-minimal.yaml**

```yaml
# Pretty format - Minimal level
global:
  output:
    format: pretty
    prettyLevel: minimal  # Compact output

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET
```

**terminal**

```text
ℹ Found 1 YAML file(s)
ℹ Processing: api-test.yaml

✓ Get User Profile [api-test]
   ├─ GET: https://api.example.com/users/123
   ├─ ✓ Status: 200
   └─ Duration: 125ms | 256.00 B

Summary: 1 request completed successfully
```

**pretty-standard.yaml**

```yaml
# Pretty format - Standard level  
global:
  output:
    format: pretty
    prettyLevel: standard  # Balanced detail
    showBody: true

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET
```

**terminal**

```text
ℹ Found 1 YAML file(s)
ℹ Processing: api-test.yaml

Executing 1 request(s) in sequential mode

✓ Get User Profile
   ├─ URL: https://api.example.com/users/123
   ├─ Method: GET
   ├─ Status: 200
   ├─ Duration: 125ms
   └─ Response Body:
      {
        "id": 123,
        "name": "John Doe",
        "email": "john@example.com",
        "active": true
      }


Summary: 1 request completed successfully (125ms)
```

**pretty-detailed.yaml**

```yaml
# Pretty format - Detailed level
global:
  output:
    format: pretty
    prettyLevel: detailed  # Full information
    # Headers and metrics are shown automatically

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET
```

**terminal**

```text
ℹ Found 1 YAML file(s)
ℹ Processing: api-test.yaml

Executing 1 request(s) in sequential mode

  Command:
    curl -X GET -w "\n__CURL_METRICS_START__%{json}__CURL_METRICS_END__" -L -s -S "https://api.example.com/users/123"
✓ Get User Profile
   ├─ URL: https://api.example.com/users/123
   ├─ Method: GET
   ├─ Status: 200
   ├─ Duration: 125ms
   ├─ Response Body:
   │  {
   │    "id": 123,
   │    "name": "John Doe",
   │    "email": "john@example.com",
   │    "active": true
   │  }
   └─ Metrics:
      ├─ Request Duration: 125ms
      ├─ Response Size: 256.00 B
      ├─ DNS Lookup: 5ms
      ├─ TCP Connection: 10ms
      ├─ TLS Handshake: 15ms
      └─ Time to First Byte: 95ms


Summary: 1 request completed successfully (125ms)
```

**raw-config.yaml**

```yaml
# Raw format configuration
global:
  output:
    format: raw
    showBody: true

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET
```

**response.json**

```json
{
  "id": 123,
  "name": "John Doe", 
  "email": "john@example.com",
  "active": true
}
```

## Saving Results

Save test results to files for later analysis or reporting.

**save-to-file.yaml**

```yaml
# Save results to file
global:
  output:
    format: json
    saveToFile: test-results.json  # Save all results to file
    verbose: false                  # Quiet terminal output
  variables:
    API_URL: https://api.example.com

requests:
  - name: Test Suite
    url: \${API_URL}/test
    method: GET
```

## CLI Options

Control output format from the command line.

### Output Flags

| Flag | Description |
| --- | --- |
| `--output-format` | Set output format (json/pretty/raw) |
| `--pretty-level` | Set pretty format detail level (minimal/standard/detailed) |
| `--output, -o` | Save results to file |
| `--verbose, -v` | Enable verbose output |
| `--quiet, -q` | Suppress non-error output |
| `--show-headers` | Include response headers |
| `--show-body` | Include response body |
| `--show-metrics` | Include performance metrics |

**terminal**

```bash
# Command line output options
# JSON format (machine-readable)
curl-runner tests.yaml --output-format json

# Pretty format with different detail levels
curl-runner tests.yaml --output-format pretty --pretty-level minimal
curl-runner tests.yaml --output-format pretty --pretty-level standard
curl-runner tests.yaml --output-format pretty --pretty-level detailed

# Raw format (response body only)
curl-runner tests.yaml --output-format raw

# Save to file
curl-runner tests.yaml --output results.json

# Verbose mode
curl-runner tests.yaml --verbose

# Show specific components
curl-runner tests.yaml --show-headers --show-metrics

# Quiet mode (errors only)
curl-runner tests.yaml --quiet
```

## Use Cases

## Best Practices

### Best Practices

• Use descriptive variable names
• Define common values as variables
• Use environment variables for secrets
• Group related variables logically
• Document complex expressions
