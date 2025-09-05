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
date: "2025-09-05T11:38:17.397Z"
lastModified: "2025-09-05T11:38:17.397Z"
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
  datePublished: "2025-09-05T11:38:17.397Z"
  dateModified: "2025-09-05T11:38:17.397Z"
---

# CLI Options

Comprehensive reference for all command-line options available in curl-runner, including examples and best practices.

`curl-runner` provides numerous CLI options to customize execution behavior, output formatting, and error handling. Options can be combined and have both short and long forms.

## Available Options

Options are grouped by functionality for easier reference.

{option.description}

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
