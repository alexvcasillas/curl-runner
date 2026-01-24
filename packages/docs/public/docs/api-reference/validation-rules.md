---
title: "Validation Rules API Reference"
description: "Complete reference for response validation using the expect configuration object. Define validation rules for status codes, headers, and response body content."
category: "Documentation"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - validation
  - rules
  - reference
  - yaml
  - authentication
  - headers
  - response
  - request
  - collection
  - environment
slug: "/docs/validation-rules"
toc: true
date: "2026-01-24T15:29:16.943Z"
lastModified: "2026-01-24T15:29:16.943Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Validation Rules API Reference"
  category: "Documentation"
tags:
  - documentation
  - documentation
og:
  title: "Validation Rules API Reference - curl-runner Documentation"
  description: "Complete reference for response validation using the expect configuration object. Define validation rules for status codes, headers, and response body content."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Validation Rules API Reference"
  description: "Complete reference for response validation using the expect configuration object. Define validation rules for status codes, headers, and response body content."
  datePublished: "2026-01-24T15:29:16.943Z"
  dateModified: "2026-01-24T15:29:16.943Z"
---

# Validation Rules API Reference

Complete reference for response validation using the expect configuration object. Define validation rules for status codes, headers, and response body content.

## Overview

Validation rules allow you to verify that API responses meet expected criteria. Using the `expect` configuration object, you can validate HTTP status codes, response headers, and body content with support for exact matching, pattern matching, and wildcard validation.

**expect-config-complete.yaml**

```yaml
# Complete ExpectConfig Example
request:
  name: "Comprehensive Validation Example"
  url: "https://api.example.com/users/123"
  method: GET
  expect:
    # Failure expectation (optional - for negative testing)
    failure: false    # false = expect success (default), true = expect failure
    
    # Status code validation
    status: 200
    
    # Response headers validation
    headers:
      content-type: "application/json"
      x-api-version: "v2"
      cache-control: "no-cache, no-store"
      etag: "^W/\\"[a-f0-9]+\\""  # Regex pattern for ETag
      
    # Response body validation
    body:
      id: 123
      username: "johndoe"
      email: "john@example.com"
      profile:
        firstName: "John"
        lastName: "Doe"
        createdAt: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$"  # ISO date
        active: true
        roles: ["user", "premium"]  # Array with specific values
        metadata:
          lastLogin: "*"  # Any value (wildcard)
          preferences: "*"  # Any object structure
```

## ExpectConfig Properties

## Status Code Validation

Validate that responses return expected HTTP status codes. You can specify a single status code or an array of acceptable codes.

**status-validation.yaml**

```yaml
# Status Code Validation Examples

# Single status code (most common)
request:
  name: "Expect Success"
  url: "https://api.example.com/data"
  expect:
    status: 200

---

# Multiple acceptable status codes
request:
  name: "Accept Multiple Status"
  url: "https://api.example.com/users/123"
  method: GET
  expect:
    status: [200, 304]  # OK or Not Modified

---

# Different scenarios
requests:
  # Successful creation
  - name: "Create Resource"
    url: "https://api.example.com/resources"
    method: POST
    body:
      name: "New Resource"
    expect:
      status: 201  # Created
      
  # Successful deletion
  - name: "Delete Resource"
    url: "https://api.example.com/resources/123"
    method: DELETE
    expect:
      status: 204  # No Content
      
  # Conditional request
  - name: "Conditional Get"
    url: "https://api.example.com/data"
    method: GET
    headers:
      If-None-Match: "some-etag"
    expect:
      status: [200, 304]  # OK or Not Modified
      
  # Error handling
  - name: "Expected Not Found"
    url: "https://api.example.com/users/nonexistent"
    method: GET
    expect:
      status: 404  # Not Found is expected
      
  # Rate limiting tolerance
  - name: "Rate Limited Request"
    url: "https://api.example.com/limited-endpoint"
    method: GET
    expect:
      status: [200, 429]  # OK or Too Many Requests
```

## Failure Testing (Negative Testing)

Use `expect.failure: true` to test that endpoints correctly fail in expected ways. This is useful for testing error handling, authentication failures, validation errors, and other scenarios where you expect the request to return a 4xx or 5xx status code.

> **✅ Success:** `expect.failure: true` + 4xx/5xx status + validations pass
> **❌ Failed:** `expect.failure: true` + 2xx/3xx status (expected failure but got success)
> **❌ Failed:** `expect.failure: true` + 4xx/5xx status + validations fail (wrong error details)

**failure-testing.yaml**

```yaml
# Failure Testing Examples

# Test authentication failure
request:
  name: "Test Invalid Authentication"
  url: "https://api.example.com/protected-resource"
  method: GET
  auth:
    type: bearer
    token: "invalid-token"
  expect:
    failure: true    # Expect this request to fail
    status: 401      # Should return Unauthorized
    body:
      error: "Invalid token"
      code: "AUTH_ERROR"

---

# Test validation failure
request:
  name: "Test Invalid Input"
  url: "https://api.example.com/users"
  method: POST
  body:
    name: ""         # Invalid: empty name
    email: "invalid" # Invalid: malformed email
  expect:
    failure: true    # Expect validation to fail
    status: 400      # Should return Bad Request
    body:
      error: "Validation failed"
      details:
        name: "Name is required"
        email: "Invalid email format"

---

# Test resource not found
request:
  name: "Test Non-existent Resource"
  url: "https://api.example.com/users/999999"
  method: GET
  expect:
    failure: true    # Expect resource not found
    status: 404      # Should return Not Found
    body:
      error: "User not found"
      
---

# Test rate limiting
request:
  name: "Test Rate Limit Exceeded"  
  url: "https://api.example.com/limited-endpoint"
  method: GET
  expect:
    failure: true    # Expect rate limit error
    status: 429      # Should return Too Many Requests
    headers:
      retry-after: "*"  # Should include retry information
      
---

# Mixed success and failure testing
requests:
  # Normal success case
  - name: "Valid Request"
    url: "https://api.example.com/data"
    method: GET
    expect:
      status: 200    # Normal success expectation
      
  # Test failure case  
  - name: "Invalid Request"
    url: "https://api.example.com/data"
    method: GET
    headers:
      authorization: "invalid"
    expect:
      failure: true  # Expect this to fail
      status: 403    # Should be Forbidden
```

## Header Validation

Validate response headers using exact values or regular expression patterns. Header names are case-insensitive.

**header-validation.yaml**

```yaml
# Header Validation Examples

# Basic header validation
request:
  name: "Basic Header Check"
  url: "https://api.example.com/data"
  expect:
    headers:
      content-type: "application/json"
      server: "nginx"
      
---

# Case-insensitive header matching
request:
  name: "Case Insensitive Headers"
  url: "https://api.example.com/data"
  expect:
    headers:
      Content-Type: "application/json"      # Works
      content-type: "application/json"      # Also works
      CONTENT-TYPE: "application/json"      # Also works
      
---

# Pattern matching with regex
request:
  name: "Pattern Matching Headers"
  url: "https://api.example.com/data"
  expect:
    headers:
      # Date header pattern
      date: "^[A-Z][a-z]{2}, \\d{2} [A-Z][a-z]{2} \\d{4}"
      
      # ETag pattern
      etag: "^(W/)?\\"[a-f0-9-]+\\""
      
      # Rate limit patterns
      x-rate-limit-limit: "^[0-9]+$"
      x-rate-limit-remaining: "^[0-9]+$"
      
      # Version patterns
      x-api-version: "^v[0-9]+(\\.0-9]+)*$"
      
---

# Security headers validation
request:
  name: "Security Headers Check"
  url: "https://api.secure.com/data"
  expect:
    headers:
      strict-transport-security: "max-age=31536000"
      x-frame-options: "DENY"
      x-content-type-options: "nosniff"
      x-xss-protection: "1; mode=block"
      content-security-policy: "*"  # Any CSP policy present
      
---

# API-specific headers
request:
  name: "API Headers Validation"
  url: "https://api.example.com/data"
  expect:
    headers:
      # Rate limiting info
      x-rate-limit-remaining: "*"
      x-rate-limit-reset: "^[0-9]+$"
      
      # Request tracking
      x-request-id: "^[a-f0-9-]{36}$"  # UUID format
      
      # Performance metrics
      x-response-time: "^[0-9]+ms$"
      
      # Location header for redirects/created resources
      location: "^https://api\\.example\\.com/resources/[0-9]+$"
```

## Body Validation

Validate response body content with support for exact matching, partial matching, and complex nested structures.

**body-validation.yaml**

```yaml
# Body Validation Examples

# Exact value matching
request:
  name: "Exact Value Match"
  url: "https://api.example.com/status"
  expect:
    body:
      status: "operational"
      code: 200
      message: "All systems operational"
      
---

# Object structure validation
request:
  name: "Object Structure Validation"
  url: "https://api.example.com/users/123"
  expect:
    body:
      id: 123
      username: "johndoe"
      profile:
        firstName: "John"
        lastName: "Doe"
        email: "john@example.com"
        settings:
          theme: "dark"
          notifications: true
          
---

# Array validation
request:
  name: "Array Validation"
  url: "https://api.example.com/users"
  expect:
    body:
      # Validate array structure
      - id: 1
        username: "user1"
        active: true
      - id: 2
        username: "user2"
        active: true
        
---

# Partial matching (only check specific fields)
request:
  name: "Partial Matching"
  url: "https://api.example.com/users/123"
  expect:
    body:
      id: 123  # Only check ID
      active: true  # Only check active status
      # Other fields ignored
      
---

# Mixed data types
request:
  name: "Mixed Data Types"
  url: "https://api.example.com/metrics"
  expect:
    body:
      # String values
      version: "2.1.0"
      environment: "production"
      
      # Numeric values
      uptime: 86400
      cpu_usage: 0.75
      
      # Boolean values
      healthy: true
      maintenance_mode: false
      
      # Null values
      last_error: null
      
      # Array of strings
      tags: ["api", "production", "monitoring"]
      
      # Array of numbers
      response_times: [120, 95, 200, 150]
```

## Pattern Validation with Regex

Use regular expressions for flexible validation of dynamic content like IDs, timestamps, emails, and custom formats.

**pattern-validation.yaml**

```yaml
# Pattern Validation with Regex

# Common patterns
request:
  name: "Common Pattern Validation"
  url: "https://api.example.com/user/profile"
  expect:
    body:
      # Email pattern
      email: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
      
      # Phone number pattern (US format)
      phone: "^\\+?1?[-\\s]?\\(?[0-9]{3}\\)?[-\\s]?[0-9]{3}[-\\s]?[0-9]{4}$"
      
      # UUID pattern
      user_id: "^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$"
      
      # ISO date pattern
      created_at: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}(\\.\\d{3})?Z$"
      
      # URL pattern
      avatar_url: "^https?://[\\w.-]+\\.[a-zA-Z]{2,}(/.*)?$"
      
---

# Numeric patterns
request:
  name: "Numeric Pattern Validation"
  url: "https://api.example.com/financial/account"
  expect:
    body:
      # Currency amounts (dollars and cents)
      balance: "^[0-9]+\\.[0-9]{2}$"
      
      # Percentage values
      interest_rate: "^[0-9]{1,2}\\.[0-9]{2}%$"
      
      # Account numbers
      account_number: "^[0-9]{10,12}$"
      
      # Credit card last 4 digits
      card_last_four: "^[0-9]{4}$"
      
---

# Custom business patterns
request:
  name: "Business Pattern Validation"
  url: "https://api.example.com/orders/12345"
  expect:
    body:
      # Order ID pattern
      order_id: "^ORD-[0-9]{8}$"
      
      # Product SKU pattern
      sku: "^[A-Z]{3}-[0-9]{4}-[A-Z]{2}$"
      
      # Tracking number pattern
      tracking_number: "^1Z[A-Z0-9]{16}$"
      
      # Status enum validation
      status: "^(pending|processing|shipped|delivered|cancelled)$"
      
---

# Array element patterns
request:
  name: "Array Element Patterns"
  url: "https://api.example.com/products"
  expect:
    body:
      products:
        - id: "^PROD-[0-9]+$"
          name: ".+"  # Any non-empty string
          price: "^[0-9]+\\.[0-9]{2}$"
          category: "^(electronics|clothing|books|home)$"
        - id: "^PROD-[0-9]+$"
          name: ".+"
          price: "^[0-9]+\\.[0-9]{2}$"
          category: "^(electronics|clothing|books|home)$"
```

## Wildcard and Flexible Validation

Use wildcards (`*`) to validate field presence without checking specific values, useful for dynamic or sensitive data.

**wildcard-validation.yaml**

```yaml
# Wildcard and Flexible Validation

# Wildcard matching (any value present)
request:
  name: "Wildcard Validation"
  url: "https://api.example.com/user/session"
  expect:
    body:
      user_id: "*"        # Any user ID value
      session_token: "*"  # Any token value
      expires_at: "*"     # Any expiration time
      created_at: "*"     # Any creation time
      
---

# Conditional validation based on response
request:
  name: "Conditional Validation"
  url: "https://api.example.com/user/123"
  expect:
    body:
      id: 123
      username: "johndoe"
      # Premium users have additional fields
      account_type: ["free", "premium", "enterprise"]
      # These fields only present for premium/enterprise
      subscription_id: "*"  # Present if premium/enterprise
      billing_cycle: "*"   # Present if premium/enterprise
      
---

# Nested wildcard validation
request:
  name: "Nested Wildcard"
  url: "https://api.example.com/complex-data"
  expect:
    body:
      metadata:
        created_by: "*"    # Any user info
        tags: "*"          # Any tag structure
        custom_fields: "*" # Any custom field object
      data:
        # Specific validation for critical fields
        id: "^[0-9]+$"
        status: "active"
        # Flexible for other fields
        properties: "*"
        
---

# Array with wildcard elements
request:
  name: "Array Wildcard Elements"
  url: "https://api.example.com/notifications"
  expect:
    body:
      notifications:
        - id: "*"          # Any notification ID
          type: "info"     # Specific type
          message: "*"     # Any message content
          timestamp: "*"   # Any timestamp
        - id: "*"
          type: "*"        # Any type for second notification
          message: "*"
          timestamp: "*"
```

## Complex Validation Scenarios

Advanced validation examples combining multiple techniques for comprehensive API testing.

**complex-validation.yaml**

```yaml
# Complex Validation Scenarios

# Nested object validation with mixed patterns
request:
  name: "E-commerce Order Validation"
  url: "https://api.ecommerce.com/orders/12345"
  expect:
    status: 200
    headers:
      content-type: "application/json"
      x-order-id: "^ORD-[0-9]{8}$"
    body:
      order:
        id: "^ORD-[0-9]{8}$"
        status: "^(pending|confirmed|shipped|delivered)$"
        customer:
          id: "^CUST-[0-9]{6}$"
          email: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
          shipping_address:
            street: "*"
            city: "*"
            state: "^[A-Z]{2}$"  # US state code
            zip_code: "^[0-9]{5}(-[0-9]{4})?$"  # US ZIP
        items:
          - sku: "^[A-Z0-9-]+$"
            quantity: "^[1-9][0-9]*$"  # Positive integer
            price: "^[0-9]+\\.[0-9]{2}$"
            discount: "^[0-9]*\\.[0-9]{2}$"  # Optional discount
        totals:
          subtotal: "^[0-9]+\\.[0-9]{2}$"
          tax: "^[0-9]+\\.[0-9]{2}$"
          shipping: "^[0-9]+\\.[0-9]{2}$"
          total: "^[0-9]+\\.[0-9]{2}$"
        payment:
          method: "^(credit_card|paypal|bank_transfer)$"
          status: "^(pending|completed|failed)$"
          transaction_id: "*"  # Any transaction ID format
          
---

# API response validation with pagination
request:
  name: "Paginated Response Validation"
  url: "https://api.example.com/products?page=1&limit=20"
  expect:
    status: 200
    headers:
      x-total-count: "^[0-9]+$"
      x-page-count: "^[0-9]+$"
    body:
      data:
        - id: "^[0-9]+$"
          name: ".+"  # Any non-empty name
          price: "^[0-9]+\\.[0-9]{2}$"
          category_id: "^[0-9]+$"
          in_stock: true  # Boolean value
          images: ["*"]   # Array with at least one image
      pagination:
        current_page: 1
        per_page: 20
        total_pages: "^[0-9]+$"
        total_items: "^[0-9]+$"
        has_next: true
        has_previous: false
        
---

# Multi-level validation with error scenarios
request:
  name: "Robust API Validation"
  url: "https://api.banking.com/accounts/123/transactions"
  expect:
    status: [200, 400, 401, 403]  # Multiple valid responses
    # Headers present for all responses
    headers:
      x-request-id: "^[a-f0-9-]{36}$"
      x-rate-limit-remaining: "^[0-9]+$"
    # Body varies by status code
    body:
      # Success response (200)
      transactions:
        - id: "^TXN-[0-9A-Z]{12}$"
          amount: "^-?[0-9]+\\.[0-9]{2}$"  # Can be negative
          type: "^(debit|credit|fee|interest)$"
          date: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$"
          description: "*"
          balance_after: "^[0-9]+\\.[0-9]{2}$"
      # OR error response (400/401/403)
      error:
        code: "^[A-Z_]+$"
        message: "*"
        details: "*"
```

## Validation Inheritance and Overrides

Understand how validation rules cascade from global to collection to request level, and how to override inherited rules.

### Validation Precedence Order

**validation-inheritance.yaml**

```yaml
# Validation Inheritance and Overrides

# Global validation defaults
global:
  defaults:
    expect:
      # All requests expect JSON by default
      headers:
        content-type: "application/json"
      # All requests should be fast
      status: [200, 201, 204]

collection:
  name: "API Test Suite"
  
  # Collection-level validation defaults
  defaults:
    expect:
      headers:
        # Inherits content-type from global
        x-api-version: "v2"  # Additional header requirement
        x-rate-limit-remaining: "*"  # Rate limit info required
      
  requests:
    # Inherits all validation from global + collection
    - name: "Standard Request"
      url: "https://api.example.com/users"
      method: GET
      # Uses: content-type: "application/json"
      #       x-api-version: "v2"  
      #       x-rate-limit-remaining: "*"
      #       status: [200, 201, 204]
      
    # Override specific validation rules
    - name: "Custom Validation Request"
      url: "https://api.example.com/upload"
      method: POST
      expect:
        status: 202  # Override global status expectation
        headers:
          # Inherits x-api-version and x-rate-limit-remaining
          content-type: "text/plain"  # Override global content-type
          x-upload-id: "^[a-f0-9-]+$"  # Additional header
          
    # Remove inherited validation  
    - name: "No Validation Request"
      url: "https://api.example.com/webhook"
      method: POST
      expect:
        status: [200, 202, 204]  # Override status
        # No header expectations (removes inherited ones)
        
    # Complex override scenario
    - name: "Special Case Request"
      url: "https://api.example.com/special"
      method: GET
      expect:
        status: [200, 404]  # Allow not found
        headers:
          content-type: ["application/json", "text/html"]  # Multiple types
          x-api-version: "v2"  # Keep collection default
          # Remove rate limit requirement for this endpoint
        body:
          # Only validate if response is 200
          data: "*"  # Any data structure
```

## Common Validation Patterns

## Validation Best Practices
