---
title: "Getting Started: Your First API Test"
description: "Learn the basics of curl-runner by creating your first API test. This tutorial covers creating YAML files, running requests, and validating responses."
category: "Tutorials"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - tutorial
  - getting-started
  - yaml
  - validation
  - request
slug: "/docs/tutorials/getting-started"
toc: true
date: "2026-01-24T00:00:00.000Z"
lastModified: "2026-01-24T00:00:00.000Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Getting Started"
  category: "Tutorials"
tags:
  - tutorials
  - getting-started
og:
  title: "Getting Started: Your First API Test - curl-runner Documentation"
  description: "Learn the basics of curl-runner by creating your first API test."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Getting Started: Your First API Test"
  description: "Learn the basics of curl-runner by creating your first API test."
  datePublished: "2026-01-24T00:00:00.000Z"
  dateModified: "2026-01-24T00:00:00.000Z"
---

# Getting Started: Your First API Test

In this tutorial, you'll learn the fundamentals of curl-runner by creating and running your first API test.

## Prerequisites

Before starting, make sure you have curl-runner installed:

```bash
npm install -g curl-runner
# or
bun install -g curl-runner
```

Verify the installation:

```bash
curl-runner --version
```

## Step 1: Create Your First Request

Create a file named `my-first-test.yaml`:

```yaml
request:
  name: Get a Post
  url: https://jsonplaceholder.typicode.com/posts/1
  method: GET
```

This simple configuration defines a GET request to a public test API.

## Step 2: Run the Request

Execute your test:

```bash
curl-runner my-first-test.yaml
```

You should see output showing the request was successful with a 200 status code.

## Step 3: Add Response Validation

Let's verify the response contains expected data. Update your file:

```yaml
request:
  name: Get a Post
  url: https://jsonplaceholder.typicode.com/posts/1
  method: GET
  expect:
    status: 200
    body:
      id: 1
      userId: 1
```

Run again:

```bash
curl-runner my-first-test.yaml
```

curl-runner now validates that the response has status 200 and the body contains the expected `id` and `userId` fields.

## Step 4: Add Headers Validation

You can also validate response headers:

```yaml
request:
  name: Get a Post
  url: https://jsonplaceholder.typicode.com/posts/1
  method: GET
  expect:
    status: 200
    headers:
      content-type: application/json; charset=utf-8
    body:
      id: 1
```

## Step 5: Create a POST Request

Now let's create a request that sends data:

```yaml
request:
  name: Create a Post
  url: https://jsonplaceholder.typicode.com/posts
  method: POST
  headers:
    Content-Type: application/json
  body:
    title: My New Post
    body: This is the content of my post
    userId: 1
  expect:
    status: 201
    body:
      title: My New Post
```

## Step 6: Use Variables

Variables make your tests more flexible. Create a file with global variables:

```yaml
global:
  variables:
    BASE_URL: https://jsonplaceholder.typicode.com
    USER_ID: 1

request:
  name: Get User Posts
  url: ${BASE_URL}/users/${USER_ID}/posts
  method: GET
  expect:
    status: 200
```

## Step 7: Run Multiple Requests

Create a file with multiple requests:

```yaml
global:
  variables:
    BASE_URL: https://jsonplaceholder.typicode.com

requests:
  - name: Get First Post
    url: ${BASE_URL}/posts/1
    method: GET
    expect:
      status: 200

  - name: Get First User
    url: ${BASE_URL}/users/1
    method: GET
    expect:
      status: 200

  - name: Get First Comment
    url: ${BASE_URL}/comments/1
    method: GET
    expect:
      status: 200
```

Run all requests:

```bash
curl-runner my-first-test.yaml
```

## Step 8: Enable Verbose Output

See detailed information about your requests:

```bash
curl-runner my-first-test.yaml -v
```

This shows headers, response bodies, and timing information.

## What You Learned

In this tutorial, you learned how to:

- Create YAML request files
- Run requests with curl-runner
- Validate status codes, headers, and response bodies
- Use variables for dynamic values
- Execute multiple requests in a single file
- Enable verbose output for debugging

## Next Steps

- [Authentication Workflows](/docs/tutorials/authentication-workflows) - Learn to chain requests with response storage
- [Advanced Validation](/docs/tutorials/advanced-validation) - Master regex, ranges, and wildcards
- [YAML Structure Reference](/docs/yaml-structure) - Explore all configuration options
