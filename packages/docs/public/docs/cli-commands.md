---
title: "CLI Commands"
description: "Complete command-line interface reference for curl-runner with all available commands, options, and usage examples."
category: "Configuration"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - commands
  - yaml
  - variables
  - parallel
  - sequential
  - retry
  - timeout
  - response
  - request
  - cli
  - environment
slug: "/docs/cli-commands"
toc: true
date: "2025-09-04T14:20:38.935Z"
lastModified: "2025-09-04T14:20:38.935Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "CLI Commands"
  category: "Configuration"
tags:
  - documentation
  - configuration
og:
  title: "CLI Commands - curl-runner Documentation"
  description: "Complete command-line interface reference for curl-runner with all available commands, options, and usage examples."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "CLI Commands"
  description: "Complete command-line interface reference for curl-runner with all available commands, options, and usage examples."
  datePublished: "2025-09-04T14:20:38.935Z"
  dateModified: "2025-09-04T14:20:38.935Z"
---

# CLI Commands

Complete command-line interface reference for curl-runner with all available commands, options, and usage examples.

## Basic Usage

The `curl-runner` CLI provides a simple interface for executing HTTP requests defined in YAML files.

**Basic Usage Examples**

```bash
# Run a single YAML file
curl-runner simple.yaml

# Run with verbose output
curl-runner simple.yaml --verbose

# Run multiple files
curl-runner file1.yaml file2.yaml file3.yaml

# Run all YAML files in current directory
curl-runner *.yaml

# Run all files in a directory
curl-runner tests/

# Run with short options
curl-runner tests/ -pvc
```

## Commands

## Options

Available command-line options to customize `curl-runner`'s behavior.

{option.description}

## File Patterns

`curl-runner` supports various file patterns and glob expressions for flexible file selection.

**File Pattern Examples**

```bash
# Glob patterns
curl-runner "api-*.yaml"        # Files starting with "api-"
curl-runner "**/*.yaml"         # All YAML files recursively
curl-runner "tests/**/*.yml"    # All YML files in tests/ recursively

# Directory patterns
curl-runner tests/              # All YAML/YML files in tests/
curl-runner tests/ --all        # Recursive search in tests/
curl-runner . --all             # All files recursively from current dir

# Multiple patterns
curl-runner "api-*.yaml" "user-*.yaml" tests/
```

## Execution Modes

**Execution Mode Examples**

```bash
# Sequential execution (default)
curl-runner tests/

# Parallel execution
curl-runner tests/ --execution parallel
curl-runner tests/ -p

# Continue on errors
curl-runner tests/ --continue-on-error
curl-runner tests/ -c

# Parallel + continue on error
curl-runner tests/ -pc
```

## Output Options

Control how `curl-runner` displays results and saves output data.

**Output Examples**

```bash
# Verbose output
curl-runner tests/ --verbose
curl-runner tests/ -v

# Save results to JSON file
curl-runner tests/ --output results.json

# Combine options
curl-runner tests/ --verbose --output results.json --continue-on-error
```

## Timeout & Retry Options

Configure timeout and retry behavior for robust request handling.

**Timeout Examples**

```bash
# Set global timeout (milliseconds)
curl-runner tests/ --timeout 10000

# Set maximum retries
curl-runner tests/ --retries 3

# Combine timeout and retries
curl-runner tests/ --timeout 5000 --retries 2
```

## Advanced Usage

Complex examples combining multiple options and environment variables.

**Advanced Examples**

```bash
# Environment-specific execution
NODE_ENV=production curl-runner api-tests.yaml

# Using environment variables
API_KEY=secret123 curl-runner auth-tests.yaml

# Complex execution with all options
curl-runner tests/ \\
  --execution parallel \\
  --continue-on-error \\
  --verbose \\
  --timeout 30000 \\
  --retries 3 \\
  --output test-results.json \\
  --all
```

## Exit Codes

`curl-runner` returns specific exit codes to indicate execution results.
