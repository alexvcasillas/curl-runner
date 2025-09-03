---
title: "Parallel Execution"
description: "Execute multiple HTTP requests simultaneously for improved performance."
---

# Parallel Execution

Execute multiple HTTP requests simultaneously for improved performance.

## Table of Contents

- [Overview](#overview)
- [Basic Usage](#basic-usage)
- [Configuration Options](#configuration-options)
- [Error Handling](#error-handling)
- [Performance Testing](#performance-testing)
- [CLI Usage](#cli-usage)
  - [CLI Flags](#cli-flags)
- [Best Practices](#best-practices)

## Overview

## Basic Usage

```yaml title="parallel-requests.yaml"
# Run multiple requests in parallel
global:
  execution: parallel
  variables:
    API_URL: https://api.example.com
  output:
    showMetrics: true

requests:
  - name: Get Users
    url: \${API_URL}/users
    method: GET
    
  - name: Get Posts
    url: \${API_URL}/posts
    method: GET
    
  - name: Get Comments
    url: \${API_URL}/comments
    method: GET
    
  - name: Get Products
    url: \${API_URL}/products
    method: GET
```

## Configuration Options

| Setting | Type | Description |
| --- | --- | --- |
| execution | string | "parallel" or "sequential" (default: "sequential") |
| continueOnError | boolean | Continue executing remaining requests if one fails |
| timeout | number | Global timeout for each request in milliseconds |


## Error Handling

```yaml title="parallel-with-error-handling.yaml"
# Control parallel execution
global:
  execution: parallel
  continueOnError: true  # Continue even if some requests fail
  variables:
    BASE_URL: https://api.example.com
  output:
    verbose: true
    showMetrics: true

requests:
  # These will all run at the same time
  - name: Request 1
    url: \${BASE_URL}/endpoint1
    timeout: 5000
    
  - name: Request 2
    url: \${BASE_URL}/endpoint2
    timeout: 5000
    
  - name: Request 3
    url: \${BASE_URL}/endpoint3
    timeout: 5000
```

> Stop on Error
>
> By default, if any request fails, curl-runner will cancel remaining requests

> Continue on Error Recommended
>
> Set {"continueOnError: true"} to complete all requests regardless of failures

## Performance Testing

```yaml title="performance-test.yaml"
# Performance testing with parallel requests
global:
  execution: parallel
  variables:
    API_URL: https://api.example.com
    AUTH_TOKEN: your-token-here
  output:
    showMetrics: true
    saveToFile: performance-results.json

# Test multiple endpoints simultaneously
requests:
  - name: Health Check 1
    url: \${API_URL}/health
    
  - name: Health Check 2
    url: \${API_URL}/health
    
  - name: Health Check 3
    url: \${API_URL}/health
    
  - name: Health Check 4
    url: \${API_URL}/health
    
  - name: Health Check 5
    url: \${API_URL}/health
```

- • Total execution time for all requests
- • Individual request duration
- • Response sizes
- • Success/failure counts
- • Average response time

## CLI Usage

```bash title="terminal"
# Run with parallel execution
curl-runner api-tests.yaml --parallel

# Override file setting to run sequentially
curl-runner api-tests.yaml --sequential

# Run parallel with verbose output
curl-runner api-tests.yaml --parallel --verbose
```

### CLI Flags

| Flag | Description |
| --- | --- |
| --parallel | Force parallel execution |
| --sequential | Force sequential execution |
| --continue-on-error | Continue execution on failures |


## Best Practices

> **Use for Independent Requests**
>


> **Set Appropriate Timeouts**
>


> **Monitor Resource Usage**
>


> **Use Metrics for Analysis**
>


