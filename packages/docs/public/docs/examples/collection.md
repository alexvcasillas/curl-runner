---
title: "Collection Example"
description: "Learn how to organize multiple requests into collections for comprehensive API testing."
---

# Collection Example

Learn how to organize multiple requests into collections for comprehensive API testing.

## Table of Contents

- [Overview](#overview)
- [Basic Collection](#basic-collection)
- [Collection Variables](#collection-variables)
- [Complete Collection Example](#complete-collection-example)
- [Collection Structure](#collection-structure)
- [Multiple Collections](#multiple-collections)
- [Running Collections](#running-collections)
- [Best Practices](#best-practices)
- [Common Use Cases](#common-use-cases)

## Overview

> **Group Related Tests**
>


> **Share Configuration**
>


## Basic Collection

```yaml title="basic-collection.yaml"
# Basic collection structure
collection:
  name: User Management API Tests
  description: Testing user CRUD operations
  
  requests:
    - name: List All Users
      url: https://api.example.com/users
      method: GET
      
    - name: Get Specific User
      url: https://api.example.com/users/1
      method: GET
      
    - name: Create New User
      url: https://api.example.com/users
      method: POST
      headers:
        Content-Type: application/json
      body:
        name: "John Doe"
        email: "john@example.com"
```

## Collection Variables

```yaml title="collection-variables.yaml"
# Collection with shared variables
collection:
  name: E-Commerce API Tests
  variables:
    USER_ID: 12345
    PRODUCT_ID: 98765
    ORDER_ID: 555
    
  defaults:
    headers:
      Authorization: Bearer \${API_TOKEN}
      Content-Type: application/json
  
  requests:
    - name: Get User Profile
      url: \${BASE_URL}/users/\${USER_ID}
      method: GET
      
    - name: Get User Orders
      url: \${BASE_URL}/users/\${USER_ID}/orders
      method: GET
      
    - name: Get Product Details
      url: \${BASE_URL}/products/\${PRODUCT_ID}
      method: GET
      
    - name: Add Product to Cart
      url: \${BASE_URL}/users/\${USER_ID}/cart
      method: POST
      body:
        productId: \${PRODUCT_ID}
        quantity: 2
```

> **Variable Precedence:**
>


## Complete Collection Example

```yaml title="complete-collection.yaml"
# Complete collection with global settings
global:
  execution: sequential
  continueOnError: true
  variables:
    BASE_URL: https://jsonplaceholder.typicode.com
    API_VERSION: v1
  output:
    verbose: true
    showMetrics: true
    saveToFile: test-results.json

collection:
  name: JSONPlaceholder Complete Test Suite
  description: Comprehensive API testing example
  
  variables:
    TEST_USER_ID: 1
    TEST_POST_ID: 1
  
  defaults:
    timeout: 5000
    retry:
      count: 2
      delay: 1000
    headers:
      Accept: application/json
      X-API-Version: \${API_VERSION}
  
  requests:
    # User Operations
    - name: Get All Users
      url: \${BASE_URL}/users
      method: GET
      expect:
        status: 200
        
    - name: Get Test User
      url: \${BASE_URL}/users/\${TEST_USER_ID}
      method: GET
      expect:
        status: 200
        body:
          id: 1
          
    # Post Operations
    - name: Get User Posts
      url: \${BASE_URL}/users/\${TEST_USER_ID}/posts
      method: GET
      expect:
        status: 200
        
    - name: Create New Post
      url: \${BASE_URL}/posts
      method: POST
      headers:
        Content-Type: application/json
      body:
        title: "Test Post"
        body: "This is a test post created by curl-runner"
        userId: \${TEST_USER_ID}
      expect:
        status: 201
        body:
          userId: 1
          
    - name: Update Post
      url: \${BASE_URL}/posts/\${TEST_POST_ID}
      method: PUT
      body:
        id: \${TEST_POST_ID}
        title: "Updated Post"
        body: "This post has been updated"
        userId: \${TEST_USER_ID}
      expect:
        status: 200
        
    - name: Delete Post
      url: \${BASE_URL}/posts/\${TEST_POST_ID}
      method: DELETE
      expect:
        status: 200
        
    # Comment Operations
    - name: Get Post Comments
      url: \${BASE_URL}/posts/\${TEST_POST_ID}/comments
      method: GET
      expect:
        status: 200
        
    # Album and Photo Operations
    - name: Get User Albums
      url: \${BASE_URL}/users/\${TEST_USER_ID}/albums
      method: GET
      expect:
        status: 200
        
    # Todo Operations
    - name: Get User Todos
      url: \${BASE_URL}/users/\${TEST_USER_ID}/todos
      method: GET
      expect:
        status: 200
```

## Collection Structure

| Field | Type | Description |
| --- | --- | --- |
| name | string | Collection name (required) |
| description | string | Collection description (optional) |
| variables | object | Collection-level variables |
| defaults | object | Default request configuration |
| requests | array | List of requests (required) |


## Multiple Collections

```yaml title="multiple-collections.yaml"
# Multiple collections in one file
global:
  variables:
    API_URL: https://api.example.com
    AUTH_TOKEN: secret-token

# First Collection - Authentication Tests
collection:
  name: Authentication Suite
  requests:
    - name: Login
      url: \${API_URL}/auth/login
      method: POST
      body:
        username: "testuser"
        password: "testpass"
      expect:
        status: 200
        body:
          token: "*"
          
    - name: Refresh Token
      url: \${API_URL}/auth/refresh
      method: POST
      headers:
        Authorization: Bearer \${AUTH_TOKEN}
      expect:
        status: 200

---
# Second Collection - User Management
collection:
  name: User Management Suite
  defaults:
    headers:
      Authorization: Bearer \${AUTH_TOKEN}
      
  requests:
    - name: Get Profile
      url: \${API_URL}/profile
      method: GET
      expect:
        status: 200
        
    - name: Update Profile
      url: \${API_URL}/profile
      method: PATCH
      body:
        displayName: "Updated Name"
      expect:
        status: 200
```

## Running Collections

```bash title="terminal"
# Run collection tests
curl-runner collection.yaml

# Run with specific output format
curl-runner collection.yaml --output-format pretty

# Run in parallel mode
curl-runner collection.yaml --parallel

# Save results to file
curl-runner collection.yaml --output results.json

# Run with verbose output
curl-runner collection.yaml --verbose
```

## Best Practices

## Common Use Cases

> **API Test Suites**
>


> **CI/CD Pipelines**
>


> **Health Checks**
>


> **API Documentation**
>


