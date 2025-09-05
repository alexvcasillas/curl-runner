---
title: "Variables"
description: "Use variables and templating to create reusable, dynamic HTTP request configurations."
category: "Configuration"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - variables
  - yaml
  - templating
  - authentication
  - headers
  - request
  - collection
  - environment
slug: "/docs/variables"
toc: true
date: "2025-09-05T11:38:17.401Z"
lastModified: "2025-09-05T11:38:17.401Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Variables"
  category: "Configuration"
tags:
  - documentation
  - configuration
og:
  title: "Variables - curl-runner Documentation"
  description: "Use variables and templating to create reusable, dynamic HTTP request configurations."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Variables"
  description: "Use variables and templating to create reusable, dynamic HTTP request configurations."
  datePublished: "2025-09-05T11:38:17.401Z"
  dateModified: "2025-09-05T11:38:17.401Z"
---

# Variables

Use variables and templating to create reusable, dynamic HTTP request configurations.

Variables in curl-runner allow you to create flexible, reusable configurations. You can define variables at different scopes and use them throughout your request definitions with template interpolation.

## Variable Scopes

Variables can be defined at different levels, each with its own scope and precedence rules.

### Global Variables

Global variables are available to all requests in the file and have the lowest precedence.

**global-variables.yaml**

```yaml
global:
  variables:
    BASE_URL: "https://api.example.com"
    API_VERSION: "v1"

collection:
  name: "API Tests"
  requests:
    - name: "Get users"
      url: "\${BASE_URL}/\${API_VERSION}/users"
      method: GET
```

### Collection Variables

Collection variables are scoped to a specific collection and override global variables.

**collection-variables.yaml**

```yaml
global:
  variables:
    TIMEOUT: "5000"      # Global level
    ENV: "production"

collection:
  variables:
    TIMEOUT: "10000"     # Overrides global
    COLLECTION_ID: "api_tests"
    
  requests:
    - name: "Test with overrides"
      variables:
        TIMEOUT: "30000"  # Overrides collection and global
        REQUEST_ID: "req_001"
      url: "\${BASE_URL}/test"
      timeout: \${TIMEOUT}  # Uses 30000
```

### Environment Variables

Access system environment variables using the `ENV` object.

**env-variables.yaml**

```yaml
# Set environment variables
export API_KEY="your-secret-key"
export BASE_URL="https://api.staging.com"

# Reference in YAML
global:
  variables:
    API_TOKEN: "\${API_KEY}"     # From environment
    API_BASE: "\${BASE_URL}"     # From environment
    TIMEOUT: "5000"              # Static value
```

**global-variables.yaml**

```yaml
global:
  variables:
    BASE_URL: "https://api.example.com"
    API_VERSION: "v1"

collection:
  name: "API Tests"
  requests:
    - name: "Get users"
      url: "\${BASE_URL}/\${API_VERSION}/users"
      method: GET
```

**collection-variables.yaml**

```yaml
global:
  variables:
    TIMEOUT: "5000"      # Global level
    ENV: "production"

collection:
  variables:
    TIMEOUT: "10000"     # Overrides global
    COLLECTION_ID: "api_tests"
    
  requests:
    - name: "Test with overrides"
      variables:
        TIMEOUT: "30000"  # Overrides collection and global
        REQUEST_ID: "req_001"
      url: "\${BASE_URL}/test"
      timeout: \${TIMEOUT}  # Uses 30000
```

**env-variables.yaml**

```yaml
# Set environment variables
export API_KEY="your-secret-key"
export BASE_URL="https://api.staging.com"

# Reference in YAML
global:
  variables:
    API_TOKEN: "\${API_KEY}"     # From environment
    API_BASE: "\${BASE_URL}"     # From environment
    TIMEOUT: "5000"              # Static value
```

## Variable Precedence

When variables are defined at multiple levels, curl-runner follows a specific precedence order.

**variable-precedence.yaml**

```yaml
global:
  variables:
    TIMEOUT: "5000"      # Global level
    ENV: "production"

collection:
  variables:
    TIMEOUT: "10000"     # Overrides global
    COLLECTION_ID: "api_tests"
    
  requests:
    - name: "Test with overrides"
      variables:
        TIMEOUT: "30000"  # Overrides collection and global
        REQUEST_ID: "req_001"
      url: "\${BASE_URL}/test"
      timeout: \${TIMEOUT}  # Uses 30000
```

## Dynamic Variables

Create dynamic values using JavaScript expressions and built-in functions.

**dynamic-variables.yaml**

```yaml
global:
  variables:
    # Dynamic timestamp
    TIMESTAMP: "\${DATE:YYYY-MM-DD}"
    REQUEST_TIME: "\${TIME:HH:mm:ss}"
    
    # UUID generation
    REQUEST_ID: "\${UUID}"
    SESSION_ID: "\${UUID:short}"
    
    # Random values
    RANDOM_NUM: "\${RANDOM:1-1000}"
    RANDOM_STR: "\${RANDOM:string:10}"

collection:
  requests:
    - name: "Request with dynamic values"
      url: "https://api.example.com/requests"
      headers:
        X-Request-ID: "\${REQUEST_ID}"
        X-Timestamp: "\${TIMESTAMP}"
        X-Session: "\${SESSION_ID}"
```

## Conditional Logic

Use JavaScript expressions to create conditional variables and environment-specific configurations.

**conditional-variables.yaml**

```yaml
global:
  variables:
    # Environment-based variables
    BASE_URL: "\${NODE_ENV:production:https://api.example.com:https://api-staging.example.com}"
    
    # Default value if environment variable not set
    API_TIMEOUT: "\${API_TIMEOUT:5000}"
    
    # Multiple environment sources
    DB_HOST: "\${DATABASE_HOST:\${DB_HOST:localhost}}"

collection:
  requests:
    - name: "Environment aware request"
      url: "\${BASE_URL}/data"
      timeout: \${API_TIMEOUT}
```

## Complex Interpolation

Combine multiple variables and expressions to create complex, computed values.

**complex-interpolation.yaml**

```yaml
global:
  variables:
    BASE_PATH: "/api/v1"
    RESOURCE: "users"
    
    # Computed from other variables
    FULL_ENDPOINT: "\${BASE_URL}\${BASE_PATH}/\${RESOURCE}"
    
    # String manipulation
    UPPER_ENV: "\${ENV:upper}"
    LOWER_RESOURCE: "\${RESOURCE:lower}"

collection:
  requests:
    - name: "Using computed variables"
      url: "\${FULL_ENDPOINT}"
      headers:
        X-Environment: "\${UPPER_ENV}"
        X-Resource-Type: "\${LOWER_RESOURCE}"
```

## Best Practices

### Best Practices

• Use descriptive variable names
• Define common values as variables
• Use environment variables for secrets
• Group related variables logically
• Document complex expressions

## Common Patterns

### API Authentication

```yaml
global:
  variables:
    ENVIRONMENT: \${ENV.NODE_ENV || 'development'}
    BASE_URL: \${ENVIRONMENT === 'production' 
      ? 'https://api.example.com' 
      : 'https://api-staging.example.com'}
```
