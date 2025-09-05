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
  - headers
  - response
  - request
  - cli
  - environment
slug: "/docs/output-formats"
toc: true
date: "2025-09-05T06:23:00.140Z"
lastModified: "2025-09-05T06:23:00.140Z"
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
  datePublished: "2025-09-05T06:23:00.140Z"
  dateModified: "2025-09-05T06:23:00.140Z"
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
| `format` | string | json | "json", "pretty", or "raw" |
| `verbose` | boolean | false | Show detailed output |
| `showHeaders` | boolean | false | Include response headers |
| `showBody` | boolean | true | Include response body |
| `showMetrics` | boolean | false | Include performance metrics |
| `saveToFile` | string | - | File path to save results |

**output-config.yaml**

```yaml
# Different output format configurations
global:
  output:
    format: json        # JSON output (default)
    verbose: true       # Show detailed information
    showHeaders: true   # Include response headers
    showBody: true      # Include response body
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

Human-readable format with colors and formatting for terminal viewing.

**pretty-config.yaml**

```yaml
# Pretty format configuration  
global:
  output:
    format: pretty
    verbose: true
    showHeaders: true
    showMetrics: true

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

**pretty-config.yaml**

```yaml
# Pretty format configuration  
global:
  output:
    format: pretty
    verbose: true
    showHeaders: true
    showMetrics: true

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET
```

**terminal**

```text
✅ Get User Profile
   URL: https://api.example.com/users/123
   Method: GET
   Status: 200 OK
   Duration: 125ms
   
   Headers:
   ├─ content-type: application/json
   ├─ x-api-version: v1
   └─ content-length: 256
   
   Response Body:
   {
     "id": 123,
     "name": "John Doe",
     "email": "john@example.com",
     "active": true
   }
   
   Metrics:
   ├─ Request Duration: 125ms
   ├─ Response Size: 256 bytes
   └─ Total Time: 125ms

Summary: 1 request completed successfully
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

# Pretty format (human-readable)
curl-runner tests.yaml --output-format pretty

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
