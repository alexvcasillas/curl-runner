---
title: "Response Validation"
description: "Validate HTTP responses to ensure your APIs return expected results."
category: "Documentation"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - response
  - validation
  - variables
  - headers
  - request
  - collection
slug: "/docs/response-validation"
toc: true
date: "2026-01-24T16:03:18.279Z"
lastModified: "2026-01-24T16:03:18.279Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Response Validation"
  category: "Documentation"
tags:
  - documentation
  - documentation
og:
  title: "Response Validation - curl-runner Documentation"
  description: "Validate HTTP responses to ensure your APIs return expected results."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Response Validation"
  description: "Validate HTTP responses to ensure your APIs return expected results."
  datePublished: "2026-01-24T16:03:18.279Z"
  dateModified: "2026-01-24T16:03:18.279Z"
---

# Response Validation

Validate HTTP responses to ensure your APIs return expected results.

## Overview

Response validation allows you to verify that your API responses meet expected criteria. curl-runner can validate status codes, headers, and response body content, making it perfect for API testing and monitoring.

## Status Code Validation

Validate that responses return expected HTTP status codes.

200, 201, 204

301, 302, 304

400, 401, 403, 404

500, 502, 503

**status-validation.yaml**

```yaml
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

## Header Validation

Verify that responses include expected headers with correct values.

**header-validation.yaml**

```yaml
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

## Body Validation

Validate response body content for JSON, text, or other formats.

**body-validation.yaml**

```yaml
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

## Complex Validation

Combine multiple validation rules for comprehensive testing.

**complex-validation.yaml**

```yaml
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

Apply validation rules across collections with defaults and overrides.

**collection-validation.yaml**

```yaml
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

## Best Practices

### Best Practices

• Use descriptive variable names
• Define common values as variables
• Use environment variables for secrets
• Group related variables logically
• Document complex expressions
