---
title: "YAML Structure"
description: "Learn the structure and syntax of curl-runner YAML configuration files."
---

# YAML Structure

Learn the structure and syntax of curl-runner YAML configuration files.

## Table of Contents

- [Basic Structure](#basic-structure)
  - [Multiple Requests](#multiple-requests)
- [Request Properties](#request-properties)
  - [Required Properties](#required-properties)
  - [Optional Properties](#optional-properties)
- [Global Configuration](#global-configuration)
- [Response Validation](#response-validation)

## Basic Structure

```yaml title="single-request.yaml"
# Single HTTP request
request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET
  headers:
    Authorization: Bearer $\{API_TOKEN}
    Content-Type: application/json
```

### Multiple Requests

```yaml title="multiple-requests.yaml"
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
      Authorization: Bearer $\{API_TOKEN}
```

## Request Properties

### Required Properties

| Property | Type | Description |
| --- | --- | --- |
| url | string | The URL to send the request to |
| method | string | HTTP method (GET, POST, PUT, DELETE, etc.) |


### Optional Properties

| Property | Type | Description |
| --- | --- | --- |
| name | string | Display name for the request |
| headers | object | HTTP headers to send with the request |
| body | object | string | Request body data |
| timeout | number | Request timeout in milliseconds |
| retries | number | Number of retry attempts |
| validation | object | Response validation rules |


## Global Configuration

| Setting | Type | Description |
| --- | --- | --- |
| variables | object | Global variables available to all requests |
| execution | string | "sequential" or "parallel" execution mode |
| continueOnError | boolean | Continue execution if a request fails |
| output.verbose | boolean | Show detailed output during execution |
| output.saveToFile | string | Save results to a JSON file |


## Response Validation

```yaml title="validation-example.yaml"
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

