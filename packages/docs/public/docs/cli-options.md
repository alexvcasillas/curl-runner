---
title: "CLI Options"
description: "Comprehensive reference for all command-line options available in curl-runner, including examples and best practices."
---

# CLI Options

Comprehensive reference for all command-line options available in curl-runner, including examples and best practices.

## Table of Contents

- [Available Options](#available-options)
- [Combining Options](#combining-options)
- [Environment Variables](#environment-variables)
  - [Supported Environment Variables](#supported-environment-variables)
- [Configuration File](#configuration-file)
- [Output Format](#output-format)
- [Option Precedence](#option-precedence)
- [Best Practices](#best-practices)

## Available Options

## Combining Options

```bash title="Option Combinations"
# Basic combinations
curl-runner tests/ -v                    # Verbose output
curl-runner tests/ -p                    # Parallel execution
curl-runner tests/ -c                    # Continue on error
curl-runner tests/ -pv                   # Parallel + verbose
curl-runner tests/ -pc                   # Parallel + continue on error
curl-runner tests/ -pvc                  # Parallel + verbose + continue on error

# With long options
curl-runner tests/ --execution parallel --verbose --continue-on-error

# Advanced combinations
curl-runner tests/ \\
  --execution parallel \\
  --continue-on-error \\
  --verbose \\
  --timeout 30000 \\
  --retries 3 \\
  --output results.json \\
  --all
```

## Environment Variables

### Supported Environment Variables

```bash title="Environment Variable Examples"
# Environment variables override CLI options
CURL_RUNNER_TIMEOUT=10000 curl-runner tests/
CURL_RUNNER_RETRIES=3 curl-runner tests/
CURL_RUNNER_VERBOSE=true curl-runner tests/

# Multiple environment variables
CURL_RUNNER_TIMEOUT=15000 \\
CURL_RUNNER_RETRIES=2 \\
CURL_RUNNER_VERBOSE=true \\
curl-runner tests/ --execution parallel

# Mix environment variables and CLI options
CURL_RUNNER_TIMEOUT=10000 curl-runner tests/ --verbose --output results.json
```

## Configuration File

```json title=".curl-runner.json"
# .curl-runner.json (configuration file)
{
  "execution": "parallel",
  "continueOnError": true,
  "verbose": false,
  "timeout": 10000,
  "retries": 2,
  "output": "results.json"
}

# CLI options override config file settings
curl-runner tests/ --verbose  # Overrides verbose: false in config
```

## Output Format

```json title="results.json"
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

Command-line flags take highest precedence

Override defaults but yield to CLI flags

Project-specific defaults from .curl-runner.json

Fallback values when nothing else is specified

## Best Practices

> **Recommended Practices**
>


> **Important Cautions**
>


