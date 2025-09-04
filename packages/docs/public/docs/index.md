---
title: "curl-runner Documentation"
description: "A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance."
category: "Getting Started"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - documentation
  - yaml
  - variables
  - templating
  - parallel
  - sequential
  - validation
  - response
  - request
  - cli
  - installation
  - bun
slug: "/docs"
toc: true
date: "2025-09-04T09:47:42.733Z"
lastModified: "2025-09-04T09:47:42.733Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "curl-runner Documentation"
  category: "Getting Started"
tags:
  - documentation
  - getting-started
og:
  title: "curl-runner Documentation - curl-runner Documentation"
  description: "A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "curl-runner Documentation"
  description: "A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance."
  datePublished: "2025-09-04T09:47:42.733Z"
  dateModified: "2025-09-04T09:47:42.733Z"
---

# curl-runner Documentation

A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance.

## Features

### YAML Configuration

Define HTTP requests using simple, readable YAML files with support for variables and templates.

### Fast Execution

Built with Bun for blazing-fast performance. Execute requests sequentially or in parallel.

### Powerful CLI

Comprehensive command-line interface with beautiful output, progress indicators, and error handling.

### Flexible Configuration

Global settings, variable interpolation, response validation, and customizable output formats.



## Quick Start

Get up and running with `curl-runner` in just a few minutes.

### 1. Create a YAML file

**simple.yaml**

```yaml
# Basic API test configuration
collection:
  name: "My API Tests"
  requests:
    - name: "Get users"
      url: "https://jsonplaceholder.typicode.com/users"
      method: GET
      expect:
        status: 200
        
    - name: "Create user"
      url: "https://jsonplaceholder.typicode.com/users"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "name": "John Doe",
          "email": "john@example.com"
        }
      expect:
        status: 201
```

**simple.yaml**

```yaml
# Basic API test configuration
collection:
  name: "My API Tests"
  requests:
    - name: "Get users"
      url: "https://jsonplaceholder.typicode.com/users"
      method: GET
      expect:
        status: 200
        
    - name: "Create user"
      url: "https://jsonplaceholder.typicode.com/users"
      method: POST
      headers:
        Content-Type: "application/json"
      body: |
        {
          "name": "John Doe",
          "email": "john@example.com"
        }
      expect:
        status: 201
```

```bash
# Install curl-runner globally
npm install -g curl-runner

# Or using yarn
yarn global add curl-runner

# Or using pnpm
pnpm install -g curl-runner
```

## Next Steps

### Installation Guide

Detailed installation instructions for all platforms

[Get Started](/docs/installation)

### YAML Structure

Learn the YAML configuration format and options

[Learn More](/docs/yaml-structure)

### Examples

Browse real-world examples and use cases

[Explore](/docs/examples/basic)
