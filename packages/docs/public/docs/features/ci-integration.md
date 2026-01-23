---
title: "CI/CD Integration"
description: "Integrate curl-runner with CI/CD pipelines for automated API testing with configurable exit codes and failure thresholds."
category: "Features"
keywords:
  - curl-runner
  - ci
  - cd
  - github-actions
  - gitlab-ci
  - jenkins
  - circleci
  - exit-codes
  - api-testing
  - automation
slug: "/docs/features/ci-integration"
toc: true
date: "2026-01-23T00:00:00.000Z"
lastModified: "2026-01-23T00:00:00.000Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "CI/CD Integration"
  category: "Features"
tags:
  - documentation
  - features
og:
  title: "CI/CD Integration - curl-runner Documentation"
  description: "Integrate curl-runner with CI/CD pipelines for automated API testing with configurable exit codes and failure thresholds."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "CI/CD Integration"
  description: "Integrate curl-runner with CI/CD pipelines for automated API testing with configurable exit codes and failure thresholds."
  datePublished: "2026-01-23T00:00:00.000Z"
  dateModified: "2026-01-23T00:00:00.000Z"
---

# CI/CD Integration

Integrate curl-runner with CI/CD pipelines for automated API testing with configurable exit codes and failure thresholds.

## Overview

curl-runner provides CI-friendly exit codes that integrate seamlessly with popular CI/CD platforms. By default, the tool follows standard conventions where exit code 0 indicates success and exit code 1 indicates failure.

## Exit Code Options

### --strict-exit

Exit with code 1 if ANY validation fails, regardless of the `--continue-on-error` setting.

```bash
# Fail the pipeline if any test fails
curl-runner tests/ --strict-exit

# Run all tests, then fail if any failed
curl-runner tests/ --continue-on-error --strict-exit
```

**Use case**: When you want to ensure all API validations pass before deploying.

### --fail-on \<count\>

Exit with code 1 only if the number of failures exceeds the specified threshold.

```bash
# Allow up to 2 failures
curl-runner tests/ --fail-on 2

# Combined with continue-on-error to run all tests
curl-runner tests/ --continue-on-error --fail-on 2
```

**Use case**: When you have known flaky tests or want to allow some tolerance.

### --fail-on-percentage \<percent\>

Exit with code 1 only if the failure percentage exceeds the specified threshold.

```bash
# Allow up to 10% of tests to fail
curl-runner tests/ --fail-on-percentage 10

# Allow up to 5% failures in a large test suite
curl-runner tests/ --continue-on-error --fail-on-percentage 5
```

**Use case**: When you have large test suites and want proportional failure tolerance.

## Exit Code Behavior Matrix

| Configuration | No Failures | Failures Exist |
|--------------|-------------|----------------|
| Default (no options) | Exit 0 | Exit 1 |
| `--continue-on-error` | Exit 0 | Exit 0 |
| `--strict-exit` | Exit 0 | Exit 1 |
| `--continue-on-error --strict-exit` | Exit 0 | Exit 1 |
| `--fail-on N` | Exit 0 | Exit 0 if ≤N, Exit 1 if >N |
| `--fail-on-percentage P` | Exit 0 | Exit 0 if ≤P%, Exit 1 if >P% |

## Environment Variables

All CI options can be configured via environment variables:

| Option | Environment Variable |
|--------|---------------------|
| `--strict-exit` | `CURL_RUNNER_STRICT_EXIT=true` |
| `--fail-on N` | `CURL_RUNNER_FAIL_ON=N` |
| `--fail-on-percentage P` | `CURL_RUNNER_FAIL_ON_PERCENTAGE=P` |

## Platform Examples

### GitHub Actions

**Basic strict mode:**

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

**With failure threshold:**

```yaml
- name: Run Smoke Tests
  env:
    CURL_RUNNER_FAIL_ON_PERCENTAGE: 10
    CURL_RUNNER_CONTINUE_ON_ERROR: true
  run: curl-runner smoke-tests/
```

### GitLab CI

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
    reports:
      junit: test-results.json
```

### Jenkins Pipeline

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

### Azure Pipelines

```yaml
trigger:
  - main

pool:
  vmImage: 'ubuntu-latest'

variables:
  CURL_RUNNER_STRICT_EXIT: 'true'
  CURL_RUNNER_CONTINUE_ON_ERROR: 'true'

steps:
  - task: NodeTool@0
    inputs:
      versionSpec: '20.x'
    displayName: 'Install Node.js'

  - script: npm install -g curl-runner
    displayName: 'Install curl-runner'

  - script: curl-runner tests/ --output-format json --output $(Build.ArtifactStagingDirectory)/test-results.json
    displayName: 'Run API Tests'

  - task: PublishBuildArtifacts@1
    condition: always()
    inputs:
      pathToPublish: '$(Build.ArtifactStagingDirectory)'
      artifactName: 'test-results'
```

## Best Practices

### 1. Always Use --continue-on-error with CI Options

When using `--strict-exit`, `--fail-on`, or `--fail-on-percentage`, combine with `--continue-on-error` to ensure all tests run before the final exit decision.

```bash
# Good: Run all tests, then determine exit code
curl-runner tests/ --continue-on-error --strict-exit

# Less ideal: Stops on first failure
curl-runner tests/ --strict-exit
```

### 2. Use JSON Output for CI

JSON output is easier to parse and integrate with CI reporting tools.

```bash
curl-runner tests/ \
  --continue-on-error \
  --strict-exit \
  --output-format json \
  --output test-results.json
```

### 3. Archive Test Results

Always archive test results as artifacts, even when tests fail:

```yaml
# GitHub Actions
- uses: actions/upload-artifact@v4
  if: always()  # Upload even if tests fail
  with:
    name: test-results
    path: test-results.json
```

### 4. Use Environment Variables for Secrets

Never hardcode API keys or tokens in YAML files:

```yaml
# GitHub Actions
- name: Run Tests
  env:
    API_KEY: ${{ secrets.API_KEY }}
  run: curl-runner tests/
```

### 5. Set Appropriate Timeout

CI environments may have different network conditions:

```bash
curl-runner tests/ \
  --timeout 30000 \
  --retries 3 \
  --retry-delay 2000 \
  --strict-exit
```

## Troubleshooting

### Tests Pass Locally but Fail in CI

Common causes:

1. **Network issues**: Increase timeout and retry settings
2. **Environment variables**: Ensure all required variables are set
3. **Timing issues**: API may respond slower in CI; adjust expectations

### Exit Code Always 0

Check that you're using CI options correctly:

```bash
# This will exit 0 even with failures (default behavior with -c)
curl-runner tests/ -c

# This will exit 1 if any failures occur
curl-runner tests/ -c --strict-exit
```

### JSON Output Not Generated

Ensure you specify both format and output file:

```bash
curl-runner tests/ --output-format json --output results.json
```
