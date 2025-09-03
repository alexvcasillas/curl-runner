---
title: "Validation Rules API Reference"
description: "Complete reference for response validation using the expect configuration object. Define validation rules for status codes, headers, and response body content."
---

# Validation Rules API Reference

Complete reference for response validation using the expect configuration object. Define validation rules for status codes, headers, and response body content.

## Table of Contents

- [Overview](#overview)
- [ExpectConfig Properties](#expectconfig-properties)
- [Status Code Validation](#status-code-validation)
- [Header Validation](#header-validation)
  - [Common Header Validation Patterns](#common-header-validation-patterns)
- [Body Validation](#body-validation)
  - [Body Validation Types](#body-validation-types)
- [Pattern Validation with Regex](#pattern-validation-with-regex)
- [Wildcard and Flexible Validation](#wildcard-and-flexible-validation)
  - [Wildcard Use Cases](#wildcard-use-cases)
- [Complex Validation Scenarios](#complex-validation-scenarios)
- [Validation Inheritance and Overrides](#validation-inheritance-and-overrides)
  - [Validation Precedence Order](#validation-precedence-order)
- [Common Validation Patterns](#common-validation-patterns)
- [Validation Best Practices](#validation-best-practices)

## Overview

```yaml title="expect-config-complete.yaml"
# Complete ExpectConfig Example
request:
  name: "Comprehensive Validation Example"
  url: "https://api.example.com/users/123"
  method: GET
  expect:
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

| Property | Type | Description |
| --- | --- | --- |
| status | number | number[] | Expected HTTP status code(s) |
| headers | Record&lt;string, string&gt; | Expected response headers with values or patterns |
| body | JsonValue | Expected response body structure and values |


## Status Code Validation

```yaml title="status-validation.yaml"
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

> 200 - OK
>
> 201 - Created
>
> 204 - No Content

> 301 - Moved
>
> 302 - Found
>
> 304 - Not Modified

> 400 - Bad Request
>
> 401 - Unauthorized
>
> 404 - Not Found

> 500 - Internal Error
>
> 502 - Bad Gateway
>
> 503 - Unavailable

## Header Validation

```yaml title="header-validation.yaml"
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
      etag: "^(W/)?\\\"[a-f0-9-]+\\\""
      
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

### Common Header Validation Patterns

> **Content Headers**
>
> Validate response content metadata

> **Security Headers**
>
> Verify security policy headers

> **API Headers**
>
> Check API versioning and tracking

> **Rate Limiting**
>
> Monitor API usage limits

## Body Validation

```yaml title="body-validation.yaml"
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

### Body Validation Types

> **Exact Matching**
>
> Values must match exactly

> **Type Validation**
>
> Validates JSON data types

> **Partial Matching**
>
> Only specified fields validated

> **Nested Objects**
>
> Deep structure validation

## Pattern Validation with Regex

```yaml title="pattern-validation.yaml"
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

```yaml title="wildcard-validation.yaml"
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

### Wildcard Use Cases

> **Dynamic Values**
>


> **Sensitive Data**
>


> **Optional Fields**
>


> **Complex Structures**
>


## Complex Validation Scenarios

```yaml title="complex-validation.yaml"
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

```yaml title="validation-inheritance.yaml"
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

### Validation Precedence Order

> **1. Request-Level Validation**
>


> **2. Collection-Level Defaults**
>


> **3. Global Defaults**
>


## Common Validation Patterns

> **API Authentication**
>
> Validate login responses and token formats

> **Data Creation**
>
> Verify new resource creation and IDs

> **Error Handling**
>
> Validate error response structures

> **Pagination**
>
> Check paginated response metadata

## Validation Best Practices

> **Start Simple, Add Complexity**
>


> **Use Partial Matching**
>


> **Leverage Patterns for Dynamic Data**
>


> **Test Both Success and Error Cases**
>


> **Use Wildcards for Sensitive Data**
>


