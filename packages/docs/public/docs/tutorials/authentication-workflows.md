---
title: "Authentication Workflows with Response Storage"
description: "Build authentication flows that extract tokens from login responses and use them in subsequent requests. Master response storage for chaining API calls."
category: "Tutorials"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - tutorial
  - authentication
  - response-storage
  - tokens
  - workflow
slug: "/docs/tutorials/authentication-workflows"
toc: true
date: "2026-01-24T00:00:00.000Z"
lastModified: "2026-01-24T00:00:00.000Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Authentication Workflows"
  category: "Tutorials"
tags:
  - tutorials
  - authentication
og:
  title: "Authentication Workflows with Response Storage - curl-runner Documentation"
  description: "Build authentication flows that extract tokens from login responses."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Authentication Workflows with Response Storage"
  description: "Build authentication flows that extract tokens from login responses."
  datePublished: "2026-01-24T00:00:00.000Z"
  dateModified: "2026-01-24T00:00:00.000Z"
---

# Authentication Workflows with Response Storage

In this tutorial, you'll learn how to build authentication workflows that extract data from responses and use it in subsequent requests.

## Understanding Response Storage

Response storage allows you to extract values from a response and use them in later requests. This is essential for:

- Extracting authentication tokens from login responses
- Using created resource IDs in follow-up requests
- Building request chains that depend on previous responses

**Important:** Response storage only works in sequential execution mode (the default).

## Basic Response Storage

Here's a simple example that extracts a user ID from the first request:

```yaml
global:
  variables:
    BASE_URL: https://jsonplaceholder.typicode.com

requests:
  - name: Get User
    url: ${BASE_URL}/users/1
    method: GET
    expect:
      status: 200
    store:
      userId: body.id
      userName: body.name

  - name: Get User Posts
    url: ${BASE_URL}/users/${store.userId}/posts
    method: GET
    expect:
      status: 200
```

The `store` section extracts values using dot notation paths:
- `body.id` extracts the `id` field from the response body
- `body.name` extracts the `name` field

Stored values are accessed with `${store.variableName}`.

## Authentication Token Flow

Here's a realistic authentication workflow:

```yaml
global:
  variables:
    API_URL: https://api.example.com
  execution: sequential

requests:
  - name: Login
    url: ${API_URL}/auth/login
    method: POST
    headers:
      Content-Type: application/json
    body:
      email: user@example.com
      password: secretpassword
    expect:
      status: 200
      body:
        success: true
    store:
      authToken: body.token
      refreshToken: body.refreshToken

  - name: Get Protected Resource
    url: ${API_URL}/api/profile
    method: GET
    headers:
      Authorization: Bearer ${store.authToken}
    expect:
      status: 200

  - name: Update Profile
    url: ${API_URL}/api/profile
    method: PUT
    headers:
      Authorization: Bearer ${store.authToken}
      Content-Type: application/json
    body:
      name: Updated Name
    expect:
      status: 200
```

## What You Can Store

Response storage supports extracting from multiple sources:

### Body Fields

```yaml
store:
  userId: body.id                    # Top-level field
  token: body.data.token             # Nested field
  firstName: body.user.profile.name  # Deep nesting
```

### Array Elements

```yaml
store:
  firstItem: body.items.0            # First element (index 0)
  secondItem: body.items.1           # Second element
  lastItem: body.items.-1            # Last element (negative index)
```

### Response Headers

```yaml
store:
  contentType: headers.content-type
  requestId: headers.x-request-id
  rateLimit: headers.x-rate-limit-remaining
```

### Status and Metrics

```yaml
store:
  statusCode: status                 # HTTP status code
  duration: metrics.duration         # Request duration in ms
  responseSize: metrics.size         # Response size in bytes
```

## CRUD Workflow Example

A complete Create-Read-Update-Delete workflow:

```yaml
global:
  variables:
    BASE_URL: https://jsonplaceholder.typicode.com
  execution: sequential

requests:
  - name: Create Post
    url: ${BASE_URL}/posts
    method: POST
    headers:
      Content-Type: application/json
    body:
      title: My New Post
      body: This is the content
      userId: 1
    expect:
      status: 201
    store:
      postId: body.id

  - name: Read Created Post
    url: ${BASE_URL}/posts/${store.postId}
    method: GET
    expect:
      status: 200
      body:
        title: My New Post

  - name: Update Post
    url: ${BASE_URL}/posts/${store.postId}
    method: PUT
    headers:
      Content-Type: application/json
    body:
      id: ${store.postId}
      title: Updated Title
      body: Updated content
      userId: 1
    expect:
      status: 200

  - name: Delete Post
    url: ${BASE_URL}/posts/${store.postId}
    method: DELETE
    expect:
      status: 200
```

## Built-in Auth Support

curl-runner has built-in authentication support for common patterns:

### Basic Authentication

```yaml
request:
  name: Basic Auth Request
  url: https://api.example.com/secure
  method: GET
  auth:
    type: basic
    username: myuser
    password: mypassword
  expect:
    status: 200
```

### Bearer Token Authentication

```yaml
request:
  name: Bearer Auth Request
  url: https://api.example.com/secure
  method: GET
  auth:
    type: bearer
    token: ${API_TOKEN}
  expect:
    status: 200
```

## Multi-Service Authentication

Handle multiple services with different auth requirements:

```yaml
global:
  variables:
    AUTH_SERVICE: https://auth.example.com
    API_SERVICE: https://api.example.com
    DATA_SERVICE: https://data.example.com
  execution: sequential

requests:
  - name: Get Auth Token
    url: ${AUTH_SERVICE}/oauth/token
    method: POST
    headers:
      Content-Type: application/x-www-form-urlencoded
    body:
      grant_type: client_credentials
      client_id: my-client
      client_secret: my-secret
    expect:
      status: 200
    store:
      accessToken: body.access_token
      tokenType: body.token_type

  - name: Call API Service
    url: ${API_SERVICE}/resources
    method: GET
    headers:
      Authorization: ${store.tokenType} ${store.accessToken}
    expect:
      status: 200
    store:
      resourceId: body.data.0.id

  - name: Fetch Resource from Data Service
    url: ${DATA_SERVICE}/items/${store.resourceId}
    method: GET
    headers:
      Authorization: ${store.tokenType} ${store.accessToken}
    expect:
      status: 200
```

## Error Handling in Auth Flows

Handle authentication failures gracefully:

```yaml
global:
  continueOnError: false  # Stop on first failure
  execution: sequential

requests:
  - name: Login
    url: ${API_URL}/auth/login
    method: POST
    body:
      email: user@example.com
      password: wrongpassword
    expect:
      status: 401
      failure: true  # Expect this to fail
    store:
      errorMessage: body.message

  - name: Login with Correct Password
    url: ${API_URL}/auth/login
    method: POST
    body:
      email: user@example.com
      password: correctpassword
    expect:
      status: 200
    store:
      authToken: body.token
```

## What You Learned

In this tutorial, you learned how to:

- Use response storage to extract values from responses
- Build authentication token workflows
- Chain requests that depend on previous responses
- Extract data from body, headers, status, and metrics
- Use built-in authentication helpers
- Handle multi-service authentication scenarios

## Next Steps

- [CI/CD Integration](/docs/tutorials/ci-cd-integration) - Automate your auth tests in pipelines
- [Advanced Validation](/docs/tutorials/advanced-validation) - Validate complex auth responses
- [Variables Reference](/docs/variables) - Learn about dynamic variables
