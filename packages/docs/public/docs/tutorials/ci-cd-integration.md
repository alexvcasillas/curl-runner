---
title: "CI/CD Integration with GitHub Actions"
description: "Set up curl-runner in your CI/CD pipeline. Configure exit codes, failure thresholds, and integrate with GitHub Actions for automated API testing."
category: "Tutorials"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - tutorial
  - ci-cd
  - github-actions
  - automation
  - pipeline
slug: "/docs/tutorials/ci-cd-integration"
toc: true
date: "2026-01-24T00:00:00.000Z"
lastModified: "2026-01-24T00:00:00.000Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "CI/CD Integration"
  category: "Tutorials"
tags:
  - tutorials
  - ci-cd
og:
  title: "CI/CD Integration with GitHub Actions - curl-runner Documentation"
  description: "Set up curl-runner in your CI/CD pipeline for automated API testing."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "CI/CD Integration with GitHub Actions"
  description: "Set up curl-runner in your CI/CD pipeline for automated API testing."
  datePublished: "2026-01-24T00:00:00.000Z"
  dateModified: "2026-01-24T00:00:00.000Z"
---

# CI/CD Integration with GitHub Actions

In this tutorial, you'll learn how to integrate curl-runner into your CI/CD pipeline using GitHub Actions for automated API testing.

## Understanding Exit Codes

curl-runner uses exit codes to communicate test results to CI/CD systems:

- **Exit code 0**: All tests passed
- **Exit code 1**: One or more tests failed

By default, curl-runner returns exit code 1 only if a request completely fails (network error, timeout). For stricter validation, use the CI options.

## CI/CD Configuration Options

### Strict Exit Mode

Fail the pipeline if ANY validation fails:

```yaml
global:
  ci:
    strictExit: true

requests:
  - name: Health Check
    url: https://api.example.com/health
    method: GET
    expect:
      status: 200
```

Or via CLI:

```bash
curl-runner tests/ --strict-exit
```

### Failure Threshold

Allow some failures before failing the pipeline:

```yaml
global:
  ci:
    failOn: 3  # Fail if more than 3 requests fail

requests:
  - name: Test 1
    # ...
```

Or via CLI:

```bash
curl-runner tests/ --fail-on 3
```

### Percentage Threshold

Fail if failure rate exceeds a percentage:

```yaml
global:
  ci:
    failOnPercentage: 10  # Fail if more than 10% fail

requests:
  - name: Test 1
    # ...
```

Or via CLI:

```bash
curl-runner tests/ --fail-on-percentage 10
```

## GitHub Actions Setup

### Basic Workflow

Create `.github/workflows/api-tests.yml`:

```yaml
name: API Tests

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  api-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v1
        with:
          bun-version: latest

      - name: Install curl-runner
        run: bun install -g curl-runner

      - name: Run API Tests
        run: curl-runner tests/ --strict-exit
```

### With Environment Variables

For testing different environments:

```yaml
name: API Tests

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CURL_RUNNER_TIMEOUT: 30000
  CURL_RUNNER_RETRIES: 3

jobs:
  api-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v1

      - name: Install curl-runner
        run: bun install -g curl-runner

      - name: Run API Tests
        env:
          API_URL: ${{ secrets.API_URL }}
          API_TOKEN: ${{ secrets.API_TOKEN }}
        run: curl-runner tests/ --strict-exit -v
```

### Multi-Environment Testing

Test against staging and production:

```yaml
name: API Tests

on:
  push:
    branches: [main]

jobs:
  test-staging:
    runs-on: ubuntu-latest
    environment: staging

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v1

      - name: Install curl-runner
        run: bun install -g curl-runner

      - name: Run Staging Tests
        env:
          BASE_URL: ${{ vars.STAGING_URL }}
          API_TOKEN: ${{ secrets.STAGING_TOKEN }}
        run: curl-runner tests/ --strict-exit

  test-production:
    runs-on: ubuntu-latest
    environment: production
    needs: test-staging

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v1

      - name: Install curl-runner
        run: bun install -g curl-runner

      - name: Run Production Tests
        env:
          BASE_URL: ${{ vars.PRODUCTION_URL }}
          API_TOKEN: ${{ secrets.PRODUCTION_TOKEN }}
        run: curl-runner tests/ --strict-exit
```

## Test Organization

### Directory Structure

Organize your tests for CI/CD:

```
tests/
├── health/
│   └── health-check.yaml
├── auth/
│   └── login-flow.yaml
├── api/
│   ├── users.yaml
│   ├── posts.yaml
│   └── comments.yaml
└── integration/
    └── full-workflow.yaml
```

### Configuration File

Create `curl-runner.yaml` in your project root for default settings:

```yaml
global:
  execution: parallel
  continueOnError: true
  defaults:
    timeout: 10000
    retry:
      count: 2
      delay: 1000
  output:
    format: pretty
    prettyLevel: minimal
  ci:
    strictExit: true
    failOnPercentage: 5
```

## Saving Test Results

### JSON Output

Save results for further processing:

```yaml
name: API Tests

jobs:
  api-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v1

      - name: Install curl-runner
        run: bun install -g curl-runner

      - name: Run API Tests
        run: curl-runner tests/ --strict-exit -o results.json

      - name: Upload Results
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: api-test-results
          path: results.json
```

### Test Summary

Add a summary to the GitHub Actions UI:

```yaml
name: API Tests

jobs:
  api-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v1

      - name: Install curl-runner
        run: bun install -g curl-runner

      - name: Run API Tests
        id: tests
        run: |
          curl-runner tests/ --strict-exit -o results.json
        continue-on-error: true

      - name: Generate Summary
        if: always()
        run: |
          echo "## API Test Results" >> $GITHUB_STEP_SUMMARY
          echo "" >> $GITHUB_STEP_SUMMARY
          if [ -f results.json ]; then
            echo "Results saved to results.json" >> $GITHUB_STEP_SUMMARY
          fi

      - name: Check Test Status
        if: steps.tests.outcome == 'failure'
        run: exit 1
```

## Scheduled Testing

Run tests on a schedule for monitoring:

```yaml
name: Scheduled API Tests

on:
  schedule:
    - cron: '0 */6 * * *'  # Every 6 hours
  workflow_dispatch:  # Manual trigger

jobs:
  health-check:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v1

      - name: Install curl-runner
        run: bun install -g curl-runner

      - name: Run Health Checks
        env:
          BASE_URL: ${{ vars.PRODUCTION_URL }}
        run: curl-runner tests/health/ --strict-exit

      - name: Notify on Failure
        if: failure()
        uses: actions/github-script@v7
        with:
          script: |
            github.rest.issues.create({
              owner: context.repo.owner,
              repo: context.repo.repo,
              title: 'API Health Check Failed',
              body: 'Scheduled health check failed. Please investigate.'
            })
```

## Best Practices

### 1. Use Retries for Flaky Networks

```yaml
global:
  defaults:
    retry:
      count: 3
      delay: 2000
```

### 2. Set Reasonable Timeouts

```yaml
global:
  defaults:
    timeout: 30000  # 30 seconds
```

### 3. Continue on Error for Full Reports

```yaml
global:
  continueOnError: true
  ci:
    failOnPercentage: 0  # Still fail if any test fails
```

### 4. Use Parallel Execution for Speed

```yaml
global:
  execution: parallel
```

### 5. Separate Critical vs Non-Critical Tests

```yaml
# critical-tests.yaml - must all pass
global:
  ci:
    strictExit: true

# non-critical-tests.yaml - some failures ok
global:
  ci:
    failOnPercentage: 20
```

## What You Learned

In this tutorial, you learned how to:

- Configure exit codes for CI/CD integration
- Set up GitHub Actions workflows
- Use environment variables and secrets
- Organize tests for maintainability
- Save and process test results
- Set up scheduled monitoring
- Apply best practices for CI/CD testing

## Next Steps

- [Parallel Execution](/docs/tutorials/parallel-execution) - Speed up your test suites
- [Environment Variables](/docs/environment-variables) - Configure via environment
- [CLI Options](/docs/cli-options) - All command-line options
