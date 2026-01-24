---
title: "YAML Structure"
description: "Learn the structure and syntax of curl-runner YAML configuration files."
category: "Configuration"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - yaml
  - structure
  - variables
  - parallel
  - sequential
  - validation
  - retry
  - timeout
  - headers
  - response
  - request
  - collection
  - environment
slug: "/docs/yaml-structure"
toc: true
date: "2026-01-24T16:03:18.277Z"
lastModified: "2026-01-24T16:03:18.277Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "YAML Structure"
  category: "Configuration"
tags:
  - documentation
  - configuration
og:
  title: "YAML Structure - curl-runner Documentation"
  description: "Learn the structure and syntax of curl-runner YAML configuration files."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "YAML Structure"
  description: "Learn the structure and syntax of curl-runner YAML configuration files."
  datePublished: "2026-01-24T16:03:18.277Z"
  dateModified: "2026-01-24T16:03:18.277Z"
---

# YAML Structure

Learn the structure and syntax of curl-runner YAML configuration files.

## Basic Structure

`curl-runner` uses YAML files to define HTTP requests. There are several ways to structure your configuration files.

### Single Request <Badge variant="secondary" className="ml-2"> Basic </Badge>

The simplest form - define a single HTTP request.

**single-request.yaml**

```yaml
# Single HTTP request
request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET
  headers:
    Authorization: Bearer \${API_TOKEN}
    Content-Type: application/json
```

### Multiple Requests

Execute multiple HTTP requests in sequence or parallel.

**multiple-requests.yaml**

```yaml
# Multiple requests collection
requests:
  - name: Create User
    url: https://api.example.com/users
    method: POST
    headers:
      Content-Type: application/json
    body:
      name: John Doe
      email: john@example.com
      
  - name: Get Created User
    url: https://api.example.com/users/\${USER_ID}
    method: GET
    headers:
      Authorization: Bearer \${API_TOKEN}
```

### Collection <Badge className="bg-cyan-500/10 text-cyan-600 dark:text-cyan-400 border-cyan-500/20 ml-2"> Advanced </Badge>

Advanced structure with global settings, variables, and defaults.

**collection.yaml**

```yaml
# Advanced collection with global settings
global:
  variables:
    BASE_URL: https://api.example.com
    API_TOKEN: your-api-token-here
  execution: parallel
  continueOnError: true
  output:
    verbose: true
    saveToFile: results.json

collection:
  name: User Management API Tests
  variables:
    USER_ID: 123
  defaults:
    headers:
      Authorization: Bearer \${API_TOKEN}
      Content-Type: application/json
  
  requests:
    - name: List Users
      url: \${BASE_URL}/users
      method: GET
      
    - name: Get Specific User
      url: \${BASE_URL}/users/\${USER_ID}
      method: GET
      
    - name: Update User
      url: \${BASE_URL}/users/\${USER_ID}
      method: PATCH
      body:
        name: Updated Name
```

**single-request.yaml**

```yaml
# Single HTTP request
request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET
  headers:
    Authorization: Bearer \${API_TOKEN}
    Content-Type: application/json
```

**multiple-requests.yaml**

```yaml
# Multiple requests collection
requests:
  - name: Create User
    url: https://api.example.com/users
    method: POST
    headers:
      Content-Type: application/json
    body:
      name: John Doe
      email: john@example.com
      
  - name: Get Created User
    url: https://api.example.com/users/\${USER_ID}
    method: GET
    headers:
      Authorization: Bearer \${API_TOKEN}
```

**collection.yaml**

```yaml
# Advanced collection with global settings
global:
  variables:
    BASE_URL: https://api.example.com
    API_TOKEN: your-api-token-here
  execution: parallel
  continueOnError: true
  output:
    verbose: true
    saveToFile: results.json

collection:
  name: User Management API Tests
  variables:
    USER_ID: 123
  defaults:
    headers:
      Authorization: Bearer \${API_TOKEN}
      Content-Type: application/json
  
  requests:
    - name: List Users
      url: \${BASE_URL}/users
      method: GET
      
    - name: Get Specific User
      url: \${BASE_URL}/users/\${USER_ID}
      method: GET
      
    - name: Update User
      url: \${BASE_URL}/users/\${USER_ID}
      method: PATCH
      body:
        name: Updated Name
```

## Request Properties

Each request can have the following properties:

### Required Properties

| Property | Type | Description |
| --- | --- | --- |
| `url` | string | The URL to send the request to |
| `method` | string | HTTP method (GET, POST, PUT, DELETE, etc.) |

### Optional Properties

| Property | Type | Description |
| --- | --- | --- |
| `name` | string | Display name for the request |
| `headers` | object | HTTP headers to send with the request |
| `body` | object | string | Request body data |
| `timeout` | number | Request timeout in milliseconds |
| `retries` | number | Number of retry attempts |
| `validation` | object | Response validation rules |

## Global Configuration

The `global` section allows you to configure settings that apply to all requests in the file.

## Response Validation

Add validation rules to verify that responses meet your expectations.

**validation-example.yaml**

```yaml
# Request with response validation
request:
  name: API Health Check
  url: https://api.example.com/health
  method: GET
  validation:
    status: 200
    headers:
      content-type: application/json
    body:
      status: ok
      version: "^1.0.0"
  timeout: 5000
  retries: 3
```

## Related Topics

> Make your YAML files dynamic with variables, environment values, and computed expressions
