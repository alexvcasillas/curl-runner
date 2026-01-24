---
title: "Basic Examples"
description: "Simple HTTP request configurations to get you started with curl-runner. Copy and modify these examples for your own use cases."
category: "Documentation"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - basic
  - examples
  - yaml
  - authentication
  - retry
  - timeout
  - headers
  - response
  - request
slug: "/docs/basic"
toc: true
date: "2026-01-24T11:05:45.948Z"
lastModified: "2026-01-24T11:05:45.948Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Basic Examples"
  category: "Documentation"
tags:
  - documentation
  - documentation
og:
  title: "Basic Examples - curl-runner Documentation"
  description: "Simple HTTP request configurations to get you started with curl-runner. Copy and modify these examples for your own use cases."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Basic Examples"
  description: "Simple HTTP request configurations to get you started with curl-runner. Copy and modify these examples for your own use cases."
  datePublished: "2026-01-24T11:05:45.948Z"
  dateModified: "2026-01-24T11:05:45.948Z"
---

# Basic Examples

Simple HTTP request configurations to get you started with curl-runner. Copy and modify these examples for your own use cases.

{example.description}

### {example.title}

```yaml
{example.code}
```

## Running the Examples

Save any of the above examples to a YAML file and run them with `curl-runner`:

```bash
# Run a specific example
curl-runner get-request.yaml

# Run with verbose output
curl-runner post-request.yaml -v

# Run with timeout
curl-runner auth-request.yaml --timeout 10000

# Save results to file
curl-runner query-params.yaml --output results.json
```

## Tips

### Testing APIs

### Best Practices

### Debugging & Troubleshooting
