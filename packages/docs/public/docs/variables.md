---
title: "Variables"
description: "Use variables and templating to create reusable, dynamic HTTP request configurations."
---

# Variables

Use variables and templating to create reusable, dynamic HTTP request configurations.

## Table of Contents

- [Variable Scopes](#variable-scopes)
  - [Global Variables](#global-variables)
  - [Collection Variables](#collection-variables)
  - [Environment Variables](#environment-variables)
- [Variable Precedence](#variable-precedence)
- [Dynamic Variables](#dynamic-variables)
  - [Built-in Functions](#built-in-functions)
- [Conditional Logic](#conditional-logic)
- [Complex Interpolation](#complex-interpolation)
- [Best Practices](#best-practices)
- [Common Patterns](#common-patterns)
  - [API Authentication](#api-authentication)
  - [Environment-Specific URLs](#environment-specific-urls)
  - [Request Correlation IDs](#request-correlation-ids)

## Variable Scopes

### Global Variables

```yaml title="global-variables.yaml"
# Global variables available to all requests
global:
  variables:
    BASE_URL: https://api.example.com
    API_VERSION: v1
    API_TOKEN: your-secret-token
    TIMEOUT: 5000

requests:
  - name: Get Users
    url: \${BASE_URL}/\${API_VERSION}/users
    method: GET
    headers:
      Authorization: Bearer \${API_TOKEN}
    timeout: \${TIMEOUT}
```

### Collection Variables

```yaml title="collection-variables.yaml"
# Collection-level variables
global:
  variables:
    BASE_URL: https://api.example.com
    
collection:
  name: User Management Tests
  variables:
    USER_ID: 123
    ADMIN_TOKEN: admin-secret-token
    
  requests:
    - name: Get User Profile
      url: \${BASE_URL}/users/\${USER_ID}
      headers:
        Authorization: Bearer \${ADMIN_TOKEN}
```

### Environment Variables

```yaml title="env-variables.yaml"
# Using environment variables
global:
  variables:
    # Environment variables are automatically available
    BASE_URL: \${ENV.API_BASE_URL}
    API_KEY: \${ENV.API_KEY}
    DEBUG: \${ENV.DEBUG}

requests:
  - name: API Call with Env Variables
    url: \${BASE_URL}/endpoint
    headers:
      X-API-Key: \${API_KEY}
    # Conditional behavior based on environment
    timeout: \${DEBUG ? 30000 : 5000}
```

## Variable Precedence

> **Precedence Order (Highest to Lowest)**
>


```yaml title="variable-precedence.yaml"
# Variable scoping and precedence
global:
  variables:
    SHARED_VAR: "global-value"
    BASE_URL: https://api.example.com
    
collection:
  variables:
    SHARED_VAR: "collection-value"  # Overrides global
    COLLECTION_VAR: "collection-only"
    
  defaults:
    headers:
      X-Shared: \${SHARED_VAR}  # Will be "collection-value"
      X-Collection: \${COLLECTION_VAR}
      
  requests:
    - name: Test Variable Precedence
      url: \${BASE_URL}/test
      # SHARED_VAR = "collection-value" (collection overrides global)
      # COLLECTION_VAR = "collection-only"
      # BASE_URL = "https://api.example.com" (from global)
```

## Dynamic Variables

### Built-in Functions

> **Date & Time**
>


> **Random Values**
>


```yaml title="dynamic-variables.yaml"
# Dynamic variables and computed values
global:
  variables:
    BASE_URL: https://api.example.com
    TIMESTAMP: \${Date.now()}
    UUID: \${crypto.randomUUID()}
    CURRENT_DATE: \${new Date().toISOString().split('T')[0]}

requests:
  - name: Create Resource with Dynamic Data
    url: \${BASE_URL}/resources
    method: POST
    body:
      id: \${UUID}
      timestamp: \${TIMESTAMP}
      created_date: \${CURRENT_DATE}
      name: "Resource-\${TIMESTAMP}"
```

## Conditional Logic

```yaml title="conditional-variables.yaml"
# Conditional variables and expressions
global:
  variables:
    ENV: production
    DEBUG_MODE: false
    API_TIMEOUT: \${ENV === 'development' ? 30000 : 5000}
    LOG_LEVEL: \${DEBUG_MODE ? 'debug' : 'info'}
    
collection:
  variables:
    # Conditional API endpoints
    BASE_URL: \${ENV === 'production' ? 'https://api.example.com' : 'https://api-staging.example.com'}
    
  requests:
    - name: Environment-Aware Request
      url: \${BASE_URL}/data
      timeout: \${API_TIMEOUT}
      headers:
        X-Log-Level: \${LOG_LEVEL}
        X-Debug: \${DEBUG_MODE ? 'true' : 'false'}
```

## Complex Interpolation

```yaml title="complex-interpolation.yaml"
# Complex variable interpolation
global:
  variables:
    API_HOST: api.example.com
    API_PORT: 443
    API_PROTOCOL: https
    API_PATH: /v1/api
    
    # Computed base URL
    BASE_URL: "\${API_PROTOCOL}://\${API_HOST}:\${API_PORT}\${API_PATH}"
    
    # User data
    USER_NAME: john.doe
    USER_DOMAIN: example.com
    USER_EMAIL: "\${USER_NAME}@\${USER_DOMAIN}"

requests:
  - name: Complex Interpolation Example
    url: \${BASE_URL}/users/search
    method: POST
    body:
      query: "email:\${USER_EMAIL}"
      filters:
        domain: \${USER_DOMAIN}
        username: \${USER_NAME}
```

## Best Practices

> **Best Practices**
>
> • Use descriptive variable names
>
> • Define common values as variables
>
> • Use environment variables for secrets
>
> • Group related variables logically
>
> • Document complex expressions

> **Avoid These Mistakes**
>
> • Hard-coding sensitive information
>
> • Using overly complex expressions
>
> • Creating circular references
>
> • Overusing dynamic variables
>
> • Ignoring variable naming conventions

## Common Patterns

### API Authentication

```yaml title="API Authentication Pattern"
global:
  variables:
    
  defaults:
    headers:
      Authorization: \${AUTH_HEADER}
```

### Environment-Specific URLs

```yaml title="Environment-Specific URLs"
global:
  variables:
    BASE_URL: \${ENVIRONMENT === 'production' 
      ? 'https://api.example.com' 
      : 'https://api-staging.example.com'}
```

### Request Correlation IDs

```yaml title="Request Correlation IDs"
global:
  variables:
    
  defaults:
    headers:
      X-Request-Time: \${Date.now()}
```

