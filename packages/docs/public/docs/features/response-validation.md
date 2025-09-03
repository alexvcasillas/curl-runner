---
title: "Response Validation"
description: "Validate HTTP responses to ensure your APIs return expected results."
---

# Response Validation

Validate HTTP responses to ensure your APIs return expected results.

## Table of Contents

- [Overview](#overview)
- [Status Code Validation](#status-code-validation)
  - [Common Status Patterns](#common-status-patterns)
- [Header Validation](#header-validation)
- [Body Validation](#body-validation)
  - [Validation Types](#validation-types)
- [Complex Validation](#complex-validation)
- [Collection Validation](#collection-validation)
- [Validation Reference](#validation-reference)
- [Best Practices](#best-practices)

## Overview

## Status Code Validation

```yaml title="status-validation.yaml"
# Validate response status codes
request:
  name: API Health Check
  url: https://api.example.com/health
  method: GET
  expect:
    status: 200  # Expect exactly 200
    
# Multiple accepted status codes
request:
  name: Get User
  url: https://api.example.com/users/123
  method: GET
  expect:
    status: [200, 304]  # Accept either 200 or 304
```

### Common Status Patterns

200, 201, 204

301, 302, 304

400, 401, 403, 404

500, 502, 503

## Header Validation

```yaml title="header-validation.yaml"
# Validate response headers
request:
  name: API Response Headers
  url: https://api.example.com/data
  method: GET
  expect:
    status: 200
    headers:
      content-type: application/json
      x-api-version: "v2"
      cache-control: "no-cache"
```

> **Note:**
>


## Body Validation

```yaml title="body-validation.yaml"
# Validate response body content
request:
  name: Get User Profile
  url: https://api.example.com/users/1
  method: GET
  expect:
    status: 200
    body:
      id: 1
      username: "johndoe"
      email: "john@example.com"
      active: true
      
# Partial matching
request:
  name: Check API Response
  url: https://api.example.com/status
  method: GET
  expect:
    body:
      status: "operational"  # Only check specific fields
```

### Validation Types

> **Exact Match**
>


> **Partial Match**
>


> **Pattern Match**
>


> **Wildcard Match**
>


## Complex Validation

```yaml title="complex-validation.yaml"
# Complex validation scenarios
requests:
  - name: Create Resource
    url: https://api.example.com/resources
    method: POST
    headers:
      Content-Type: application/json
    body:
      name: "Test Resource"
      type: "document"
    expect:
      status: 201
      headers:
        location: "^/resources/[0-9]+$"  # Regex pattern
      body:
        id: "^[0-9]+$"  # Validate format
        name: "Test Resource"
        createdAt: "^\\d{4}-\\d{2}-\\d{2}"  # Date format
        
  - name: Validate Array Response
    url: https://api.example.com/items
    method: GET
    expect:
      status: 200
      body:
        - id: 1
          name: "Item 1"
        - id: 2
          name: "Item 2"
```

## Collection Validation

```yaml title="collection-validation.yaml"
# Validation in collections
global:
  variables:
    BASE_URL: https://api.example.com
  continueOnError: false  # Stop on validation failure

collection:
  name: API Test Suite
  defaults:
    expect:
      headers:
        content-type: application/json  # Default for all requests
  
  requests:
    - name: Login
      url: \${BASE_URL}/auth/login
      method: POST
      body:
        username: "testuser"
        password: "testpass"
      expect:
        status: 200
        body:
          token: "^[A-Za-z0-9-_]+\\.[A-Za-z0-9-_]+\\.[A-Za-z0-9-_]+$"  # JWT format
          
    - name: Get Protected Resource
      url: \${BASE_URL}/protected
      headers:
        Authorization: "Bearer \${token}"
      expect:
        status: 200
        body:
          data: "*"  # Any non-null value
```

## Validation Reference

| Field | Type | Description |
| --- | --- | --- |
| expect.status | number | number[] | Expected HTTP status code(s) |
| expect.headers | object | Expected response headers |
| expect.body | any | Expected response body content |


## Best Practices

> **Start Simple**
>


> **Use Partial Matching**
>


> **Leverage Patterns**
>


> **Group Related Validations**
>


