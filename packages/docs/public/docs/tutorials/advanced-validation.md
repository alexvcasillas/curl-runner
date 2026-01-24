---
title: "Advanced Validation Patterns"
description: "Master complex validation techniques including regex patterns, numeric ranges, array selectors, wildcards, and negative testing scenarios."
category: "Tutorials"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - tutorial
  - validation
  - regex
  - patterns
  - advanced
slug: "/docs/tutorials/advanced-validation"
toc: true
date: "2026-01-24T00:00:00.000Z"
lastModified: "2026-01-24T00:00:00.000Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Advanced Validation"
  category: "Tutorials"
tags:
  - tutorials
  - validation
og:
  title: "Advanced Validation Patterns - curl-runner Documentation"
  description: "Master complex validation techniques including regex, ranges, and wildcards."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Advanced Validation Patterns"
  description: "Master complex validation techniques including regex, ranges, and wildcards."
  datePublished: "2026-01-24T00:00:00.000Z"
  dateModified: "2026-01-24T00:00:00.000Z"
---

# Advanced Validation Patterns

In this tutorial, you'll master advanced validation techniques for complex API testing scenarios.

## Wildcard Matching

Use `*` to accept any value for a field:

```yaml
request:
  name: Get User
  url: https://api.example.com/users/1
  method: GET
  expect:
    status: 200
    body:
      id: 1
      name: "*"           # Any string value
      email: "*"          # Any email value
      createdAt: "*"      # Any timestamp
```

This validates that the fields exist but accepts any value.

## Regex Patterns

Validate values against regular expression patterns:

```yaml
request:
  name: Get User
  url: https://api.example.com/users/1
  method: GET
  expect:
    status: 200
    body:
      id: "^[0-9]+$"                          # Numeric ID
      email: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"  # Email format
      uuid: "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"  # UUID format
      phone: "^\\+?[1-9]\\d{1,14}$"           # E.164 phone format
```

### Common Regex Patterns

```yaml
# Date formats
date: "^\\d{4}-\\d{2}-\\d{2}$"                    # YYYY-MM-DD
datetime: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}"  # ISO 8601

# IDs and codes
numericId: "^[0-9]+$"
alphanumeric: "^[a-zA-Z0-9]+$"
slug: "^[a-z0-9-]+$"

# URLs and paths
url: "^https?://"
path: "^/[a-zA-Z0-9/_-]*$"

# Versions
semver: "^\\d+\\.\\d+\\.\\d+$"
```

## Numeric Ranges

Validate numeric values are within expected ranges:

```yaml
request:
  name: Get Product
  url: https://api.example.com/products/1
  method: GET
  expect:
    status: 200
    body:
      price: "> 0"           # Greater than 0
      stock: ">= 0"          # Greater than or equal to 0
      discount: "<= 100"     # Less than or equal to 100
      rating: "1-5"          # Between 1 and 5 inclusive
      quantity: ">= 1, <= 1000"  # Combined constraints
```

### Range Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `> n` | Greater than | `"> 0"` |
| `>= n` | Greater than or equal | `">= 1"` |
| `< n` | Less than | `"< 100"` |
| `<= n` | Less than or equal | `"<= 50"` |
| `n-m` | Between (inclusive) | `"1-10"` |
| Combined | Multiple constraints | `">= 0, <= 100"` |

## Multiple Accepted Values

Accept one of several possible values:

```yaml
request:
  name: Get Order
  url: https://api.example.com/orders/1
  method: GET
  expect:
    status: 200
    body:
      status: [pending, processing, shipped, delivered]
      priority: [1, 2, 3]
      type: [standard, express, overnight]
```

This validates that the field value matches one of the values in the array.

## Array Validation

### First and Last Elements

```yaml
request:
  name: Get Posts
  url: https://api.example.com/posts
  method: GET
  expect:
    status: 200
    body:
      "[0]":              # First element
        id: "*"
        title: "*"
      "[-1]":             # Last element
        id: "*"
        title: "*"
```

### Specific Index

```yaml
expect:
  body:
    "items[0]":
      name: First Item
    "items[1]":
      name: Second Item
    "items[2]":
      name: Third Item
```

### Array Slice

```yaml
expect:
  body:
    "items.slice(0,3)":   # First 3 elements
      - name: "*"
      - name: "*"
      - name: "*"
```

### All Elements

Validate all elements in an array match a pattern:

```yaml
request:
  name: Get Users
  url: https://api.example.com/users
  method: GET
  expect:
    status: 200
    body:
      "*":                # All elements must match
        id: "^[0-9]+$"
        email: "*"
        active: true
```

## Nested Object Validation

Validate deeply nested structures:

```yaml
request:
  name: Get User Profile
  url: https://api.example.com/users/1/profile
  method: GET
  expect:
    status: 200
    body:
      user:
        id: 1
        profile:
          firstName: "*"
          lastName: "*"
          address:
            street: "*"
            city: "*"
            country: "^[A-Z]{2}$"  # ISO country code
            postalCode: "^[0-9]{5}$"
        settings:
          notifications:
            email: [true, false]
            push: [true, false]
          theme: [light, dark, auto]
```

## Status Code Validation

### Single Status

```yaml
expect:
  status: 200
```

### Multiple Acceptable Statuses

```yaml
expect:
  status: [200, 201]      # Accept either 200 or 201
```

### Range of Statuses

```yaml
expect:
  status: [200, 201, 202, 204]  # Any success without body
```

## Header Validation

```yaml
request:
  name: Check Headers
  url: https://api.example.com/data
  method: GET
  expect:
    status: 200
    headers:
      content-type: application/json; charset=utf-8
      cache-control: "*"              # Any value
      x-request-id: "^[a-f0-9-]+$"   # UUID pattern
```

Note: Header names are case-insensitive.

## Negative Testing

Test that endpoints correctly reject invalid requests:

```yaml
request:
  name: Test Invalid Request
  url: https://api.example.com/users
  method: POST
  headers:
    Content-Type: application/json
  body:
    email: invalid-email  # Invalid format
  expect:
    failure: true         # Expect 4xx or 5xx status
    status: 400
    body:
      error: "*"
      message: "*"
```

### Testing Authentication Failures

```yaml
requests:
  - name: Test Missing Auth
    url: https://api.example.com/protected
    method: GET
    # No Authorization header
    expect:
      failure: true
      status: 401

  - name: Test Invalid Token
    url: https://api.example.com/protected
    method: GET
    headers:
      Authorization: Bearer invalid-token
    expect:
      failure: true
      status: 401

  - name: Test Forbidden Access
    url: https://api.example.com/admin
    method: GET
    headers:
      Authorization: Bearer ${USER_TOKEN}  # Non-admin token
    expect:
      failure: true
      status: 403
```

### Testing Not Found

```yaml
request:
  name: Test Not Found
  url: https://api.example.com/users/99999999
  method: GET
  expect:
    failure: true
    status: 404
    body:
      error: [not_found, "Not Found"]
```

## Combined Validation Example

A comprehensive validation example:

```yaml
global:
  variables:
    BASE_URL: https://api.example.com

request:
  name: Create and Validate Order
  url: ${BASE_URL}/orders
  method: POST
  headers:
    Content-Type: application/json
    Authorization: Bearer ${API_TOKEN}
  body:
    customerId: 123
    items:
      - productId: 1
        quantity: 2
      - productId: 2
        quantity: 1
  expect:
    status: 201
    headers:
      content-type: application/json; charset=utf-8
      x-request-id: "^[0-9a-f-]+$"
    body:
      id: "^[0-9]+$"
      status: pending
      customerId: 123
      total: "> 0"
      currency: [USD, EUR, GBP]
      items:
        "[0]":
          productId: 1
          quantity: 2
          price: "> 0"
        "[1]":
          productId: 2
          quantity: 1
          price: "> 0"
      createdAt: "^\\d{4}-\\d{2}-\\d{2}T"
      updatedAt: "*"
    responseTime: "< 1000"
```

## Partial Validation

You don't need to validate every field. Only specify what matters:

```yaml
# Only validate critical fields, ignore the rest
expect:
  status: 200
  body:
    id: "*"       # Must exist
    status: active # Must be active
    # Other fields in response are ignored
```

## Validation Tips

### 1. Start Simple, Add Complexity

```yaml
# Start with basic validation
expect:
  status: 200

# Then add more specific checks
expect:
  status: 200
  body:
    success: true

# Finally add detailed validation
expect:
  status: 200
  body:
    success: true
    data:
      id: "^[0-9]+$"
```

### 2. Use Wildcards for Dynamic Data

```yaml
body:
  id: "*"              # Generated ID
  createdAt: "*"       # Server timestamp
  updatedAt: "*"       # Server timestamp
  name: "Expected Name" # Known value
```

### 3. Validate Structure, Not Just Values

```yaml
# Ensure the response has the right shape
body:
  user:
    id: "*"
    profile:
      firstName: "*"
      lastName: "*"
```

### 4. Use Negative Tests for Security

```yaml
# Test auth is required
- name: Require Auth
  url: ${BASE_URL}/admin
  expect:
    failure: true
    status: 401
```

## What You Learned

In this tutorial, you learned how to:

- Use wildcards for flexible matching
- Apply regex patterns for format validation
- Validate numeric ranges
- Accept multiple possible values
- Validate arrays with selectors
- Handle nested object validation
- Write negative tests
- Combine validation techniques

## Next Steps

- [Authentication Workflows](/docs/tutorials/authentication-workflows) - Apply validation to auth flows
- [CI/CD Integration](/docs/tutorials/ci-cd-integration) - Automate validation in pipelines
- [YAML Structure](/docs/yaml-structure) - Complete configuration reference
