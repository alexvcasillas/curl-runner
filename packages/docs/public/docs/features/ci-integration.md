---
title: "CI/CD Integration"
description: "Integrate curl-runner with CI/CD pipelines using CI-friendly exit codes and configurable failure thresholds."
category: "Documentation"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - ci/cd
  - integration
  - variables
  - validation
  - retry
  - timeout
  - request
  - cli
  - installation
  - bun
  - npm
  - environment
slug: "/docs/ci-integration"
toc: true
date: "2026-01-23T22:45:48.034Z"
lastModified: "2026-01-23T22:45:48.034Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "CI/CD Integration"
  category: "Documentation"
tags:
  - documentation
  - documentation
og:
  title: "CI/CD Integration - curl-runner Documentation"
  description: "Integrate curl-runner with CI/CD pipelines using CI-friendly exit codes and configurable failure thresholds."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "CI/CD Integration"
  description: "Integrate curl-runner with CI/CD pipelines using CI-friendly exit codes and configurable failure thresholds."
  datePublished: "2026-01-23T22:45:48.034Z"
  dateModified: "2026-01-23T22:45:48.034Z"
---

# CI/CD Integration

Integrate curl-runner with CI/CD pipelines using CI-friendly exit codes and configurable failure thresholds.

## Overview

`curl-runner` provides CI-friendly exit codes that integrate seamlessly with popular CI/CD platforms. By default, the tool follows standard conventions where exit code 0 indicates success and exit code 1 indicates failure.

> **Strict Mode**
> Exit 1 if any validation fails

## Exit Code Options

Control how curl-runner determines its exit code based on test results.

### Option Reference

| Option | Description |
| --- | --- |
| `--strict-exit` | Exit with code 1 if any validation fails, regardless of --continue-on-error |
| `--fail-on &lt;count&gt;` | Exit with code 1 only if failures exceed the specified count |
| `--fail-on-percentage &lt;pct&gt;` | Exit with code 1 only if failure percentage exceeds the threshold (0-100) |

**terminal**

```bash
# CI/CD exit code options

# Strict mode - exit 1 if ANY validation fails
curl-runner tests/ --strict-exit

# Run all tests, then fail if any failed
curl-runner tests/ --continue-on-error --strict-exit

# Allow up to 2 failures before failing
curl-runner tests/ --fail-on 2

# Allow up to 10% of tests to fail
curl-runner tests/ --fail-on-percentage 10

# Combine options for comprehensive CI setup
curl-runner tests/ \\
  --continue-on-error \\
  --strict-exit \\
  --output-format json \\
  --output results.json
```

## Exit Code Behavior

Understanding how different configurations affect the exit code.

## Environment Variables

All CI options can be configured via environment variables for easy integration with CI/CD platforms.

### Environment Variable Reference

| Variable | Type | Description |
| --- | --- | --- |
| `CURL_RUNNER_STRICT_EXIT` | boolean | Enable strict exit mode (true/false) |
| `CURL_RUNNER_FAIL_ON` | number | Maximum allowed failures |
| `CURL_RUNNER_FAIL_ON_PERCENTAGE` | number | Maximum allowed failure percentage (0-100) |

**terminal**

```bash
# Environment variables for CI/CD

# Enable strict exit mode
export CURL_RUNNER_STRICT_EXIT=true

# Set failure threshold (count)
export CURL_RUNNER_FAIL_ON=5

# Set failure threshold (percentage)
export CURL_RUNNER_FAIL_ON_PERCENTAGE=10

# Combine with other settings
export CURL_RUNNER_STRICT_EXIT=true
export CURL_RUNNER_CONTINUE_ON_ERROR=true
export CURL_RUNNER_OUTPUT_FORMAT=json
curl-runner tests/
```

## Platform Examples

### GitHub Actions

Complete workflow for running API tests with artifact upload.

**.github/workflows/api-tests.yml**

```yaml
name: API Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v2

      - name: Install curl-runner
        run: bun install -g curl-runner

      - name: Run API Tests
        env:
          CURL_RUNNER_STRICT_EXIT: true
          CURL_RUNNER_CONTINUE_ON_ERROR: true
        run: curl-runner tests/ --output-format json --output results.json

      - name: Upload Results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: test-results
          path: results.json
```

### GitLab CI

GitLab CI configuration with test artifacts.

**.gitlab-ci.yml**

```yaml
stages:
  - test

api-tests:
  stage: test
  image: oven/bun:latest
  variables:
    CURL_RUNNER_STRICT_EXIT: "true"
    CURL_RUNNER_CONTINUE_ON_ERROR: "true"
    CURL_RUNNER_OUTPUT_FORMAT: json
  before_script:
    - bun install -g curl-runner
  script:
    - curl-runner tests/ --output test-results.json
  artifacts:
    when: always
    paths:
      - test-results.json
```

### Jenkins Pipeline

Declarative Jenkins pipeline with artifact archiving.

**Jenkinsfile**

```groovy
pipeline {
    agent any

    environment {
        CURL_RUNNER_STRICT_EXIT = 'true'
        CURL_RUNNER_CONTINUE_ON_ERROR = 'true'
        CURL_RUNNER_OUTPUT_FORMAT = 'json'
    }

    stages {
        stage('Install') {
            steps {
                sh 'npm install -g curl-runner'
            }
        }

        stage('Test') {
            steps {
                sh 'curl-runner tests/ --output test-results.json'
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

CircleCI configuration with test artifact storage.

**.circleci/config.yml**

```yaml
version: 2.1

executors:
  node:
    docker:
      - image: cimg/node:20.0

jobs:
  test:
    executor: node
    environment:
      CURL_RUNNER_STRICT_EXIT: "true"
      CURL_RUNNER_CONTINUE_ON_ERROR: "true"
    steps:
      - checkout
      - run:
          name: Install curl-runner
          command: npm install -g curl-runner
      - run:
          name: Run API Tests
          command: curl-runner tests/ --output-format json --output test-results.json
      - store_artifacts:
          path: test-results.json
          destination: test-results

workflows:
  test:
    jobs:
      - test
```

**.github/workflows/api-tests.yml**

```yaml
name: API Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v2

      - name: Install curl-runner
        run: bun install -g curl-runner

      - name: Run API Tests
        env:
          CURL_RUNNER_STRICT_EXIT: true
          CURL_RUNNER_CONTINUE_ON_ERROR: true
        run: curl-runner tests/ --output-format json --output results.json

      - name: Upload Results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: test-results
          path: results.json
```

**.gitlab-ci.yml**

```yaml
stages:
  - test

api-tests:
  stage: test
  image: oven/bun:latest
  variables:
    CURL_RUNNER_STRICT_EXIT: "true"
    CURL_RUNNER_CONTINUE_ON_ERROR: "true"
    CURL_RUNNER_OUTPUT_FORMAT: json
  before_script:
    - bun install -g curl-runner
  script:
    - curl-runner tests/ --output test-results.json
  artifacts:
    when: always
    paths:
      - test-results.json
```

**Jenkinsfile**

```groovy
pipeline {
    agent any

    environment {
        CURL_RUNNER_STRICT_EXIT = 'true'
        CURL_RUNNER_CONTINUE_ON_ERROR = 'true'
        CURL_RUNNER_OUTPUT_FORMAT = 'json'
    }

    stages {
        stage('Install') {
            steps {
                sh 'npm install -g curl-runner'
            }
        }

        stage('Test') {
            steps {
                sh 'curl-runner tests/ --output test-results.json'
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

**.circleci/config.yml**

```yaml
version: 2.1

executors:
  node:
    docker:
      - image: cimg/node:20.0

jobs:
  test:
    executor: node
    environment:
      CURL_RUNNER_STRICT_EXIT: "true"
      CURL_RUNNER_CONTINUE_ON_ERROR: "true"
    steps:
      - checkout
      - run:
          name: Install curl-runner
          command: npm install -g curl-runner
      - run:
          name: Run API Tests
          command: curl-runner tests/ --output-format json --output test-results.json
      - store_artifacts:
          path: test-results.json
          destination: test-results

workflows:
  test:
    jobs:
      - test
```

## Best Practices

### Best Practices

• Use descriptive variable names
• Define common values as variables
• Use environment variables for secrets
• Group related variables logically
• Document complex expressions

## Troubleshooting
