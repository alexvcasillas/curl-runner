---
title: "Retry Mechanism"
description: "Automatically retry failed requests with configurable delays and attempts."
category: "Documentation"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - retry
  - mechanism
  - variables
  - validation
  - timeout
  - headers
  - request
  - cli
slug: "/docs/retry-mechanism"
toc: true
date: "2025-09-04T10:22:07.590Z"
lastModified: "2025-09-04T10:22:07.590Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Retry Mechanism"
  category: "Documentation"
tags:
  - documentation
  - documentation
og:
  title: "Retry Mechanism - curl-runner Documentation"
  description: "Automatically retry failed requests with configurable delays and attempts."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Retry Mechanism"
  description: "Automatically retry failed requests with configurable delays and attempts."
  datePublished: "2025-09-04T10:22:07.590Z"
  dateModified: "2025-09-04T10:22:07.590Z"
---

# Retry Mechanism

Automatically retry failed requests with configurable delays and attempts.

## Overview

The retry mechanism in `curl-runner` allows you to automatically retry failed requests, making your API tests more resilient to temporary failures, network issues, and rate limiting.

Automatically retry on network errors, timeouts, and 5xx status codes

Control retry count, delays, and conditions per request or globally

## Basic Usage

Configure retries using the `retry` field in your request configuration.

**basic-retry.yaml**

```yaml
# Basic retry configuration
request:
  name: Flaky API Endpoint
  url: https://api.example.com/unstable
  method: GET
  retry:
    count: 3      # Retry up to 3 times
    delay: 1000   # Wait 1 second between retries
```

## Configuration Options

Fine-tune retry behavior with these configuration options.

## Advanced Scenarios

Handle complex retry requirements with different configurations per request.

**advanced-retry.yaml**

```yaml
# Advanced retry scenarios
requests:
  - name: Critical API Call
    url: https://api.example.com/important
    method: POST
    headers:
      Content-Type: application/json
    body:
      data: "important"
    retry:
      count: 5      # More retries for critical requests
      delay: 2000   # 2 second delay
    timeout: 10000  # 10 second timeout per attempt
    
  - name: Fast Retry
    url: https://api.example.com/quick
    method: GET
    retry:
      count: 10     # Many quick retries
      delay: 100    # Very short delay
    timeout: 1000   # Short timeout
```

## Global Configuration

Set default retry behavior for all requests and override as needed.

**global-retry.yaml**

```yaml
# Global retry configuration
global:
  defaults:
    retry:
      count: 2      # Default retry count for all requests
      delay: 500    # Default delay between retries
  variables:
    API_URL: https://api.example.com

requests:
  - name: Uses Global Retry
    url: \${API_URL}/endpoint1
    method: GET
    # Will use global retry settings
    
  - name: Override Retry
    url: \${API_URL}/endpoint2
    method: GET
    retry:
      count: 5      # Override global setting
      delay: 1000   # Override global delay
      
  - name: No Retry
    url: \${API_URL}/stable
    method: GET
    retry:
      count: 0      # Disable retries for this request
```

## Retry with Validation

Combine retries with validation rules to handle eventually consistent APIs.

**retry-validation.yaml**

```yaml
# Retry with validation rules
request:
  name: Eventually Consistent API
  url: https://api.example.com/resource
  method: GET
  retry:
    count: 5
    delay: 3000   # Wait 3 seconds between attempts
  expect:
    status: 200
    body:
      status: "ready"  # Keep retrying until status is "ready"
      
# Retry on specific status codes
requests:
  - name: Handle Rate Limiting
    url: https://api.example.com/rate-limited
    method: GET
    retry:
      count: 3
      delay: 5000   # Back off for 5 seconds
    expect:
      status: [200, 201]  # Retry if not 200 or 201
```

## Retry Strategies

### Fixed Delay

Use a consistent delay between all retry attempts.

**fixed-delay.yaml**

```yaml
# Fixed delay retry strategy
request:
  name: Reliable API Call
  url: https://api.example.com/data
  method: GET
  retry:
    count: 3      # Retry up to 3 times
    delay: 1000   # Wait exactly 1 second between retries
    
# Will attempt sequence:
# 1. Initial request
# 2. Wait 1s → Retry attempt 1  
# 3. Wait 1s → Retry attempt 2
# 4. Wait 1s → Retry attempt 3
```

### Increasing Delay

Simulate exponential backoff by using multiple requests with increasing delays.

**exponential-backoff.yaml**

```yaml
# Simulating exponential backoff
requests:
  - name: First Attempt
    url: https://api.example.com/endpoint
    retry:
      count: 1
      delay: 1000   # 1 second
      
  - name: Second Attempt (if needed)
    url: https://api.example.com/endpoint
    retry:
      count: 1
      delay: 2000   # 2 seconds
      
  - name: Third Attempt (if needed)
    url: https://api.example.com/endpoint
    retry:
      count: 1
      delay: 4000   # 4 seconds
      
  - name: Fourth Attempt (if needed)
    url: https://api.example.com/endpoint
    retry:
      count: 1
      delay: 8000   # 8 seconds
```

**fixed-delay.yaml**

```yaml
# Fixed delay retry strategy
request:
  name: Reliable API Call
  url: https://api.example.com/data
  method: GET
  retry:
    count: 3      # Retry up to 3 times
    delay: 1000   # Wait exactly 1 second between retries
    
# Will attempt sequence:
# 1. Initial request
# 2. Wait 1s → Retry attempt 1  
# 3. Wait 1s → Retry attempt 2
# 4. Wait 1s → Retry attempt 3
```

**exponential-backoff.yaml**

```yaml
# Simulating exponential backoff
requests:
  - name: First Attempt
    url: https://api.example.com/endpoint
    retry:
      count: 1
      delay: 1000   # 1 second
      
  - name: Second Attempt (if needed)
    url: https://api.example.com/endpoint
    retry:
      count: 1
      delay: 2000   # 2 seconds
      
  - name: Third Attempt (if needed)
    url: https://api.example.com/endpoint
    retry:
      count: 1
      delay: 4000   # 4 seconds
      
  - name: Fourth Attempt (if needed)
    url: https://api.example.com/endpoint
    retry:
      count: 1
      delay: 8000   # 8 seconds
```

## When to Retry

## Best Practices

### Best Practices

• Use descriptive variable names
• Define common values as variables
• Use environment variables for secrets
• Group related variables logically
• Document complex expressions

## CLI Options

Control retry behavior from the command line.

**terminal**

```bash
# Override retry count globally
curl-runner api-tests.yaml --retry 5

# Disable all retries
curl-runner api-tests.yaml --no-retry

# Set retry delay
curl-runner api-tests.yaml --retry-delay 2000
```
