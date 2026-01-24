---
title: "Request Object API Reference"
description: "Complete reference for the RequestConfig interface and all available options for configuring HTTP requests."
category: "Documentation"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - request
  - object
  - reference
  - variables
  - authentication
  - validation
  - retry
  - timeout
  - headers
  - response
  - collection
  - environment
slug: "/docs/request-object"
toc: true
date: "2026-01-24T16:01:46.350Z"
lastModified: "2026-01-24T16:01:46.350Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Request Object API Reference"
  category: "Documentation"
tags:
  - documentation
  - documentation
og:
  title: "Request Object API Reference - curl-runner Documentation"
  description: "Complete reference for the RequestConfig interface and all available options for configuring HTTP requests."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Request Object API Reference"
  description: "Complete reference for the RequestConfig interface and all available options for configuring HTTP requests."
  datePublished: "2026-01-24T16:01:46.350Z"
  dateModified: "2026-01-24T16:01:46.350Z"
---

# Request Object API Reference

Complete reference for the RequestConfig interface and all available options for configuring HTTP requests.

## Overview

The Request Object defines the configuration for individual HTTP requests in curl-runner. It supports all standard HTTP methods, authentication, headers, body content, error handling, and response validation.

**complete-request.yaml**

```yaml
# Complete Request Object Example
request:
  name: "Complete API Request"
  url: "https://api.example.com/users"
  method: POST
  headers:
    Content-Type: "application/json"
    Authorization: "Bearer \${API_TOKEN}"
    X-API-Version: "v2"
    User-Agent: "curl-runner/1.0.0"
  params:
    include: "profile,settings"
    sort: "created_at"
    limit: "10"
  body:
    username: "john_doe"
    email: "john@example.com"
    profile:
      firstName: "John"
      lastName: "Doe"
      age: 30
  timeout: 5000
  followRedirects: true
  maxRedirects: 3
  auth:
    type: bearer
    token: "\${API_TOKEN}"
  proxy: "http://proxy.company.com:8080"
  insecure: false
  output: "user-creation-result.json"
  retry:
    count: 3
    delay: 1000
  variables:
    USER_TYPE: "premium"
    REGION: "us-east"
  expect:
    failure: false  # Expect this request to succeed (default)
    status: 201
    headers:
      location: "^/users/[0-9]+$"
      content-type: "application/json"
    body:
      id: "^[0-9]+$"
      username: "john_doe"
      email: "john@example.com"
```

## Properties Reference

## HTTP Methods

curl-runner supports all standard HTTP methods. The default method is GET if not specified.

**http-methods.yaml**

```yaml
# HTTP Methods Examples
requests:
  # GET - Retrieve data
  - name: "Get User Profile"
    url: "https://api.example.com/users/123"
    method: GET
    
  # POST - Create new resource
  - name: "Create New User"
    url: "https://api.example.com/users"
    method: POST
    body:
      username: "newuser"
      email: "newuser@example.com"
      
  # PUT - Complete resource replacement
  - name: "Update User (Complete)"
    url: "https://api.example.com/users/123"
    method: PUT
    body:
      id: 123
      username: "updated_user"
      email: "updated@example.com"
      profile:
        firstName: "Updated"
        lastName: "User"
        
  # PATCH - Partial resource update
  - name: "Update User (Partial)"
    url: "https://api.example.com/users/123"
    method: PATCH
    body:
      email: "new_email@example.com"
      
  # DELETE - Remove resource
  - name: "Delete User"
    url: "https://api.example.com/users/123"
    method: DELETE
    
  # HEAD - Get headers only
  - name: "Check Resource Exists"
    url: "https://api.example.com/users/123"
    method: HEAD
    
  # OPTIONS - Get allowed methods
  - name: "Check Allowed Methods"
    url: "https://api.example.com/users"
    method: OPTIONS
```

## Headers Configuration

Configure HTTP headers for authentication, content type, custom metadata, and more.

**headers-config.yaml**

```yaml
# Headers Configuration Examples
requests:
  # Basic headers
  - name: "JSON API Request"
    url: "https://api.example.com/data"
    method: POST
    headers:
      Content-Type: "application/json"
      Accept: "application/json"
      
  # Authentication headers
  - name: "Authenticated Request"
    url: "https://api.example.com/protected"
    method: GET
    headers:
      Authorization: "Bearer \${ACCESS_TOKEN}"
      X-API-Key: "\${API_KEY}"
      
  # Custom headers
  - name: "Custom Headers Request"
    url: "https://api.example.com/custom"
    method: GET
    headers:
      X-Request-ID: "\${REQUEST_UUID}"
      X-Client-Version: "1.2.3"
      X-Environment: "production"
      User-Agent: "MyApp/1.0.0"
      
  # Content encoding
  - name: "Compressed Request"
    url: "https://api.example.com/upload"
    method: POST
    headers:
      Content-Type: "application/json"
      Content-Encoding: "gzip"
      Accept-Encoding: "gzip, deflate"
      
  # Cache control
  - name: "Cache-Controlled Request"
    url: "https://api.example.com/cached-data"
    method: GET
    headers:
      Cache-Control: "no-cache"
      If-None-Match: "\${ETAG_VALUE}"
      If-Modified-Since: "Wed, 21 Oct 2024 07:28:00 GMT"
```

## URL Parameters

Add query parameters to URLs using the params object. Parameters are automatically URL-encoded.

**url-parameters.yaml**

```yaml
# URL Parameters Examples
requests:
  # Query parameters as object
  - name: "Search with Parameters"
    url: "https://api.example.com/search"
    method: GET
    params:
      q: "search term"
      page: "1"
      limit: "20"
      sort: "created_at"
      order: "desc"
      include: "author,tags"
      
  # Parameters with arrays (multiple values)
  - name: "Multi-value Parameters"
    url: "https://api.example.com/items"
    method: GET
    params:
      category: "electronics,books"  # Comma-separated
      status: "active"
      tags: "new,featured,sale"
      
  # Parameters with special characters
  - name: "Special Character Parameters"
    url: "https://api.example.com/search"
    method: GET
    params:
      q: "hello world"      # Spaces
      filter: "price>100"   # Special characters
      location: "New York, NY"
      
  # Parameters with variables
  - name: "Variable Parameters"
    url: "https://api.example.com/users"
    method: GET
    params:
      userId: "\${USER_ID}"
      region: "\${USER_REGION}"
      locale: "\${USER_LOCALE}"
      apiVersion: "\${API_VERSION}"
```

## Request Body

Configure request body content for POST, PUT, PATCH, and other methods that support body data.

**request-body.yaml**

```yaml
# Request Body Examples
requests:
  # JSON object body
  - name: "JSON Object Body"
    url: "https://api.example.com/users"
    method: POST
    headers:
      Content-Type: "application/json"
    body:
      username: "johndoe"
      email: "john@example.com"
      profile:
        firstName: "John"
        lastName: "Doe"
        preferences:
          theme: "dark"
          notifications: true
          
  # JSON array body
  - name: "JSON Array Body"
    url: "https://api.example.com/batch-create"
    method: POST
    headers:
      Content-Type: "application/json"
    body:
      - name: "Item 1"
        type: "document"
      - name: "Item 2"
        type: "image"
      - name: "Item 3"
        type: "video"
        
  # Form data body (URL encoded)
  - name: "Form Data Body"
    url: "https://api.example.com/form-submit"
    method: POST
    headers:
      Content-Type: "application/x-www-form-urlencoded"
    body: "name=John+Doe&email=john%40example.com&message=Hello+World"
    
  # Plain text body
  - name: "Plain Text Body"
    url: "https://api.example.com/text-content"
    method: POST
    headers:
      Content-Type: "text/plain"
    body: "This is plain text content that will be sent as the request body."
    
  # Empty body (common for GET, DELETE)
  - name: "No Body Request"
    url: "https://api.example.com/resource/123"
    method: DELETE
    # No body field needed
    
  # Body with variables
  - name: "Variable Body"
    url: "https://api.example.com/dynamic"
    method: POST
    headers:
      Content-Type: "application/json"
    body:
      userId: "\${USER_ID}"
      timestamp: "\${CURRENT_TIME}"
      environment: "\${ENV}"
      data:
        value: "\${DYNAMIC_VALUE}"
        reference: "\${REF_ID}"
```

## Form Data & File Uploads

Use the `formData` property to send multipart/form-data requests with file uploads. This is mutually exclusive with the `body` property.

### File Attachment Properties

| Property | Type | Required | Description |
| --- | --- | --- | --- |
| `file` | string | _Required_ | Path to the file (relative or absolute) |
| `filename` | string | _Optional_ | Custom filename sent to the server |
| `contentType` | string | _Optional_ | MIME type (curl auto-detects if not specified) |

**form-data.yaml**

```yaml
# Form Data and File Upload Examples
requests:
  # Simple file upload
  - name: "Single file upload"
    url: "https://api.example.com/upload"
    method: POST
    formData:
      document:
        file: "./report.pdf"

  # File with custom filename and content type
  - name: "File with options"
    url: "https://api.example.com/upload"
    method: POST
    formData:
      document:
        file: "./local-file.pdf"
        filename: "quarterly-report.pdf"
        contentType: "application/pdf"

  # Multiple files with form fields
  - name: "Mixed form data"
    url: "https://api.example.com/submit"
    method: POST
    formData:
      # Text fields
      title: "Document Submission"
      description: "Q4 Financial Reports"
      submitted_by: "john@example.com"

      # File attachments
      main_document:
        file: "./report.pdf"
        filename: "financial-report.pdf"
        contentType: "application/pdf"
      supporting_document:
        file: "./appendix.xlsx"
        contentType: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"

  # File upload with variables
  - name: "Dynamic file upload"
    url: "\${API_URL}/users/\${USER_ID}/avatar"
    method: POST
    formData:
      description: "Uploaded on \${DATE:YYYY-MM-DD}"
      request_id: "\${UUID}"
      avatar:
        file: "./profile-photo.jpg"
        contentType: "image/jpeg"
```

## Authentication

Configure authentication using the auth object or manual headers.

**authentication.yaml**

```yaml
# Authentication Examples
requests:
  # Basic authentication
  - name: "Basic Auth Request"
    url: "https://api.example.com/protected"
    method: GET
    auth:
      type: basic
      username: "\${BASIC_USER}"
      password: "\${BASIC_PASS}"
      
  # Bearer token authentication
  - name: "Bearer Token Request"
    url: "https://api.github.com/user"
    method: GET
    auth:
      type: bearer
      token: "\${GITHUB_TOKEN}"
      
  # Manual authorization header (alternative to auth object)
  - name: "Manual Auth Header"
    url: "https://api.example.com/protected"
    method: GET
    headers:
      Authorization: "Bearer \${ACCESS_TOKEN}"
      
  # API key authentication
  - name: "API Key Request"
    url: "https://api.example.com/data"
    method: GET
    headers:
      X-API-Key: "\${API_KEY}"
      X-API-Secret: "\${API_SECRET}"
      
  # Custom authentication scheme
  - name: "Custom Auth Request"
    url: "https://api.example.com/custom-auth"
    method: GET
    headers:
      Authorization: "Custom \${CUSTOM_TOKEN}"
      X-Signature: "\${REQUEST_SIGNATURE}"
      X-Timestamp: "\${REQUEST_TIMESTAMP}"
```

## Timeout & Retry Configuration

Configure request timeouts and retry behavior for handling network issues and unreliable services.

**timeout-retry.yaml**

```yaml
# Timeout and Retry Configuration
requests:
  # Basic timeout
  - name: "Quick Request"
    url: "https://fast-api.example.com/data"
    method: GET
    timeout: 3000  # 3 seconds
    
  # Long timeout for slow endpoints
  - name: "Slow Processing Request"
    url: "https://api.example.com/heavy-processing"
    method: POST
    timeout: 60000  # 60 seconds
    body:
      processLargeDataset: true
      
  # Retry configuration
  - name: "Reliable Request with Retries"
    url: "https://unreliable-api.example.com/data"
    method: GET
    timeout: 5000
    retry:
      count: 5      # Retry up to 5 times
      delay: 2000   # Wait 2 seconds between retries
      
  # No retries (default behavior)
  - name: "One-Shot Request"
    url: "https://api.example.com/one-time-action"
    method: POST
    timeout: 10000
    retry:
      count: 0      # Explicitly no retries
      
  # Quick retry for flaky services
  - name: "Flaky Service Request"
    url: "https://flaky-service.example.com/endpoint"
    method: GET
    timeout: 2000
    retry:
      count: 3
      delay: 500    # Quick retry interval
```

## Redirect Handling

Control how curl-runner handles HTTP redirects (3xx status codes).

**redirects.yaml**

```yaml
# Redirect Handling Configuration
requests:
  # Follow redirects (default behavior)
  - name: "Follow Redirects"
    url: "https://example.com/redirect-me"
    method: GET
    followRedirects: true
    maxRedirects: 5
    
  # Don't follow redirects
  - name: "No Redirect Following"
    url: "https://example.com/redirect-me"
    method: GET
    followRedirects: false
    expect:
      status: [301, 302, 307, 308]  # Expect redirect status
      
  # Limited redirect following
  - name: "Limited Redirects"
    url: "https://example.com/redirect-chain"
    method: GET
    followRedirects: true
    maxRedirects: 2  # Only follow up to 2 redirects
    
  # POST request with redirect handling
  - name: "POST with Redirect"
    url: "https://example.com/form-submit"
    method: POST
    followRedirects: true
    maxRedirects: 3
    body:
      action: "submit"
```

## Proxy & Security Configuration

Configure proxy settings and SSL/TLS security options.

**proxy-security.yaml**

```yaml
# Proxy and Security Configuration
requests:
  # HTTP proxy
  - name: "HTTP Proxy Request"
    url: "https://api.example.com/data"
    method: GET
    proxy: "http://proxy.company.com:8080"
    
  # SOCKS proxy - Coming Soon
  - name: "SOCKS Proxy Request"
    url: "https://api.example.com/data"
    method: GET
    proxy: "socks5://proxy.company.com:1080"  # Coming Soon
    
  # Proxy with authentication - Coming Soon  
  - name: "Authenticated Proxy Request"
    url: "https://api.example.com/data"
    method: GET
    proxy: "http://username:password@proxy.company.com:8080"  # Coming Soon
    
  # Disable SSL verification (insecure)
  - name: "Insecure SSL Request"
    url: "https://self-signed-cert.example.com/api"
    method: GET
    insecure: true  # Skip SSL certificate verification
    
  # Secure request (default behavior)
  - name: "Secure Request"
    url: "https://api.example.com/secure-endpoint"
    method: GET
    insecure: false  # Verify SSL certificates (default)
```

## Request-Level Variables

Define variables at the request level that can be used within that specific request.

**request-variables.yaml**

```yaml
# Request-Level Variables
requests:
  # Variables defined at request level
  - name: "Request with Local Variables"
    url: "https://api.example.com/users/\${LOCAL_USER_ID}"
    method: GET
    variables:
      LOCAL_USER_ID: "12345"
      REQUEST_TYPE: "profile"
      CACHE_TTL: "300"
    headers:
      X-Request-Type: "\${REQUEST_TYPE}"
      X-Cache-TTL: "\${CACHE_TTL}"
      
  # Variables override global ones
  - name: "Override Global Variables"
    url: "\${BASE_URL}/api/v2/data"  # Uses global BASE_URL
    method: GET
    variables:
      API_VERSION: "v2"  # Overrides global API_VERSION
      TIMEOUT: "8000"    # Local timeout override
    timeout: \${TIMEOUT}
    
  # Variables with complex values
  - name: "Complex Variable Values"
    url: "https://api.example.com/search"
    method: POST
    variables:
      SEARCH_FILTERS:
        status: "active"
        category: "electronics"
        price_range:
          min: 10
          max: 100
    body: \${SEARCH_FILTERS}
```

## Response Validation

Configure response validation using the expect object to verify status codes, headers, and body content. Includes support for negative testing with expect.failure.

> Set `expect.failure: true` to test scenarios where you expect the request to fail:

**validation-config.yaml**

```yaml
# Response Validation Examples
requests:
  # Standard success validation
  - name: "Success Response Validation"
    url: "https://api.example.com/users"
    method: POST
    body:
      username: "newuser"
      email: "newuser@example.com"
    expect:
      failure: false    # Expect success (default)
      status: 201       # Created
      headers:
        content-type: "application/json"
        location: "^/users/[0-9]+$"
      body:
        id: "^[0-9]+$"
        username: "newuser"
        email: "newuser@example.com"
        created_at: "*"   # Any timestamp value
        
  # Negative testing - expect failure
  - name: "Invalid Input Validation"  
    url: "https://api.example.com/users"
    method: POST
    body:
      username: ""      # Invalid: empty username
      email: "invalid"  # Invalid: malformed email
    expect:
      failure: true     # Expect this request to fail
      status: 400       # Bad Request
      body:
        error: "Validation failed"
        details:
          username: "Username is required"
          email: "Invalid email format"
          
  # Authentication failure testing
  - name: "Unauthorized Access Test"
    url: "https://api.example.com/protected"
    method: GET
    headers:
      authorization: "Bearer invalid-token"
    expect:
      failure: true     # Expect authentication to fail
      status: 401       # Unauthorized
      body:
        error: "Invalid token"
        
  # Mixed validation with multiple acceptable statuses
  - name: "Flexible Status Validation"
    url: "https://api.example.com/cached-resource"
    method: GET
    headers:
      if-none-match: "some-etag"
    expect:
      failure: false    # Expect success
      status: [200, 304] # OK or Not Modified
      headers:
        cache-control: "*"  # Any cache control header
```

## Output Configuration

Configure where to save response data from individual requests.

**output-config.yaml**

```yaml
# Output Configuration
requests:
  # Save response to file
  - name: "Save Response to File"
    url: "https://api.example.com/large-dataset"
    method: GET
    output: "dataset.json"
    
  # Save with dynamic filename
  - name: "Dynamic Output Filename"
    url: "https://api.example.com/reports/daily"
    method: GET
    output: "daily-report-\${DATE}.json"
    
  # Save to directory structure
  - name: "Organized Output"
    url: "https://api.example.com/user/\${USER_ID}/profile"
    method: GET
    output: "data/users/\${USER_ID}/profile.json"
    
  # No output file (default - display in console)
  - name: "Console Output"
    url: "https://api.example.com/status"
    method: GET
    # No output field - results shown in console
```

## Best Practices

### Best Practices

• Use descriptive variable names
• Define common values as variables
• Use environment variables for secrets
• Group related variables logically
• Document complex expressions
