---
title: "Retry Mechanism"
description: "Automatically retry failed requests with configurable delays and attempts."
---

# Retry Mechanism

Automatically retry failed requests with configurable delays and attempts.

## Table of Contents

- [Overview](#overview)
- [Basic Usage](#basic-usage)
- [Configuration Options](#configuration-options)
- [Advanced Scenarios](#advanced-scenarios)
- [Global Configuration](#global-configuration)
- [Retry with Validation](#retry-with-validation)
- [Retry Strategies](#retry-strategies)
  - [Fixed Delay](#fixed-delay)
  - [Increasing Delay](#increasing-delay)
- [When to Retry](#when-to-retry)
  - [Automatic Retry Conditions](#automatic-retry-conditions)
- [Best Practices](#best-practices)
- [CLI Options](#cli-options)

## Overview

## Basic Usage

```yaml title="basic-retry.yaml"
# Basic retry configuration
request:
  name: Flaky API Endpoint
  url: https://api.example.com/unstable
  method: GET
  retry:
    count: 3      # Retry up to 3 times
    delay: 1000   # Wait 1 second between retries
```

1. First attempt is made immediately
2. If it fails, wait for the specified delay
3. Retry up to the specified count
4. Stop on success or after all retries exhausted

## Configuration Options

| Option | Type | Default | Description |
| --- | --- | --- | --- |
| count | number | 0 | Number of retry attempts |
| delay | number | 1000 | Delay between retries (ms) |


## Advanced Scenarios

```yaml title="advanced-retry.yaml"
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

```yaml title="global-retry.yaml"
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

```yaml title="retry-validation.yaml"
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

```yaml title="fixed-delay.yaml"
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

```yaml title="exponential-backoff.yaml"
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

### Automatic Retry Conditions

> **Network Errors**
>


> **Timeouts**
>


> **5xx Errors**
>


> **Validation Failures**
>


## Best Practices

> **Set Reasonable Limits**
>


> **Use Appropriate Delays**
>


> **Consider Exponential Backoff**
>


> **Monitor Total Time**
>


## CLI Options

```bash title="terminal"
# Override retry count globally
curl-runner api-tests.yaml --retry 5

# Disable all retries
curl-runner api-tests.yaml --no-retry

# Set retry delay
curl-runner api-tests.yaml --retry-delay 2000
```

