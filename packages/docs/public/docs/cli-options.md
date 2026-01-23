---
title: "CLI Options"
description: "Comprehensive reference for all command-line options available in curl-runner, including examples and best practices."
category: "Configuration"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - options
  - yaml
  - variables
  - parallel
  - sequential
  - retry
  - timeout
  - headers
  - response
  - request
  - cli
  - environment
slug: "/docs/cli-options"
toc: true
date: "2026-01-23T21:27:49.055Z"
lastModified: "2026-01-23T21:27:49.055Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "CLI Options"
  category: "Configuration"
tags:
  - documentation
  - configuration
og:
  title: "CLI Options - curl-runner Documentation"
  description: "Comprehensive reference for all command-line options available in curl-runner, including examples and best practices."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "CLI Options"
  description: "Comprehensive reference for all command-line options available in curl-runner, including examples and best practices."
  datePublished: "2026-01-23T21:27:49.055Z"
  dateModified: "2026-01-23T21:27:49.055Z"
---

# CLI Options

Comprehensive reference for all command-line options available in curl-runner, including examples and best practices.

`curl-runner` provides numerous CLI options to customize execution behavior, output formatting, and error handling. Options can be combined and have both short and long forms.

## Available Options

Options are grouped by functionality for easier reference.

### General Options

| Option | Short | Description |
|--------|-------|-------------|
| `--help` | `-h` | Show help message |
| `--version` | | Show version number |
| `--verbose` | `-v` | Enable verbose output |
| `--quiet` | `-q` | Suppress non-error output |
| `--all` | | Find all YAML files recursively |

### Execution Options

| Option | Short | Description |
|--------|-------|-------------|
| `--execution parallel` | `-p` | Execute requests in parallel |
| `--continue-on-error` | `-c` | Continue execution on errors |
| `--timeout <ms>` | | Set request timeout in milliseconds |
| `--retries <count>` | | Set maximum retry attempts |
| `--retry-delay <ms>` | | Set delay between retries in milliseconds |
| `--no-retry` | | Disable retry mechanism |

### Output Options

| Option | Short | Description |
|--------|-------|-------------|
| `--output <file>` | `-o` | Save results to file |
| `--output-format <format>` | | Set output format (json\|pretty\|raw) |
| `--pretty-level <level>` | | Set pretty format level (minimal\|standard\|detailed) |
| `--show-headers` | | Include response headers in output |
| `--show-body` | | Include response body in output |
| `--show-metrics` | | Include performance metrics in output |

### CI/CD Options

These options control exit code behavior for CI/CD pipeline integration.

| Option | Description |
|--------|-------------|
| `--strict-exit` | Exit with code 1 if any validation fails |
| `--fail-on <count>` | Exit with code 1 if failures exceed this count |
| `--fail-on-percentage <pct>` | Exit with code 1 if failure percentage exceeds this value |

## Combining Options

Short options can be combined, and multiple options can be used together for powerful configurations.

**Option Combinations**

```bash
# Basic combinations
curl-runner tests/ -v                    # Verbose output
curl-runner tests/ -p                    # Parallel execution
curl-runner tests/ -c                    # Continue on error
curl-runner tests/ -pv                   # Parallel + verbose
curl-runner tests/ -pc                   # Parallel + continue on error
curl-runner tests/ -pvc                  # Parallel + verbose + continue on error

# With long options
curl-runner tests/ --execution parallel --verbose --continue-on-error

# Output format combinations
curl-runner tests/ --output-format json --show-metrics
curl-runner tests/ --output-format pretty --pretty-level detailed
curl-runner tests/ --show-headers --show-body --show-metrics

# Advanced combinations
curl-runner tests/ \\
  --execution parallel \\
  --continue-on-error \\
  --output-format pretty \\
  --pretty-level detailed \\
  --show-headers \\
  --show-metrics \\
  --timeout 30000 \\
  --retries 3 \\
  --retry-delay 2000 \\
  --output results.json \\
  --all

# Retry configurations
curl-runner tests/ --retries 5 --retry-delay 1500  # Custom retry settings
curl-runner tests/ --no-retry                      # Disable retries completely

# CI/CD configurations
curl-runner tests/ --strict-exit                    # Fail if ANY validation fails
curl-runner tests/ -c --strict-exit                 # Run all tests, fail if any fail
curl-runner tests/ --fail-on 2                      # Allow up to 2 failures
curl-runner tests/ --fail-on-percentage 10          # Allow up to 10% failures
```

## CI/CD Exit Codes

By default, curl-runner exits with code 0 if `continueOnError` is set, even when validations fail. The CI/CD options provide finer control over exit behavior.

### Exit Code Behavior

| Configuration | No Failures | Failures Exist |
|--------------|-------------|----------------|
| Default (no options) | Exit 0 | Exit 1 |
| `--continue-on-error` | Exit 0 | Exit 0 |
| `--strict-exit` | Exit 0 | Exit 1 |
| `--continue-on-error --strict-exit` | Exit 0 | Exit 1 |
| `--fail-on N` | Exit 0 | Exit 0 if <= N failures, Exit 1 if > N |
| `--fail-on-percentage P` | Exit 0 | Exit 0 if <= P%, Exit 1 if > P% |

### CI/CD Examples

**Strict Mode (Fail on Any Failure)**

```bash
# GitHub Actions example
- name: Run API Tests
  run: curl-runner tests/ --strict-exit
```

**Run All Tests, Then Fail**

Use `--continue-on-error` with `--strict-exit` to run all tests but still fail the pipeline:

```bash
# Run all tests, then report overall status
curl-runner tests/ --continue-on-error --strict-exit
```

**Tolerance Thresholds**

Allow some failures in large test suites:

```bash
# Allow up to 5 failures
curl-runner tests/ --fail-on 5

# Allow up to 10% of tests to fail
curl-runner tests/ --fail-on-percentage 10
```

**Combined with Output**

```bash
# CI: Run all tests, output JSON, fail if any validation fails
curl-runner tests/ \
  --continue-on-error \
  --strict-exit \
  --output-format json \
  --output results.json
```

## Environment Variables

Many CLI options can be set via environment variables, which take precedence over default values but are overridden by explicit CLI options.

### Supported Environment Variables

**Environment Variable Examples**

```bash
# Environment variables override CLI options
CURL_RUNNER_TIMEOUT=10000 curl-runner tests/
CURL_RUNNER_RETRIES=3 curl-runner tests/
CURL_RUNNER_RETRY_DELAY=2000 curl-runner tests/
CURL_RUNNER_VERBOSE=true curl-runner tests/
CURL_RUNNER_EXECUTION=parallel curl-runner tests/
CURL_RUNNER_CONTINUE_ON_ERROR=true curl-runner tests/

# Output format environment variables
CURL_RUNNER_OUTPUT_FORMAT=json curl-runner tests/
CURL_RUNNER_PRETTY_LEVEL=detailed curl-runner tests/
CURL_RUNNER_OUTPUT_FILE=results.json curl-runner tests/

# Multiple environment variables
CURL_RUNNER_TIMEOUT=15000 \\
CURL_RUNNER_RETRIES=2 \\
CURL_RUNNER_RETRY_DELAY=1500 \\
CURL_RUNNER_VERBOSE=true \\
CURL_RUNNER_OUTPUT_FORMAT=pretty \\
CURL_RUNNER_PRETTY_LEVEL=detailed \\
curl-runner tests/ --execution parallel

# CI/CD environment variables
CURL_RUNNER_STRICT_EXIT=true curl-runner tests/
CURL_RUNNER_FAIL_ON=5 curl-runner tests/
CURL_RUNNER_FAIL_ON_PERCENTAGE=10 curl-runner tests/

# Mix environment variables and CLI options
CURL_RUNNER_TIMEOUT=10000 \\
CURL_RUNNER_OUTPUT_FORMAT=json \\
curl-runner tests/ --verbose --show-metrics --output results.json
```

## Configuration File

Create a `curl-runner.yaml` file in your project root to set default options.

**curl-runner.yaml**

```yaml
# curl-runner.yaml (configuration file)
global:
  execution: parallel
  continueOnError: true
  output:
    verbose: false
    format: pretty
    prettyLevel: detailed
    showHeaders: true
    showBody: true
    showMetrics: true
    saveToFile: results.json
  defaults:
    timeout: 10000
    retry:
      count: 2
      delay: 1000
  variables:
    API_BASE: "https://api.example.com"
    API_VERSION: "v2"

# CLI options override config file settings
curl-runner tests/ --verbose --pretty-level minimal  # Overrides config file settings
```

## Output Format

When using `--output`, results are saved in structured JSON format.

**results.json**

```json
# JSON output format (when using --output)
{
  "summary": {
    "total": 10,
    "successful": 8,
    "failed": 2,
    "duration": 2345
  },
  "requests": [
    {
      "name": "Get Users",
      "status": "success",
      "statusCode": 200,
      "duration": 234,
      "url": "https://api.example.com/users"
    },
    {
      "name": "Invalid Request", 
      "status": "failed",
      "error": "Request timeout after 5000ms",
      "url": "https://slow-api.example.com/endpoint"
    }
  ]
}
```

## Option Precedence

When options are specified in multiple ways, curl-runner follows this precedence order:

## Best Practices

### Best Practices

• Use descriptive variable names
• Define common values as variables
• Use environment variables for secrets
• Group related variables logically
• Document complex expressions
