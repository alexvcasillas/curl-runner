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
  - timeout
  - headers
  - response
  - request
  - cli
  - installation
  - bun
  - npm
  - environment
slug: "/docs"
toc: true
date: "2025-09-19T16:15:05.333Z"
lastModified: "2025-09-19T16:15:05.333Z"
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
  datePublished: "2025-09-19T16:15:05.333Z"
  dateModified: "2025-09-19T16:15:05.333Z"
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



## What is curl-runner?

`curl-runner` is a modern CLI tool that transforms how you manage and execute HTTP requests. Instead of writing complex curl commands or maintaining scattered shell scripts, you define your API requests in clean, readable YAML files with powerful features like parallel execution, response validation, and variable templating.

Built with Bun for exceptional performance, curl-runner is perfect for API testing, automation workflows, CI/CD pipelines, and development environments where speed and reliability matter.

## Quick Start

Get up and running with `curl-runner` in just a few minutes.

### Prerequisites

**For npm/yarn/pnpm installation:** Bun runtime must be installed on your system.

### 1. Install curl-runner

```bash
# Install curl-runner globally
npm install -g @curl-runner/cli

# Or using yarn
yarn global add @curl-runner/cli

# Or using pnpm
pnpm install -g @curl-runner/cli

# Or using bun
bun install -g @curl-runner/cli
```

### 2. Create a YAML file

**api-test.yaml**

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

### 3. Run your requests

```bash
curl-runner api-test.yaml
```

```bash
# Install curl-runner globally
npm install -g @curl-runner/cli

# Or using yarn
yarn global add @curl-runner/cli

# Or using pnpm
pnpm install -g @curl-runner/cli

# Or using bun
bun install -g @curl-runner/cli
```

**api-test.yaml**

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
curl-runner api-test.yaml
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

## Key Concepts

> Create dynamic requests with variables, environment values, and computed expressions
