---
title: "Output Formats"
description: "Control how curl-runner displays and saves request results."
---

# Output Formats

Control how curl-runner displays and saves request results.

## Table of Contents

- [Overview](#overview)
- [Output Configuration](#output-configuration)
  - [Configuration Options](#configuration-options)
- [Format Types](#format-types)
  - [JSON Format](#json-format)
  - [Pretty Format](#pretty-format)
  - [Raw Format](#raw-format)
- [Saving Results](#saving-results)
- [CLI Options](#cli-options)
  - [Output Flags](#output-flags)
- [Use Cases](#use-cases)
- [Best Practices](#best-practices)

## Overview

## Output Configuration

```yaml title="output-config.yaml"
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

### Configuration Options

| Option | Type | Default | Description |
| --- | --- | --- | --- |
| format | string | json | "json", "pretty", or "raw" |
| verbose | boolean | false | Show detailed output |
| showHeaders | boolean | false | Include response headers |
| showBody | boolean | true | Include response body |
| showMetrics | boolean | false | Include performance metrics |
| saveToFile | string | - | File path to save results |


## Format Types

### JSON Format

```yaml title="json-config.yaml"
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

```json title="output.json"
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

### Pretty Format

```yaml title="pretty-config.yaml"
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

```text title="terminal"
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

### Raw Format

```yaml title="raw-config.yaml"
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

```json title="response.json"
{
  "id": 123,
  "name": "John Doe", 
  "email": "john@example.com",
  "active": true
}
```

## Saving Results

```yaml title="save-to-file.yaml"
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

> **Tip:**
>


## CLI Options

```bash title="terminal"
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

### Output Flags

| Flag | Description |
| --- | --- |
| --output-format | Set output format (json/pretty/raw) |
| --output, -o | Save results to file |
| --verbose, -v | Enable verbose output |
| --quiet, -q | Suppress non-error output |
| --show-headers | Include response headers |
| --show-body | Include response body |
| --show-metrics | Include performance metrics |


## Use Cases

> **Continuous Integration**
>


> **Local Testing**
>


> **Health Checks & Monitoring**
>


## Best Practices

> **Choose the Right Format**
>


> **Save Important Results**
>


> **Use Quiet Mode in Scripts**
>


> **Enable Metrics for Performance Testing**
>


