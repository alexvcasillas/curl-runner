---
title: "Performance Testing with Parallel Execution"
description: "Speed up your test suites by running requests in parallel. Learn when to use parallel vs sequential execution and how to configure performance thresholds."
category: "Tutorials"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - tutorial
  - parallel
  - performance
  - execution
  - speed
slug: "/docs/tutorials/parallel-execution"
toc: true
date: "2026-01-24T00:00:00.000Z"
lastModified: "2026-01-24T00:00:00.000Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Parallel Execution"
  category: "Tutorials"
tags:
  - tutorials
  - performance
og:
  title: "Performance Testing with Parallel Execution - curl-runner Documentation"
  description: "Speed up your test suites by running requests in parallel."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Performance Testing with Parallel Execution"
  description: "Speed up your test suites by running requests in parallel."
  datePublished: "2026-01-24T00:00:00.000Z"
  dateModified: "2026-01-24T00:00:00.000Z"
---

# Performance Testing with Parallel Execution

In this tutorial, you'll learn how to speed up your test suites using parallel execution and validate API performance.

## Sequential vs Parallel Execution

curl-runner supports two execution modes:

**Sequential (default):**
- Requests run one at a time
- Response storage works between requests
- Slower but supports data dependencies

**Parallel:**
- All requests run simultaneously
- Response storage does NOT work
- Much faster for independent tests

## Enabling Parallel Execution

### Via YAML Configuration

```yaml
global:
  execution: parallel

requests:
  - name: Check API Health
    url: https://api.example.com/health
    method: GET
    expect:
      status: 200

  - name: Check User Service
    url: https://api.example.com/users/health
    method: GET
    expect:
      status: 200

  - name: Check Order Service
    url: https://api.example.com/orders/health
    method: GET
    expect:
      status: 200
```

### Via CLI

```bash
curl-runner tests/ --execution parallel
# or shorthand
curl-runner tests/ -p
```

## Performance Validation

Validate that your APIs respond within acceptable time limits:

```yaml
global:
  execution: parallel

requests:
  - name: Fast Endpoint
    url: https://api.example.com/cached-data
    method: GET
    expect:
      status: 200
      responseTime: "< 100"  # Must respond in under 100ms

  - name: Standard Endpoint
    url: https://api.example.com/users
    method: GET
    expect:
      status: 200
      responseTime: "< 500"  # Under 500ms

  - name: Slow Endpoint
    url: https://api.example.com/reports
    method: GET
    expect:
      status: 200
      responseTime: "< 2000"  # Under 2 seconds
```

### Response Time Operators

```yaml
expect:
  responseTime: "< 1000"   # Less than 1000ms
  responseTime: "<= 1000"  # Less than or equal to 1000ms
  responseTime: "> 100"    # Greater than 100ms
  responseTime: ">= 100"   # Greater than or equal to 100ms
```

## Health Check Patterns

### Multi-Service Health Check

Test all your microservices simultaneously:

```yaml
global:
  execution: parallel
  continueOnError: true

requests:
  - name: API Gateway
    url: https://gateway.example.com/health
    method: GET
    expect:
      status: 200
      responseTime: "< 200"

  - name: Auth Service
    url: https://auth.example.com/health
    method: GET
    expect:
      status: 200
      responseTime: "< 200"

  - name: User Service
    url: https://users.example.com/health
    method: GET
    expect:
      status: 200
      responseTime: "< 200"

  - name: Order Service
    url: https://orders.example.com/health
    method: GET
    expect:
      status: 200
      responseTime: "< 200"

  - name: Notification Service
    url: https://notifications.example.com/health
    method: GET
    expect:
      status: 200
      responseTime: "< 200"
```

### Geographic Endpoint Testing

Test the same endpoint from different regions:

```yaml
global:
  execution: parallel
  variables:
    ENDPOINT: /api/v1/products

requests:
  - name: US East
    url: https://us-east.api.example.com${ENDPOINT}
    method: GET
    expect:
      status: 200
      responseTime: "< 300"

  - name: US West
    url: https://us-west.api.example.com${ENDPOINT}
    method: GET
    expect:
      status: 200
      responseTime: "< 300"

  - name: EU
    url: https://eu.api.example.com${ENDPOINT}
    method: GET
    expect:
      status: 200
      responseTime: "< 300"

  - name: Asia Pacific
    url: https://ap.api.example.com${ENDPOINT}
    method: GET
    expect:
      status: 200
      responseTime: "< 300"
```

## Load Testing Patterns

### Concurrent Request Testing

Test how your API handles concurrent requests:

```yaml
global:
  execution: parallel
  variables:
    BASE_URL: https://api.example.com

requests:
  - name: Concurrent Request 1
    url: ${BASE_URL}/products
    method: GET
    expect:
      status: 200

  - name: Concurrent Request 2
    url: ${BASE_URL}/products
    method: GET
    expect:
      status: 200

  - name: Concurrent Request 3
    url: ${BASE_URL}/products
    method: GET
    expect:
      status: 200

  - name: Concurrent Request 4
    url: ${BASE_URL}/products
    method: GET
    expect:
      status: 200

  - name: Concurrent Request 5
    url: ${BASE_URL}/products
    method: GET
    expect:
      status: 200
```

### Mixed Endpoint Load Test

```yaml
global:
  execution: parallel
  continueOnError: true
  variables:
    BASE_URL: https://api.example.com

requests:
  # Read operations (70% of traffic)
  - name: Get Products 1
    url: ${BASE_URL}/products
    method: GET
    expect:
      status: 200

  - name: Get Products 2
    url: ${BASE_URL}/products
    method: GET
    expect:
      status: 200

  - name: Get Products 3
    url: ${BASE_URL}/products
    method: GET
    expect:
      status: 200

  - name: Get Users 1
    url: ${BASE_URL}/users
    method: GET
    expect:
      status: 200

  - name: Get Users 2
    url: ${BASE_URL}/users
    method: GET
    expect:
      status: 200

  - name: Get Orders 1
    url: ${BASE_URL}/orders
    method: GET
    expect:
      status: 200

  - name: Get Orders 2
    url: ${BASE_URL}/orders
    method: GET
    expect:
      status: 200

  # Write operations (30% of traffic)
  - name: Create Order
    url: ${BASE_URL}/orders
    method: POST
    headers:
      Content-Type: application/json
    body:
      productId: 1
      quantity: 1
    expect:
      status: [200, 201]

  - name: Update User
    url: ${BASE_URL}/users/1
    method: PUT
    headers:
      Content-Type: application/json
    body:
      name: Test User
    expect:
      status: 200

  - name: Create Product View
    url: ${BASE_URL}/analytics/views
    method: POST
    headers:
      Content-Type: application/json
    body:
      productId: 1
    expect:
      status: [200, 201, 204]
```

## When to Use Each Mode

### Use Sequential When:

- Requests depend on previous responses
- You need response storage
- Testing authentication flows
- Testing CRUD operations on the same resource

```yaml
global:
  execution: sequential

requests:
  - name: Create User
    url: ${BASE_URL}/users
    method: POST
    body:
      name: John
    store:
      userId: body.id

  - name: Get Created User
    url: ${BASE_URL}/users/${store.userId}  # Depends on previous
    method: GET
```

### Use Parallel When:

- Requests are independent
- Testing health endpoints
- Running smoke tests
- Load testing
- Performance benchmarking

```yaml
global:
  execution: parallel

requests:
  - name: Health Check 1
    url: ${BASE_URL}/health
    method: GET

  - name: Health Check 2
    url: ${BASE_URL}/health
    method: GET
```

## Combining Sequential and Parallel

You can combine both modes by using separate files:

**setup.yaml (sequential):**
```yaml
global:
  execution: sequential

requests:
  - name: Login
    url: ${BASE_URL}/auth/login
    method: POST
    body:
      email: test@example.com
      password: password
    store:
      token: body.token
```

**tests.yaml (parallel):**
```yaml
global:
  execution: parallel
  variables:
    AUTH_TOKEN: ${TOKEN}  # Set via environment

requests:
  - name: Test Endpoint 1
    url: ${BASE_URL}/api/endpoint1
    headers:
      Authorization: Bearer ${AUTH_TOKEN}

  - name: Test Endpoint 2
    url: ${BASE_URL}/api/endpoint2
    headers:
      Authorization: Bearer ${AUTH_TOKEN}
```

Run them in order:

```bash
# Get token from setup, export it, run parallel tests
curl-runner setup.yaml && curl-runner tests.yaml -p
```

## Performance Metrics

View detailed performance metrics with verbose output:

```bash
curl-runner tests/ -p -v --show-metrics
```

This shows:
- DNS lookup time
- TCP connection time
- TLS handshake time
- Time to first byte
- Download time
- Total duration

## Best Practices

### 1. Set Timeouts

Prevent tests from hanging:

```yaml
global:
  defaults:
    timeout: 10000  # 10 seconds
```

### 2. Use Continue on Error

Get complete results even if some fail:

```yaml
global:
  continueOnError: true
```

### 3. Set Reasonable Performance Expectations

```yaml
expect:
  responseTime: "< 1000"  # Be realistic based on your SLAs
```

### 4. Use Retries for Flaky Tests

```yaml
global:
  defaults:
    retry:
      count: 2
      delay: 1000
```

## What You Learned

In this tutorial, you learned how to:

- Enable parallel execution mode
- Validate API response times
- Create health check test suites
- Design load testing patterns
- Choose between sequential and parallel modes
- Combine both modes effectively
- View and analyze performance metrics

## Next Steps

- [Advanced Validation](/docs/tutorials/advanced-validation) - Complex validation patterns
- [CI/CD Integration](/docs/tutorials/ci-cd-integration) - Automate performance tests
- [CLI Options](/docs/cli-options) - All execution options
