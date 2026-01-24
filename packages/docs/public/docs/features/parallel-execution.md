---
title: "Parallel Execution"
description: "Execute multiple HTTP requests simultaneously for improved performance."
category: "Documentation"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - parallel
  - execution
  - yaml
  - variables
  - sequential
  - timeout
  - response
  - request
  - cli
slug: "/docs/parallel-execution"
toc: true
date: "2026-01-24T16:04:59.529Z"
lastModified: "2026-01-24T16:04:59.529Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Parallel Execution"
  category: "Documentation"
tags:
  - documentation
  - documentation
og:
  title: "Parallel Execution - curl-runner Documentation"
  description: "Execute multiple HTTP requests simultaneously for improved performance."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Parallel Execution"
  description: "Execute multiple HTTP requests simultaneously for improved performance."
  datePublished: "2026-01-24T16:04:59.529Z"
  dateModified: "2026-01-24T16:04:59.529Z"
---

# Parallel Execution

Execute multiple HTTP requests simultaneously for improved performance.

## Overview

Parallel execution allows curl-runner to send multiple HTTP requests simultaneously, significantly reducing the total execution time when testing multiple endpoints or performing load testing.

## Basic Usage

Configure parallel execution in your YAML file using the global execution setting.

**parallel-requests.yaml**

```yaml
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

Control how parallel execution behaves with these settings:

## Error Handling

When running requests in parallel, you can control how errors are handled using the `continueOnError` setting.

**parallel-with-error-handling.yaml**

```yaml
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

## Performance Testing

Parallel execution is ideal for performance testing and load simulation. Use it to test how your API handles concurrent requests.

**performance-test.yaml**

```yaml
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

## CLI Usage

Control parallel execution from the command line.

### CLI Flags

| Flag | Description |
| --- | --- |
| `--parallel` | Force parallel execution |
| `--sequential` | Force sequential execution |
| `--continue-on-error` | Continue execution on failures |

**terminal**

```bash
# Run with parallel execution
curl-runner api-tests.yaml --parallel

# Override file setting to run sequentially
curl-runner api-tests.yaml --sequential

# Run parallel with verbose output
curl-runner api-tests.yaml --parallel --verbose
```

## Best Practices

### Best Practices

• Use descriptive variable names
• Define common values as variables
• Use environment variables for secrets
• Group related variables logically
• Document complex expressions
