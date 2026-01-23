---
title: "Response Object API Reference"
description: "Complete reference for response objects returned by curl-runner, including ExecutionResult and ExecutionSummary interfaces."
category: "Documentation"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - response
  - object
  - reference
  - validation
  - retry
  - timeout
  - headers
  - request
  - collection
slug: "/docs/response-object"
toc: true
date: "2026-01-23T21:27:49.036Z"
lastModified: "2026-01-23T21:27:49.036Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Response Object API Reference"
  category: "Documentation"
tags:
  - documentation
  - documentation
og:
  title: "Response Object API Reference - curl-runner Documentation"
  description: "Complete reference for response objects returned by curl-runner, including ExecutionResult and ExecutionSummary interfaces."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Response Object API Reference"
  description: "Complete reference for response objects returned by curl-runner, including ExecutionResult and ExecutionSummary interfaces."
  datePublished: "2026-01-23T21:27:49.036Z"
  dateModified: "2026-01-23T21:27:49.036Z"
---

# Response Object API Reference

Complete reference for response objects returned by curl-runner, including ExecutionResult and ExecutionSummary interfaces.

## Overview

curl-runner provides detailed response information through structured objects. Every request returns an ExecutionResult, and collections return an ExecutionSummary containing multiple results with aggregate statistics.

## ExecutionResult Interface

The ExecutionResult interface contains complete information about a single request execution, including the original request configuration, response data, and performance metrics.

**execution-result.json**

```json
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

## Error Response Structure

When requests fail due to network issues, timeouts, or other errors, the ExecutionResult provides error information.

**error-result.json**

```json
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

## ExecutionSummary Interface

The ExecutionSummary provides aggregate information about a collection of requests, including overall statistics and all individual results.

**execution-summary.json**

```json
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

## Response Body Types

curl-runner automatically parses JSON responses and preserves the original data types for all response content.

**body-types.json**

```json
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

## Performance Metrics

Detailed timing information for analyzing request performance and identifying bottlenecks.

**metrics-details.json**

```json
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

## HTTP Status Code Handling

Understanding how curl-runner processes different HTTP status codes and determines success/failure.

**status-codes.json**

```json
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

## Response Headers

Common response headers and their significance in API responses.

**response-headers.json**

```json
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

## Processing Response Data

Techniques for working with response data programmatically and in automated workflows.

**response-processing.yaml**

```yaml
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
      
# Chain requests using response data - Coming Soon
collection:
  name: "Request Chain Example"  # Coming Soon
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
          
    # Second request - use data from first - Coming Soon
    - name: "Get User Data"  # Coming Soon
      url: "https://api.example.com/users/\${USER_ID}"  # Coming Soon - from previous response
      headers:
        Authorization: "Bearer \${TOKEN}"              # Coming Soon - from previous response
      expect:
        status: 200
```

## Response Handling Best Practices
