---
title: "Response Storage"
description: "Store response values from one request to use in subsequent requests."
category: "Documentation"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - response
  - storage
  - yaml
  - variables
  - authentication
  - parallel
  - sequential
  - headers
  - request
slug: "/docs/response-storage"
toc: true
date: "2026-01-23T21:27:49.063Z"
lastModified: "2026-01-23T21:27:49.063Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Response Storage"
  category: "Documentation"
tags:
  - documentation
  - documentation
og:
  title: "Response Storage - curl-runner Documentation"
  description: "Store response values from one request to use in subsequent requests."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Response Storage"
  description: "Store response values from one request to use in subsequent requests."
  datePublished: "2026-01-23T21:27:49.063Z"
  dateModified: "2026-01-23T21:27:49.063Z"
---

# Response Storage

Store response values from one request to use in subsequent requests.

## Overview

Response storage allows you to extract data from API responses and use it in subsequent requests. This is essential for building realistic API test workflows where requests depend on each other, such as authentication flows, CRUD operations, and data pipelines.

Extract values from any part of the response using dot-notation paths

Build complex workflows by passing data between sequential requests

## Basic Usage

Use the `store` property in your request to extract and save response values. Reference stored values using the `${store.variableName}` syntax.

**basic-storage.yaml**

```yaml
# Basic response storage
requests:
  # First request: Create a resource and store the ID
  - name: Create User
    url: https://api.example.com/users
    method: POST
    headers:
      Content-Type: application/json
    body:
      name: "John Doe"
      email: "john@example.com"
    store:
      userId: body.id        # Store the returned user ID

  # Second request: Use the stored ID
  - name: Get User Details
    url: https://api.example.com/users/\${store.userId}
    method: GET
    expect:
      status: 200
```

## Path Syntax

The store configuration uses dot-notation paths to extract values from different parts of the response.

**path-reference.yaml**

```yaml
# Path syntax examples
store:
  # Body fields
  id: body.id                    # Top-level field
  name: body.user.name           # Nested field
  token: body.data.auth.token    # Deeply nested

  # Array access
  first: body.items.0            # First array element
  second: body.items.1           # Second element
  nested: body.items.0.id        # Field from first element

  # Headers (case-sensitive)
  contentType: headers.content-type
  authHeader: headers.authorization
  customHeader: headers.x-custom

  # Status code
  status: status                 # HTTP status code

  # Metrics
  duration: metrics.duration     # Request duration in ms
  size: metrics.size             # Response size in bytes
```

## Common Use Cases

### Authentication Flow

Store authentication tokens from login responses to use in subsequent authenticated requests.

**auth-flow.yaml**

```yaml
# Authentication flow with token storage
global:
  execution: sequential  # Required for response storage
  variables:
    API_URL: https://api.example.com

requests:
  # Step 1: Login and store the token
  - name: Login
    url: \${API_URL}/auth/login
    method: POST
    headers:
      Content-Type: application/json
    body:
      username: "admin"
      password: "secret123"
    expect:
      status: 200
    store:
      authToken: body.accessToken
      refreshToken: body.refreshToken
      tokenExpiry: body.expiresIn

  # Step 2: Use token for authenticated request
  - name: Get Protected Resource
    url: \${API_URL}/protected/data
    method: GET
    headers:
      Authorization: Bearer \${store.authToken}
    expect:
      status: 200

  # Step 3: Create a resource with authentication
  - name: Create Resource
    url: \${API_URL}/protected/resources
    method: POST
    headers:
      Authorization: Bearer \${store.authToken}
      Content-Type: application/json
    body:
      name: "New Resource"
    expect:
      status: 201
```

### CRUD Workflow

Create, read, update, and delete resources by passing IDs between requests.

**crud-workflow.yaml**

```yaml
# Complete CRUD workflow
requests:
  # CREATE: Make a new post
  - name: Create Post
    url: https://api.example.com/posts
    method: POST
    headers:
      Content-Type: application/json
    body:
      title: "My First Post"
      content: "Hello, World!"
      published: false
    expect:
      status: 201
    store:
      postId: body.id
      createdAt: body.createdAt

  # READ: Verify the post was created
  - name: Get Post
    url: https://api.example.com/posts/\${store.postId}
    method: GET
    expect:
      status: 200
      body:
        id: "\${store.postId}"

  # UPDATE: Modify the post
  - name: Update Post
    url: https://api.example.com/posts/\${store.postId}
    method: PUT
    headers:
      Content-Type: application/json
    body:
      title: "Updated Title"
      content: "Updated content"
      published: true
    expect:
      status: 200

  # DELETE: Remove the post
  - name: Delete Post
    url: https://api.example.com/posts/\${store.postId}
    method: DELETE
    expect:
      status: 204
```

### Data Extraction

Extract specific data from list responses to use in follow-up requests.

**data-extraction.yaml**

```yaml
# Extracting data for chained requests
requests:
  # Get a list and extract specific items
  - name: List All Users
    url: https://api.example.com/users
    method: GET
    expect:
      status: 200
    store:
      firstUserId: body.users.0.id
      firstUserEmail: body.users.0.email
      totalCount: body.meta.total

  # Use extracted data for follow-up
  - name: Get First User Details
    url: https://api.example.com/users/\${store.firstUserId}
    method: GET
    expect:
      status: 200
      body:
        email: "\${store.firstUserEmail}"

  # Create related resource
  - name: Create User Profile
    url: https://api.example.com/profiles
    method: POST
    headers:
      Content-Type: application/json
    body:
      userId: "\${store.firstUserId}"
      email: "\${store.firstUserEmail}"
    expect:
      status: 201
```

**auth-flow.yaml**

```yaml
# Authentication flow with token storage
global:
  execution: sequential  # Required for response storage
  variables:
    API_URL: https://api.example.com

requests:
  # Step 1: Login and store the token
  - name: Login
    url: \${API_URL}/auth/login
    method: POST
    headers:
      Content-Type: application/json
    body:
      username: "admin"
      password: "secret123"
    expect:
      status: 200
    store:
      authToken: body.accessToken
      refreshToken: body.refreshToken
      tokenExpiry: body.expiresIn

  # Step 2: Use token for authenticated request
  - name: Get Protected Resource
    url: \${API_URL}/protected/data
    method: GET
    headers:
      Authorization: Bearer \${store.authToken}
    expect:
      status: 200

  # Step 3: Create a resource with authentication
  - name: Create Resource
    url: \${API_URL}/protected/resources
    method: POST
    headers:
      Authorization: Bearer \${store.authToken}
      Content-Type: application/json
    body:
      name: "New Resource"
    expect:
      status: 201
```

**crud-workflow.yaml**

```yaml
# Complete CRUD workflow
requests:
  # CREATE: Make a new post
  - name: Create Post
    url: https://api.example.com/posts
    method: POST
    headers:
      Content-Type: application/json
    body:
      title: "My First Post"
      content: "Hello, World!"
      published: false
    expect:
      status: 201
    store:
      postId: body.id
      createdAt: body.createdAt

  # READ: Verify the post was created
  - name: Get Post
    url: https://api.example.com/posts/\${store.postId}
    method: GET
    expect:
      status: 200
      body:
        id: "\${store.postId}"

  # UPDATE: Modify the post
  - name: Update Post
    url: https://api.example.com/posts/\${store.postId}
    method: PUT
    headers:
      Content-Type: application/json
    body:
      title: "Updated Title"
      content: "Updated content"
      published: true
    expect:
      status: 200

  # DELETE: Remove the post
  - name: Delete Post
    url: https://api.example.com/posts/\${store.postId}
    method: DELETE
    expect:
      status: 204
```

**data-extraction.yaml**

```yaml
# Extracting data for chained requests
requests:
  # Get a list and extract specific items
  - name: List All Users
    url: https://api.example.com/users
    method: GET
    expect:
      status: 200
    store:
      firstUserId: body.users.0.id
      firstUserEmail: body.users.0.email
      totalCount: body.meta.total

  # Use extracted data for follow-up
  - name: Get First User Details
    url: https://api.example.com/users/\${store.firstUserId}
    method: GET
    expect:
      status: 200
      body:
        email: "\${store.firstUserEmail}"

  # Create related resource
  - name: Create User Profile
    url: https://api.example.com/profiles
    method: POST
    headers:
      Content-Type: application/json
    body:
      userId: "\${store.firstUserId}"
      email: "\${store.firstUserEmail}"
    expect:
      status: 201
```

## Extracting Nested Values

Store values from deeply nested response structures using dot-notation paths.

**nested-paths.yaml**

```yaml
# Extracting nested values
request:
  name: Get Complex Response
  url: https://api.example.com/data
  method: GET
  expect:
    status: 200
  store:
    # Top-level field
    requestId: body.id

    # Nested object field
    userName: body.user.name
    userEmail: body.user.email

    # Deeply nested field
    accessToken: body.auth.tokens.access

    # Array element by index
    firstItemId: body.items.0.id

    # Response headers
    contentType: headers.content-type
    requestId: headers.x-request-id

    # HTTP status code
    statusCode: status
```

## Combining with Other Variables

Response storage works seamlessly with static variables, dynamic variables, and date/time formatting.

**mixed-variables.yaml**

```yaml
# Combining store with other variables
global:
  variables:
    BASE_URL: https://api.example.com
    API_VERSION: v2

requests:
  - name: Create Session
    url: \${BASE_URL}/\${API_VERSION}/sessions
    method: POST
    headers:
      X-Request-ID: "\${UUID}"           # Dynamic variable
      X-Timestamp: "\${CURRENT_TIME}"    # Dynamic variable
    body:
      createdAt: "\${DATE:YYYY-MM-DD}"   # Date formatting
    store:
      sessionId: body.id

  - name: Use All Variable Types
    url: \${BASE_URL}/\${API_VERSION}/sessions/\${store.sessionId}
    method: GET
    headers:
      X-Request-ID: "\${UUID}"
      X-Session: "\${store.sessionId}"   # Stored value
    expect:
      status: 200
```

## Real-World Example

A complete e-commerce checkout flow demonstrating how response storage enables complex, realistic API test scenarios.

**checkout-flow.yaml**

```yaml
# Real-world: E-commerce checkout flow
global:
  execution: sequential
  variables:
    API_URL: https://api.shop.example.com

requests:
  # 1. Add item to cart
  - name: Add to Cart
    url: \${API_URL}/cart/items
    method: POST
    headers:
      Content-Type: application/json
    body:
      productId: "PROD-123"
      quantity: 2
    expect:
      status: 201
    store:
      cartId: body.cartId
      cartItemId: body.itemId

  # 2. Apply discount code
  - name: Apply Discount
    url: \${API_URL}/cart/\${store.cartId}/discounts
    method: POST
    headers:
      Content-Type: application/json
    body:
      code: "SAVE20"
    expect:
      status: 200
    store:
      discountAmount: body.discount.amount
      finalPrice: body.totals.final

  # 3. Create order from cart
  - name: Create Order
    url: \${API_URL}/orders
    method: POST
    headers:
      Content-Type: application/json
    body:
      cartId: "\${store.cartId}"
      shippingAddress:
        street: "123 Main St"
        city: "Example City"
        zip: "12345"
    expect:
      status: 201
    store:
      orderId: body.orderId
      orderTotal: body.total

  # 4. Process payment
  - name: Process Payment
    url: \${API_URL}/orders/\${store.orderId}/payment
    method: POST
    headers:
      Content-Type: application/json
    body:
      method: "credit_card"
      amount: "\${store.orderTotal}"
    expect:
      status: 200
      body:
        status: "completed"
```

## Important Notes

> Response storage only works in sequential execution mode (the default). In parallel mode, request order is not guaranteed, so stored values may not be available when expected.

## Best Practices

### Best Practices

• Use descriptive variable names
• Define common values as variables
• Use environment variables for secrets
• Group related variables logically
• Document complex expressions
