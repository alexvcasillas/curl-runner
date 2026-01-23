---
title: "Global Settings"
description: "Configure global execution settings, defaults, and behaviors that apply to all requests in your YAML files."
category: "Configuration"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - global
  - settings
  - yaml
  - variables
  - parallel
  - sequential
  - validation
  - retry
  - timeout
  - headers
  - response
  - request
  - cli
  - environment
slug: "/docs/global-settings"
toc: true
date: "2026-01-23T22:45:48.025Z"
lastModified: "2026-01-23T22:45:48.025Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Global Settings"
  category: "Configuration"
tags:
  - documentation
  - configuration
og:
  title: "Global Settings - curl-runner Documentation"
  description: "Configure global execution settings, defaults, and behaviors that apply to all requests in your YAML files."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Global Settings"
  description: "Configure global execution settings, defaults, and behaviors that apply to all requests in your YAML files."
  datePublished: "2026-01-23T22:45:48.025Z"
  dateModified: "2026-01-23T22:45:48.025Z"
---

# Global Settings

Configure global execution settings, defaults, and behaviors that apply to all requests in your YAML files.

Global settings allow you to configure default behavior for all requests in a YAML file. These settings can be overridden by individual requests when needed.

**basic-global-config.yaml**

```yaml
# Basic global configuration
global:
  # Execution settings
  execution: sequential
  continueOnError: false
  
  # Timeout settings
  timeout: 5000
  retries: 3
  
  # Output settings
  output:
    verbose: true
    saveToFile: "results.json"
    
  # Default request settings
  defaults:
    headers:
      User-Agent: "curl-runner/1.0.0"
      Accept: "application/json"
    timeout: 10000

requests:
  - name: Example Request
    url: https://api.example.com/data
    method: GET
```

## Execution Settings

Control how requests are executed and how errors are handled.

**execution-modes.yaml**

```yaml
# Sequential execution (default)
global:
  execution: sequential  # Requests run one after another
  
# Parallel execution  
global:
  execution: parallel   # All requests run simultaneously
  
# With error handling
global:
  execution: parallel
  continueOnError: true  # Don't stop if some requests fail
```

## Timeout & Retry Settings

Configure global timeout and retry behavior for robust request handling.

**timeout-retry-config.yaml**

```yaml
# Global timeout and retry settings
global:
  # Global timeout for all requests (milliseconds)
  timeout: 10000
  
  # Maximum retries for failed requests
  retries: 3
  
  # Retry delay settings
  retryDelay: 1000        # Initial delay between retries (ms)
  retryBackoff: 2.0       # Exponential backoff multiplier
  
  # Individual request can override these
  defaults:
    timeout: 5000         # Default per-request timeout
    retries: 1            # Default per-request retries

requests:
  - name: Standard Request
    url: https://api.example.com/data
    method: GET
    # Uses global defaults
    
  - name: Custom Timeout Request
    url: https://slow-api.example.com/data
    method: GET
    timeout: 30000        # Overrides global default
    retries: 5            # Overrides global default
```

## Output Configuration

Customize how `curl-runner` displays results and saves output data.

**output-config.yaml**

```yaml
# Output configuration options
global:
  output:
    # Verbose output with request/response details
    verbose: true
    
    # Save results to JSON file
    saveToFile: "test-results.json"
    
    # Include response bodies in output
    includeResponseBody: true
    
    # Include request details in output
    includeRequestDetails: true
    
    # Format for console output
    format: "detailed"  # Options: "minimal", "standard", "detailed"
    
    # Colorize console output
    colors: true
    
    # Show progress indicators
    showProgress: true
    
    # Timestamp format
    timestampFormat: "ISO"  # Options: "ISO", "unix", "relative"
```

## Global Defaults

Set default values that are automatically applied to all requests unless overridden.

**global-defaults.yaml**

```yaml
# Global defaults applied to all requests
global:
  defaults:
    # Default headers for all requests
    headers:
      User-Agent: "MyApp/1.0.0"
      Accept: "application/json"
      Content-Type: "application/json"
      X-Client-Version: "1.2.3"
      
    # Default timeout and retry settings
    timeout: 8000
    retries: 2
    
    # Default validation rules
    validation:
      statusCodes: [200, 201, 202]
      maxResponseTime: 5000
      
    # Default query parameters
    query:
      api_version: "2023-01-01"
      format: "json"

requests:
  - name: Inherits All Defaults
    url: https://api.example.com/users
    method: GET
    # All global defaults are applied
    
  - name: Overrides Some Defaults
    url: https://api.example.com/posts
    method: POST
    headers:
      Content-Type: "application/xml"  # Overrides default
      # Other headers from defaults still apply
    timeout: 15000  # Overrides default timeout
```

## Global Variables

Define reusable variables that can be referenced throughout your request configurations.

**global-variables.yaml**

```yaml
# Global variables for reuse across requests
global:
  variables:
    # API configuration
    BASE_URL: https://api.example.com
    API_VERSION: v1
    API_KEY: your-secret-api-key
    
    # Environment-specific variables
    ENVIRONMENT: \${ENV.NODE_ENV || 'development'}
    DEBUG_MODE: \${ENVIRONMENT === 'development'}
    
    # Computed variables
    API_ENDPOINT: "\${BASE_URL}/\${API_VERSION}"
    AUTH_HEADER: "Bearer \${API_KEY}"
    TIMEOUT: \${DEBUG_MODE ? 30000 : 5000}
    
  defaults:
    headers:
      Authorization: \${AUTH_HEADER}
    timeout: \${TIMEOUT}

requests:
  - name: Get Users
    url: \${API_ENDPOINT}/users
    method: GET
    
  - name: Create User
    url: \${API_ENDPOINT}/users
    method: POST
    body:
      name: "John Doe"
      environment: \${ENVIRONMENT}
```

## Advanced Configuration

Advanced global settings for complex scenarios including SSL, proxies, and rate limiting.

Some advanced features like SSL certificates, proxies, and rate limiting may require additional setup or may not be available in all environments.

**advanced-global-config.yaml**

```yaml
# Advanced global configuration
global:
  # Execution control
  execution: parallel
  continueOnError: true
  maxConcurrency: 5  # Limit concurrent requests in parallel mode
  
  # Advanced timeout settings
  timeout: 10000
  connectionTimeout: 5000
  readTimeout: 15000
  retries: 3
  retryDelay: 1000
  retryBackoff: 2.0
  retryOn: ["timeout", "5xx", "network"]
  
  # SSL/TLS settings
  ssl:
    verify: true
    ca: "./certs/ca.pem"
    cert: "./certs/client.pem"
    key: "./certs/client-key.pem"
  
  # Proxy settings
  proxy:
    http: "http://proxy.company.com:8080"
    https: "https://secure-proxy.company.com:8443"
    bypass: ["localhost", "*.internal.com"]
    
  # Rate limiting
  rateLimit:
    maxRequests: 100
    perSecond: 10
    
  # Output configuration
  output:
    verbose: true
    saveToFile: "results-\${Date.now()}.json"
    format: "detailed"
    colors: true
    includeMetrics: true
    
  # Global variables
  variables:
    BASE_URL: \${ENV.API_BASE_URL || 'https://api.example.com'}
    API_KEY: \${ENV.API_KEY}
    TRACE_ID: \${crypto.randomUUID()}
    
  # Global defaults
  defaults:
    headers:
      User-Agent: "curl-runner/1.0.0"
      X-Trace-ID: \${TRACE_ID}
      Authorization: "Bearer \${API_KEY}"
    timeout: 8000
    retries: 2
```

## Setting Precedence

Understanding how individual request settings override global settings.

**setting-precedence.yaml**

```yaml
# How individual requests override global settings
global:
  execution: sequential
  timeout: 5000
  retries: 2
  defaults:
    headers:
      Accept: "application/json"
      User-Agent: "curl-runner/1.0.0"

requests:
  - name: Uses Global Settings
    url: https://api.example.com/users
    method: GET
    # Inherits: timeout=5000, retries=2, headers from defaults
    
  - name: Overrides Global Settings  
    url: https://slow-api.example.com/data
    method: GET
    timeout: 30000     # Overrides global timeout
    retries: 5         # Overrides global retries
    headers:
      Accept: "application/xml"  # Overrides global default
      User-Agent: "MyApp/2.0.0"  # Overrides global default
      Custom-Header: "value"     # Adds new header
```

## Best Practices

### Best Practices

• Use descriptive variable names
• Define common values as variables
• Use environment variables for secrets
• Group related variables logically
• Document complex expressions

## Alternative Configuration

> Override global settings using CURL_RUNNER_* environment variables without modifying YAML files
