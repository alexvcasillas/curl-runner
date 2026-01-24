---
title: "Quick Start"
description: "Get up and running with curl-runner in just a few minutes. Follow these simple steps to make your first HTTP request."
category: "Getting Started"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - quick
  - start
  - yaml
  - variables
  - parallel
  - sequential
  - validation
  - response
  - request
  - collection
  - cli
  - installation
slug: "/docs/quick-start"
toc: true
date: "2026-01-24T11:05:45.876Z"
lastModified: "2026-01-24T11:05:45.876Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Quick Start"
  category: "Getting Started"
tags:
  - documentation
  - getting-started
og:
  title: "Quick Start - curl-runner Documentation"
  description: "Get up and running with curl-runner in just a few minutes. Follow these simple steps to make your first HTTP request."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Quick Start"
  description: "Get up and running with curl-runner in just a few minutes. Follow these simple steps to make your first HTTP request."
  datePublished: "2026-01-24T11:05:45.876Z"
  dateModified: "2026-01-24T11:05:45.876Z"
---

# Quick Start

Get up and running with curl-runner in just a few minutes. Follow these simple steps to make your first HTTP request.

### {step.title}

{step.description}

```text
{step.code}
```

## More Examples

Try these more advanced examples to explore `curl-runner`'s capabilities.

Execute multiple requests in sequence with shared variables.

Run multiple requests simultaneously for faster execution.

### Request Collection

### Parallel Execution

**collection.yaml**

```yaml
# collection.yaml
global:
  variables:
    BASE_URL: https://jsonplaceholder.typicode.com
  execution: sequential

requests:
  - name: Get All Posts
    url: \${BASE_URL}/posts
    method: GET
    
  - name: Get Specific Post
    url: \${BASE_URL}/posts/1
    method: GET
    
  - name: Get Post Comments
    url: \${BASE_URL}/posts/1/comments
    method: GET
```

```bash
curl-runner collection.yaml
```

**parallel.yaml**

```yaml
# parallel.yaml
global:
  execution: parallel
  continueOnError: true

requests:
  - name: Check API Health
    url: https://jsonplaceholder.typicode.com/posts/1
    method: GET
    
  - name: Check Users Endpoint
    url: https://jsonplaceholder.typicode.com/users/1
    method: GET
    
  - name: Check Albums Endpoint
    url: https://jsonplaceholder.typicode.com/albums/1
    method: GET
```

```bash
curl-runner parallel.yaml -v
```

## Tips & Tricks

## What's Next?

## Continue Learning
