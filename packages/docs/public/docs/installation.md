---
title: "Installation"
description: "Install curl-runner on your system using your preferred package manager or download a pre-built binary."
category: "Getting Started"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - installation
  - yaml
  - request
  - cli
  - docker
  - bun
  - npm
slug: "/docs/installation"
toc: true
date: "2026-01-24T16:04:59.525Z"
lastModified: "2026-01-24T16:04:59.525Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "Installation"
  category: "Getting Started"
tags:
  - documentation
  - getting-started
og:
  title: "Installation - curl-runner Documentation"
  description: "Install curl-runner on your system using your preferred package manager or download a pre-built binary."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "Installation"
  description: "Install curl-runner on your system using your preferred package manager or download a pre-built binary."
  datePublished: "2026-01-24T16:04:59.525Z"
  dateModified: "2026-01-24T16:04:59.525Z"
---

# Installation

Install curl-runner on your system using your preferred package manager or download a pre-built binary.

## Prerequisites

`curl-runner` is optimized for Bun and provides the best performance when used with Bun runtime.

### Bun Runtime

```bash
# Install Bun (if not already installed)
curl -fsSL https://bun.sh/install | bash

# Verify Bun installation
bun --version
```

## Installation Methods

Install using Bun for optimal performance and compatibility.

### Bun Package Manager

### npm Package Manager

Install using npm if you prefer Node.js ecosystem tools.

```bash
npm install -g @curl-runner/cli
```

### Prerequisites

**For npm/yarn/pnpm installation:** Bun runtime must be installed on your system.

```bash
bun install -g @curl-runner/cli
```

```bash
npm install -g @curl-runner/cli
```

## Verify Installation

After installation, verify that `curl-runner` is working correctly:

```bash
curl-runner --version
```

## Platform Support

## Troubleshooting

### Common Issues

```bash
# Check if curl-runner is in PATH
which curl-runner

# Add to PATH if needed (replace with your installation path)
export PATH="$PATH:~/.bun/bin"
```

```bash
chmod +x /path/to/curl-runner
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
