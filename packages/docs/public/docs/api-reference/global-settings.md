---
title: "Global Settings API Reference"
description: "Complete reference for global configuration options that control execution behavior, output formatting, and default request settings."
category: "Configuration"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - global
  - settings
  - reference
  - variables
  - authentication
  - parallel
  - sequential
  - validation
  - retry
  - timeout
  - headers
  - response
  - request
  - collection
  - cli
  - environment
slug: "/docs/global-settings"
toc: true
date: "2026-01-24T16:05:37.563Z"
lastModified: "2026-01-24T16:05:37.563Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Global Settings API Reference"
  category: "Configuration"
tags:
  - documentation
  - configuration
og:
  title: "Global Settings API Reference - curl-runner Documentation"
  description: "Complete reference for global configuration options that control execution behavior, output formatting, and default request settings."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Global Settings API Reference"
  description: "Complete reference for global configuration options that control execution behavior, output formatting, and default request settings."
  datePublished: "2026-01-24T16:05:37.563Z"
  dateModified: "2026-01-24T16:05:37.563Z"
---

# Global Settings API Reference

Complete reference for global configuration options that control execution behavior, output formatting, and default request settings.

## Overview

Global settings control the overall behavior of `curl-runner` execution, including how requests are processed, output formatting, default values, and variable management. These settings apply to all requests unless overridden at the collection or request level.

**global-config-complete.yaml**

```yaml
# Complete Global Configuration Example
global:
  # Execution behavior
  execution: sequential
  continueOnError: false
  
  # Global variables available to all requests
  variables:
    BASE_URL: "https://api.example.com"
    API_VERSION: "v2"
    TIMEOUT_MS: "5000"
    USER_AGENT: "curl-runner/1.0.0"
    
  # Output configuration
  output:
    verbose: true
    showHeaders: true
    showBody: true
    showMetrics: true
    format: pretty
    prettyLevel: standard  # minimal, standard, or detailed
    saveToFile: "results/api-test-results.json"
    
  # Default settings applied to all requests
  defaults:
    timeout: \${TIMEOUT_MS}
    headers:
      User-Agent: \${USER_AGENT}
      Accept: "application/json"
      X-API-Version: \${API_VERSION}
    followRedirects: true
    maxRedirects: 3
    retry:
      count: 2
      delay: 1000
    expect:
      headers:
        content-type: "application/json"
```

## Global Configuration Properties

## Execution Modes

Control how multiple requests are executed: sequentially (one after another) or in parallel (simultaneously).

**execution-modes.yaml**

```yaml
# Execution Mode Examples

# Sequential execution (default)
global:
  execution: sequential  # Requests run one after another
  continueOnError: false # Stop on first error
  
collection:
  name: "Sequential API Tests"
  requests:
    - name: "Step 1: Login"
      url: "https://api.example.com/login"
      method: POST
      # Must complete before next request
      
    - name: "Step 2: Get Data"  
      url: "https://api.example.com/data"
      method: GET
      # Uses login token from previous request
      
    - name: "Step 3: Process"
      url: "https://api.example.com/process"
      method: POST
      # Depends on data from step 2

---

# Parallel execution for performance
global:
  execution: parallel    # All requests run simultaneously
  continueOnError: true  # Continue even if some fail
  
collection:
  name: "Load Testing"
  requests:
    - name: "Load Test 1"
      url: "https://api.example.com/endpoint1"
      
    - name: "Load Test 2"
      url: "https://api.example.com/endpoint2"
      
    - name: "Load Test 3"
      url: "https://api.example.com/endpoint3"
      # All three run at the same time
```

## Error Handling Configuration

Configure how `curl-runner` responds to request failures and validation errors.

Stop execution on first error. Best for critical workflows.

Continue despite errors. Best for comprehensive testing.

**error-handling.yaml**

```yaml
# Error Handling Configuration

# Stop on first error (strict mode)
global:
  continueOnError: false  # Default behavior
  execution: sequential
  
collection:
  requests:
    - name: "Critical Setup"
      url: "https://api.example.com/setup"
      method: POST
      # If this fails, execution stops
      
    - name: "Dependent Operation"
      url: "https://api.example.com/operation"
      method: POST
      # Only runs if setup succeeds

---

# Continue on error (resilient mode)  
global:
  continueOnError: true   # Keep going despite failures
  execution: sequential
  
collection:
  requests:
    - name: "Optional Check 1"
      url: "https://api.example.com/check1"
      # If fails, continue to next
      
    - name: "Optional Check 2"
      url: "https://api.example.com/check2"
      # Runs regardless of check1 result
      
    - name: "Important Operation"
      url: "https://api.example.com/important"
      # Runs regardless of previous failures
```

## Global Variables

Define variables that are available to all requests in the configuration. Variables support environment variable interpolation and can be overridden at collection and request levels.

**global-variables.yaml**

```yaml
# Global Variables Configuration

global:
  variables:
    # Environment configuration
    BASE_URL: "https://api.production.com"
    API_VERSION: "v2"
    ENVIRONMENT: "production"
    
    # Authentication
    API_KEY: "\${API_KEY}"        # From environment
    CLIENT_ID: "\${CLIENT_ID}"    # From environment
    
    # Timeouts and limits
    DEFAULT_TIMEOUT: "5000"
    MAX_RETRIES: "3"
    RATE_LIMIT: "100"
    
    # Common values
    CONTENT_TYPE: "application/json"
    USER_AGENT: "MyApp/1.0.0 (production)"
    
    # Dynamic values (can be overridden)
    REQUEST_ID: "req_\${TIMESTAMP}"
    TRACE_ID: "trace_\${UUID}"

collection:
  variables:
    # Override global variables at collection level
    ENVIRONMENT: "staging"  # Override global
    COLLECTION_ID: "api_tests_v2"
    
  requests:
    - name: "Use Global Variables"
      url: "\${BASE_URL}/\${API_VERSION}/users"
      headers:
        Authorization: "Bearer \${API_KEY}"
        User-Agent: "\${USER_AGENT}"
        X-Environment: "\${ENVIRONMENT}"
        X-Request-ID: "\${REQUEST_ID}"
      timeout: \${DEFAULT_TIMEOUT}
      
    - name: "Override Variables"
      url: "\${BASE_URL}/\${API_VERSION}/slow-endpoint"
      variables:
        # Local override for this request only
        DEFAULT_TIMEOUT: "30000"  # 30 seconds for slow endpoint
      timeout: \${DEFAULT_TIMEOUT}
```

## Output Configuration

Control how results are displayed and saved, including verbosity, format, and file output options.

The pretty format supports three verbosity levels to control how much information is displayed.

**output-config.yaml**

```yaml
# Output Configuration Options

# Minimal output - only basic status
global:
  output:
    format: pretty
    prettyLevel: minimal  # Only show status and errors
    showHeaders: false
    showBody: false
    showMetrics: false

---

# Standard output - body and metrics when enabled
global:
  output:
    format: pretty
    prettyLevel: standard  # Show body/metrics if enabled
    showHeaders: false
    showBody: true        # Will show body with standard level
    showMetrics: true     # Will show basic metrics

---

# Detailed output - everything visible
global:
  output:
    format: pretty
    prettyLevel: detailed # Always show all information
    # prettyLevel: detailed overrides individual flags

---

# JSON output for CI/CD
global:
  output:
    format: json      # Structured JSON output
    showHeaders: true
    showBody: true
    showMetrics: true
    saveToFile: "test-results/\${DATE}-api-tests.json"

---

# Raw output for data processing
global:
  output:
    format: raw       # Raw response content only
    showHeaders: false
    showBody: true
    saveToFile: "data.txt"
```

## Default Request Settings

Configure default values that apply to all requests, reducing repetition and ensuring consistency across your API configurations.

### Setting Precedence Order

Values defined directly on individual requests have the highest priority and override all defaults.

**defaults-config.yaml**

```yaml
# Default Request Settings

global:
  defaults:
    # Timeouts and retries
    timeout: 10000        # 10 seconds default
    followRedirects: true
    maxRedirects: 5
    retry:
      count: 3
      delay: 2000
      
    # Common headers for all requests
    headers:
      User-Agent: "curl-runner/1.0.0"
      Accept: "application/json"
      Content-Type: "application/json"
      X-Client-Version: "1.2.3"
      
    # Security settings
    insecure: false       # Verify SSL certificates
    
    # Response validation
    expect:
      headers:
        content-type: "application/json"  # Expect JSON responses
        
collection:
  name: "API Tests with Defaults"
  
  # Collection-level defaults override global
  defaults:
    headers:
      Authorization: "Bearer \${API_TOKEN}"  # Add auth to all
      X-Collection: "api-tests"
    timeout: 15000  # Override global timeout
    
  requests:
    - name: "Quick Request"
      url: "https://api.example.com/quick"
      # Uses all defaults from global and collection
      
    - name: "Custom Request"
      url: "https://api.example.com/custom"
      timeout: 30000  # Override timeout just for this request
      headers:
        X-Special: "custom-header"  # Adds to default headers
        Authorization: "Basic xyz"  # Overrides collection default
      
    - name: "Slow Request"
      url: "https://api.example.com/slow"
      retry:
        count: 5      # Override default retry count
        delay: 5000   # Override default retry delay
```

## Production Configuration Example

A comprehensive example showing how to configure `curl-runner` for production monitoring and CI/CD integration.

**production-config.yaml**

```yaml
# Complex Production Configuration

global:
  # Execution settings
  execution: sequential
  continueOnError: false
  
  # Environment variables
  variables:
    # Load balancer endpoints
    API_PRIMARY: "https://api-primary.company.com"
    API_BACKUP: "https://api-backup.company.com"
    
    # Authentication
    SERVICE_ACCOUNT_TOKEN: "\${SERVICE_TOKEN}"
    CLIENT_CERT_PATH: "/etc/ssl/certs/client.pem"
    
    # Rate limiting
    REQUESTS_PER_MINUTE: "60"
    BURST_LIMIT: "10"
    
    # Monitoring
    TRACE_ENDPOINT: "https://tracing.company.com/v1/traces"
    LOG_LEVEL: "info"
    
  # Output for CI/CD integration
  output:
    verbose: true
    showHeaders: false    # Reduce log noise
    showBody: false      # Don't log sensitive data
    showMetrics: true    # Track performance
    format: json         # Machine-readable
    saveToFile: "artifacts/api-test-results.json"
    
  # Production defaults
  defaults:
    timeout: 30000       # 30 seconds for production APIs
    followRedirects: true
    maxRedirects: 3
    insecure: false      # Always verify certificates
    
    # Standard headers
    headers:
      User-Agent: "curl-runner-ci/1.0.0"
      Accept: "application/json"
      X-Environment: "production"
      X-Service: "api-tests"
      
    # Retry configuration for reliability
    retry:
      count: 3
      delay: 5000        # 5 second delays
      
    # Basic validation
    expect:
      headers:
        content-type: "application/json"
        x-rate-limit-remaining: "*"  # Ensure rate limits tracked
        
collection:
  name: "Production Health Checks"
  description: "Critical API endpoints monitoring"
  
  variables:
    HEALTH_CHECK_VERSION: "v1"
    ALERT_WEBHOOK: "\${SLACK_WEBHOOK_URL}"
    
  defaults:
    headers:
      Authorization: "Bearer \${SERVICE_ACCOUNT_TOKEN}"
      X-Health-Check: "automated"
      
  requests:
    - name: "Primary API Health"
      url: "\${API_PRIMARY}/\${HEALTH_CHECK_VERSION}/health"
      method: GET
      expect:
        status: 200
        body:
          status: "healthy"
          version: "*"
          timestamp: "*"
          
    - name: "Database Connection Check"
      url: "\${API_PRIMARY}/\${HEALTH_CHECK_VERSION}/health/database"
      method: GET
      timeout: 60000     # Database checks can be slow
      expect:
        status: 200
        body:
          database:
            status: "connected"
            response_time: "^[0-9]+ms$"
            
    - name: "Cache Health Check"  
      url: "\${API_PRIMARY}/\${HEALTH_CHECK_VERSION}/health/cache"
      method: GET
      expect:
        status: 200
        body:
          cache:
            status: "operational"
            hit_rate: "*"
            
    - name: "External Service Dependencies"
      url: "\${API_PRIMARY}/\${HEALTH_CHECK_VERSION}/health/dependencies"
      method: GET
      timeout: 45000     # External services may be slow
      expect:
        status: [200, 207]  # 207 for partial failures
        body:
          dependencies:
            - name: "*"
              status: ["healthy", "degraded"]
```

## Environment Integration

```bash
# .env file
API_TOKEN=your_secret_token
BASE_URL=https://api.staging.com
DEBUG_MODE=true
```

```bash
# Run with specific environment
curl-runner config/production.yaml
curl-runner config/staging.yaml
curl-runner config/development.yaml
```

## Global Configuration Best Practices
