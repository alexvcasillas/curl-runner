---
title: "Response Object API Reference"
description: "Complete reference for response objects returned by curl-runner, including ExecutionResult and ExecutionSummary interfaces."
---

# Response Object API Reference

Complete reference for response objects returned by curl-runner, including ExecutionResult and ExecutionSummary interfaces.

## Table of Contents

- [Overview](#overview)
- [ExecutionResult Interface](#executionresult-interface)
- [Error Response Structure](#error-response-structure)
- [ExecutionSummary Interface](#executionsummary-interface)
- [Response Body Types](#response-body-types)
  - [JSON Response Parsing](#json-response-parsing)
- [Performance Metrics](#performance-metrics)
- [HTTP Status Code Handling](#http-status-code-handling)
- [Response Headers](#response-headers)
  - [Header Categories](#header-categories)
- [Processing Response Data](#processing-response-data)
- [Response Handling Best Practices](#response-handling-best-practices)

## Overview

## ExecutionResult Interface

```json title="execution-result.json"
# Example ExecutionResult object structure
{
  "request": {
    "name": "Get User Profile",
    "url": "https://api.example.com/users/123",
    "method": "GET",
    "headers": {
      "Authorization": "Bearer token123",
      "Accept": "application/json"
    },
    "timeout": 5000
  },
  "success": true,
  "status": 200,
  "headers": {
    "content-type": "application/json; charset=utf-8",
    "content-length": "256",
    "date": "Wed, 21 Oct 2024 07:28:00 GMT",
    "server": "nginx/1.18.0",
    "x-rate-limit-remaining": "99",
    "x-rate-limit-reset": "1634799480",
    "etag": "W/\\"d41d8cd98f00b204e9800998ecf8427e\\"",
    "cache-control": "no-cache, no-store, must-revalidate"
  },
  "body": {
    "id": 123,
    "username": "johndoe",
    "email": "john@example.com",
    "profile": {
      "firstName": "John",
      "lastName": "Doe",
      "createdAt": "2024-01-15T09:30:00Z",
      "lastLogin": "2024-10-21T07:25:00Z"
    },
    "preferences": {
      "theme": "dark",
      "notifications": true,
      "language": "en"
    }
  },
  "metrics": {
    "duration": 245,
    "size": 256,
    "dnsLookup": 12,
    "tcpConnection": 45,
    "tlsHandshake": 67,
    "firstByte": 189,
    "download": 8
  }
}
```

| Property | Type | Description |
| --- | --- | --- |
| request | RequestConfig | Original request configuration that was executed |
| success | boolean | Whether the request completed successfully |
| status | number? | HTTP status code (if response received) |
| headers | Record&lt;string, string&gt;? | Response headers (if response received) |
| body | JsonValue? | Response body content (parsed if JSON) |
| error | string? | Error message (if request failed) |
| metrics | MetricsObject? | Performance timing information |


## Error Response Structure

```json title="error-result.json"
# Example failed request ExecutionResult
{
  "request": {
    "name": "Failed Request",
    "url": "https://api.unreachable.com/data",
    "method": "GET",
    "timeout": 3000
  },
  "success": false,
  "error": "Connection timeout after 3000ms",
  "metrics": {
    "duration": 3000,
    "dnsLookup": 2500,
    "tcpConnection": 500
  }
}
```

> **Network Errors**
>


> **HTTP Error Status**
>


## ExecutionSummary Interface

```json title="execution-summary.json"
# Example ExecutionSummary for a collection run
{
  "total": 5,
  "successful": 3,
  "failed": 2,
  "duration": 2456,
  "results": [
    {
      "request": {
        "name": "Get Users List",
        "url": "https://api.example.com/users",
        "method": "GET"
      },
      "success": true,
      "status": 200,
      "body": [
        {"id": 1, "username": "user1"},
        {"id": 2, "username": "user2"}
      ],
      "metrics": {
        "duration": 234,
        "size": 124
      }
    },
    {
      "request": {
        "name": "Create User",
        "url": "https://api.example.com/users",
        "method": "POST"
      },
      "success": true,
      "status": 201,
      "headers": {
        "location": "/users/3",
        "content-type": "application/json"
      },
      "body": {
        "id": 3,
        "username": "newuser",
        "createdAt": "2024-10-21T07:30:00Z"
      },
      "metrics": {
        "duration": 456,
        "size": 89
      }
    },
    {
      "request": {
        "name": "Get Invalid User",
        "url": "https://api.example.com/users/999",
        "method": "GET"
      },
      "success": false,
      "status": 404,
      "body": {
        "error": "User not found",
        "code": "USER_NOT_FOUND"
      },
      "metrics": {
        "duration": 123,
        "size": 45
      }
    },
    {
      "request": {
        "name": "Update User",
        "url": "https://api.example.com/users/1",
        "method": "PUT"
      },
      "success": true,
      "status": 200,
      "body": {
        "id": 1,
        "username": "updated_user",
        "updatedAt": "2024-10-21T07:31:00Z"
      },
      "metrics": {
        "duration": 345,
        "size": 78
      }
    },
    {
      "request": {
        "name": "Timeout Request",
        "url": "https://api.slow.com/data",
        "method": "GET"
      },
      "success": false,
      "error": "Request timeout after 5000ms",
      "metrics": {
        "duration": 5000
      }
    }
  ]
}
```

| Property | Type | Description |
| --- | --- | --- |
| total | number | Total number of requests executed |
| successful | number | Number of successful requests |
| failed | number | Number of failed requests |
| duration | number | Total execution time in milliseconds |
| results | ExecutionResult[] | Array of individual request results |


## Response Body Types

```json title="body-types.json"
# Different response body types curl-runner handles

# JSON Object Response
{
  "body": {
    "user": {
      "id": 123,
      "name": "John Doe"
    },
    "metadata": {
      "timestamp": "2024-10-21T07:30:00Z"
    }
  }
}

# JSON Array Response  
{
  "body": [
    {"id": 1, "name": "Item 1"},
    {"id": 2, "name": "Item 2"},
    {"id": 3, "name": "Item 3"}
  ]
}

# Plain Text Response
{
  "body": "This is a plain text response from the server"
}

# Number Response
{
  "body": 42
}

# Boolean Response
{
  "body": true
}

# Null Response
{
  "body": null
}

# Empty Response (no body)
{
  "status": 204,
  "headers": {
    "content-length": "0"
  }
  // No body field present
}
```

### JSON Response Parsing

> **Automatic Parsing**
>


> **Type Preservation**
>


> **Text Fallback**
>


> **Empty Responses**
>


## Performance Metrics

```json title="metrics-details.json"
# Detailed explanation of response metrics

{
  "metrics": {
    "duration": 1245,        // Total request duration in milliseconds
    "size": 2048,           // Response body size in bytes
    "dnsLookup": 23,        // Time to resolve DNS in milliseconds
    "tcpConnection": 45,     // Time to establish TCP connection
    "tlsHandshake": 127,    // Time for TLS/SSL handshake (HTTPS only)
    "firstByte": 856,       // Time to first response byte (TTFB)
    "download": 67          // Time to download response body
  }
}

# Metrics breakdown:
# - Total time = dnsLookup + tcpConnection + tlsHandshake + firstByte + download
# - Some metrics may be 0 or undefined based on request type and caching
# - DNS lookup may be 0 if DNS is cached
# - TLS handshake only present for HTTPS requests
# - Connection time may be 0 if connection is reused
```

| Metric | Unit | Description |
| --- | --- | --- |
| duration | milliseconds | Total request time from start to finish |
| size | bytes | Response body size |
| dnsLookup | milliseconds | DNS resolution time |
| tcpConnection | milliseconds | TCP connection establishment time |
| tlsHandshake | milliseconds | TLS/SSL handshake time (HTTPS only) |
| firstByte | milliseconds | Time to first response byte (TTFB) |
| download | milliseconds | Response body download time |


## HTTP Status Code Handling

```json title="status-codes.json"
# Common HTTP status codes in responses

# Success responses (2xx)
{
  "success": true,
  "status": 200,    // OK - Request successful
  "body": "data"
}

{
  "success": true,
  "status": 201,    // Created - Resource created successfully
  "headers": {
    "location": "/users/123"
  }
}

{
  "success": true,
  "status": 204,    // No Content - Successful with no body
  "headers": {
    "content-length": "0"
  }
}

# Client error responses (4xx)
{
  "success": false,
  "status": 400,    // Bad Request
  "body": {
    "error": "Invalid request parameters",
    "details": ["Missing required field: email"]
  }
}

{
  "success": false,
  "status": 401,    // Unauthorized
  "body": {
    "error": "Authentication required",
    "code": "UNAUTHORIZED"
  }
}

{
  "success": false,
  "status": 404,    // Not Found
  "body": {
    "error": "Resource not found",
    "code": "NOT_FOUND"
  }
}

{
  "success": false,
  "status": 429,    // Too Many Requests
  "headers": {
    "retry-after": "60",
    "x-rate-limit-remaining": "0"
  },
  "body": {
    "error": "Rate limit exceeded"
  }
}

# Server error responses (5xx)
{
  "success": false,
  "status": 500,    // Internal Server Error
  "body": {
    "error": "Internal server error",
    "code": "INTERNAL_ERROR"
  }
}

{
  "success": false,
  "status": 503,    // Service Unavailable
  "headers": {
    "retry-after": "120"
  },
  "body": {
    "error": "Service temporarily unavailable"
  }
}
```

> 200 OK
>
> 201 Created
>
> 204 No Content

> 301 Moved
>
> 302 Found
>
> 304 Not Modified

> 400 Bad Request
>
> 401 Unauthorized
>
> 404 Not Found

> 500 Internal Error
>
> 502 Bad Gateway
>
> 503 Unavailable

## Response Headers

```json title="response-headers.json"
# Common response headers and their meanings

{
  "headers": {
    // Content headers
    "content-type": "application/json; charset=utf-8",
    "content-length": "1024",
    "content-encoding": "gzip",
    "content-language": "en-US",
    
    // Caching headers
    "cache-control": "public, max-age=3600",
    "etag": "W/\\"686897696a7c876b7e\\"",
    "expires": "Thu, 22 Oct 2024 07:30:00 GMT",
    "last-modified": "Wed, 21 Oct 2024 07:30:00 GMT",
    
    // Security headers
    "strict-transport-security": "max-age=31536000",
    "content-security-policy": "default-src 'self'",
    "x-frame-options": "DENY",
    "x-content-type-options": "nosniff",
    
    // Rate limiting headers
    "x-rate-limit-limit": "1000",
    "x-rate-limit-remaining": "999",
    "x-rate-limit-reset": "1634799600",
    
    // Server information
    "server": "nginx/1.18.0",
    "x-powered-by": "Express",
    "date": "Wed, 21 Oct 2024 07:28:00 GMT",
    
    // Custom API headers
    "x-api-version": "v2",
    "x-request-id": "req_123456789",
    "x-response-time": "45ms",
    
    // Location header (for redirects/created resources)
    "location": "https://api.example.com/users/123"
  }
}
```

### Header Categories

> **Content Headers**
>
> Describe response body

> **Caching Headers**
>
> Control client-side caching

> **Security Headers**
>
> Enforce security policies

> **API Headers**
>
> API-specific metadata

## Processing Response Data

```yaml title="response-processing.yaml"
# How to process response data programmatically

# Save response to file for processing
request:
  name: "Get Large Dataset"
  url: "https://api.example.com/dataset"
  output: "dataset.json"
  
# Use response validation to check structure
request:
  name: "Validate API Response"
  url: "https://api.example.com/user/profile"
  expect:
    status: 200
    headers:
      content-type: "application/json"
    body:
      id: "^[0-9]+$"        # Validate ID format
      email: ".*@.*\\..*"    # Validate email format
      created_at: "^\\d{4}-\\d{2}-\\d{2}"  # ISO date format
      
# Chain requests using response data
collection:
  name: "Request Chain Example"
  requests:
    # First request - get user ID
    - name: "Login User"
      url: "https://api.example.com/auth/login"
      method: POST
      body:
        username: "testuser"
        password: "password"
      expect:
        status: 200
        body:
          token: "*"        # Any token value
          user_id: "*"      # Any user ID
          
    # Second request - use data from first
    - name: "Get User Data"
      url: "https://api.example.com/users/\${USER_ID}"  # From previous response
      headers:
        Authorization: "Bearer \${TOKEN}"              # From previous response
      expect:
        status: 200
```

> **Pro Tip:**
>


## Response Handling Best Practices

> **Validate Critical Fields**
>


> **Monitor Performance Metrics**
>


> **Handle Errors Gracefully**
>


> **Use Response Headers**
>


