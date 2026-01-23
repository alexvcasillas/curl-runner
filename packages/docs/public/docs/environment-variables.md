---
title: "Environment Variables"
description: "Configure curl-runner behavior using environment variables for consistent settings across different environments."
category: "Documentation"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - environment
  - variables
  - yaml
  - parallel
  - sequential
  - retry
  - timeout
  - response
  - request
  - cli
  - installation
  - docker
  - npm
slug: "/docs/environment-variables"
toc: true
date: "2026-01-23T21:27:49.020Z"
lastModified: "2026-01-23T21:27:49.020Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Environment Variables"
  category: "Documentation"
tags:
  - documentation
  - documentation
og:
  title: "Environment Variables - curl-runner Documentation"
  description: "Configure curl-runner behavior using environment variables for consistent settings across different environments."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Environment Variables"
  description: "Configure curl-runner behavior using environment variables for consistent settings across different environments."
  datePublished: "2026-01-23T21:27:49.020Z"
  dateModified: "2026-01-23T21:27:49.020Z"
---

# Environment Variables

Configure curl-runner behavior using environment variables for consistent settings across different environments.

## Overview

Environment variables provide a way to configure `curl-runner` without modifying YAML files or using command-line options. This is particularly useful for CI/CD pipelines, containerized environments, and maintaining consistent settings across teams.

## Basic Usage

Set environment variables in your shell or include them inline when running commands.

**Environment Variable Examples**

```bash
# Set verbose output
export CURL_RUNNER_VERBOSE=true
curl-runner tests/

# Set parallel execution
export CURL_RUNNER_EXECUTION=parallel
curl-runner api-tests.yaml

# Set multiple variables
export CURL_RUNNER_VERBOSE=true
export CURL_RUNNER_EXECUTION=parallel
export CURL_RUNNER_CONTINUE_ON_ERROR=true
curl-runner tests/

# Inline environment variables (one-time use)
CURL_RUNNER_VERBOSE=true curl-runner tests/
```

## Available Variables

All curl-runner environment variables follow the `CURL_RUNNER_*` naming convention.

### General Variables

| Variable | Type | Description |
|----------|------|-------------|
| `CURL_RUNNER_VERBOSE` | boolean | Enable verbose output (`true`/`false`) |
| `CURL_RUNNER_EXECUTION` | string | Execution mode (`sequential`/`parallel`) |
| `CURL_RUNNER_CONTINUE_ON_ERROR` | boolean | Continue on errors (`true`/`false`) |

### Request Variables

| Variable | Type | Description |
|----------|------|-------------|
| `CURL_RUNNER_TIMEOUT` | number | Request timeout in milliseconds |
| `CURL_RUNNER_RETRIES` | number | Maximum retry attempts |
| `CURL_RUNNER_RETRY_DELAY` | number | Delay between retries in milliseconds |

### Output Variables

| Variable | Type | Description |
|----------|------|-------------|
| `CURL_RUNNER_OUTPUT_FORMAT` | string | Output format (`json`/`pretty`/`raw`) |
| `CURL_RUNNER_PRETTY_LEVEL` | string | Pretty format level (`minimal`/`standard`/`detailed`) |
| `CURL_RUNNER_OUTPUT_FILE` | string | Path to save results |

### CI/CD Variables

| Variable | Type | Description |
|----------|------|-------------|
| `CURL_RUNNER_STRICT_EXIT` | boolean | Exit with code 1 if any validation fails (`true`/`false`) |
| `CURL_RUNNER_FAIL_ON` | number | Maximum allowed failures before exit code 1 |
| `CURL_RUNNER_FAIL_ON_PERCENTAGE` | number | Maximum allowed failure percentage (0-100) |

## Output Configuration

Control how results are displayed and saved using output-related environment variables.

**Output Configuration**

```bash
# Set output format
export CURL_RUNNER_OUTPUT_FORMAT=json
curl-runner tests/

# Set pretty level for detailed output
export CURL_RUNNER_OUTPUT_FORMAT=pretty
export CURL_RUNNER_PRETTY_LEVEL=detailed
curl-runner tests/

# Save output to file
export CURL_RUNNER_OUTPUT_FILE=results.json
curl-runner tests/

# Enable verbose logging
export CURL_RUNNER_VERBOSE=true
curl-runner tests/
```

## Request Configuration

Configure default request behavior including timeouts and retry logic.

**Request Configuration**

```bash
# Set global timeout (milliseconds)
export CURL_RUNNER_TIMEOUT=10000
curl-runner tests/

# Configure retry behavior
export CURL_RUNNER_RETRIES=3
export CURL_RUNNER_RETRY_DELAY=2000
curl-runner tests/

# Disable retries
export CURL_RUNNER_RETRIES=0
curl-runner tests/
```

## Execution Configuration

Control how requests are executed and how errors are handled.

**Execution Configuration**

```bash
# Set execution mode
export CURL_RUNNER_EXECUTION=parallel
curl-runner tests/

# Continue on errors
export CURL_RUNNER_CONTINUE_ON_ERROR=true
curl-runner tests/

# Combine execution settings
export CURL_RUNNER_EXECUTION=parallel
export CURL_RUNNER_CONTINUE_ON_ERROR=true
curl-runner tests/
```

## Docker & Containers

Environment variables are ideal for containerized deployments where configuration should be injected at runtime.

**Docker Example**

```dockerfile
# Dockerfile example
FROM node:20-alpine
ENV CURL_RUNNER_VERBOSE=false
ENV CURL_RUNNER_OUTPUT_FORMAT=json
ENV CURL_RUNNER_EXECUTION=sequential
WORKDIR /app
RUN npm install -g curl-runner
CMD ["curl-runner", "/tests"]
```

## CI/CD Integration

Examples of using environment variables in popular CI/CD platforms.

### GitHub Actions

```yaml
# Basic CI workflow with strict exit
- name: Run API Tests
  env:
    CURL_RUNNER_STRICT_EXIT: true
    CURL_RUNNER_CONTINUE_ON_ERROR: true
    CURL_RUNNER_OUTPUT_FORMAT: json
    CURL_RUNNER_OUTPUT_FILE: test-results.json
  run: curl-runner tests/

# Allow some failures in large test suite
- name: Run Smoke Tests
  env:
    CURL_RUNNER_FAIL_ON_PERCENTAGE: 10
    CURL_RUNNER_CONTINUE_ON_ERROR: true
  run: curl-runner smoke-tests/
```

### GitLab CI

```yaml
test:
  variables:
    CURL_RUNNER_STRICT_EXIT: "true"
    CURL_RUNNER_EXECUTION: parallel
    CURL_RUNNER_TIMEOUT: "30000"
    CURL_RUNNER_OUTPUT_FORMAT: json
  script:
    - curl-runner api-tests.yaml
  artifacts:
    when: always
    paths:
      - test-results.json
```

### Jenkins Pipeline

```groovy
pipeline {
  environment {
    CURL_RUNNER_STRICT_EXIT = 'true'
    CURL_RUNNER_CONTINUE_ON_ERROR = 'true'
    CURL_RUNNER_OUTPUT_FILE = 'test-results.json'
  }
  stages {
    stage('Test') {
      steps {
        sh 'curl-runner tests/'
      }
    }
  }
  post {
    always {
      archiveArtifacts artifacts: 'test-results.json', allowEmptyArchive: true
    }
  }
}
```

### CircleCI

```yaml
version: 2.1
jobs:
  test:
    docker:
      - image: node:20
    environment:
      CURL_RUNNER_STRICT_EXIT: "true"
      CURL_RUNNER_CONTINUE_ON_ERROR: "true"
    steps:
      - checkout
      - run: npm install -g curl-runner
      - run: curl-runner tests/ --output-format json --output test-results.json
      - store_artifacts:
          path: test-results.json
```

## Best Practices

### Best Practices

• Use descriptive variable names
• Define common values as variables
• Use environment variables for secrets
• Group related variables logically
• Document complex expressions
