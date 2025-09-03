---
title: "curl-runner Documentation"
description: "A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance."
path: "/docs"
generated_at: "2025-09-03T14:32:03.404Z"
generator: "curl-runner-advanced-extractor"
---

# curl-runner Documentation

A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance.

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
  - [1. Create a YAML file](#1-create-a-yaml-file)
- [Next Steps](#next-steps)

## Features

## Quick Start

### 1. Create a YAML file

```bash
# Install curl-runner
bun install -g @curl-runner/cli

# Run your first request
curl-runner simple.yaml
```

## Next Steps

Detailed installation instructions for all platforms

Learn the YAML configuration format and options

Browse real-world examples and use cases

## Examples

### Quick Start Yaml

```
# simple.yaml
request:
  name: Get JSONPlaceholder Post
  url: https://jsonplaceholder.typicode.com/posts/1
  method: GET
```

---

*This documentation was automatically extracted from the curl-runner JSX source files.*

**Links:**
- [curl-runner Documentation](https://curl-runner.dev)
- [GitHub Repository](https://github.com/alexvcasillas/curl-runner)
- [Report Issues](https://github.com/alexvcasillas/curl-runner/issues)

**AI Integration:**
This markdown is optimized for AI tools like Claude, ChatGPT, and GitHub Copilot.
