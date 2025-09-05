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
  - cli
  - docker
  - bun
  - npm
slug: "/docs/installation"
toc: true
date: "2025-09-05T06:23:00.139Z"
lastModified: "2025-09-05T06:23:00.139Z"
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
  datePublished: "2025-09-05T06:23:00.139Z"
  dateModified: "2025-09-05T06:23:00.139Z"
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
npm install -g curl-runner
```

### Pre-built Binary

Download a standalone executable for your platform.

```bash
# Pull the Docker image
docker pull curlrunner/curl-runner:latest

# Run tests using Docker
docker run --rm -v "$(pwd)":/workspace curlrunner/curl-runner:latest my-tests.yaml
```

```bash
bun install -g curl-runner
```

```bash
npm install -g curl-runner
```

```bash
# Pull the Docker image
docker pull curlrunner/curl-runner:latest

# Run tests using Docker
docker run --rm -v "$(pwd)":/workspace curlrunner/curl-runner:latest my-tests.yaml
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
