#!/usr/bin/env bun
// @ts-nocheck

import { $ } from 'bun';
import packageJson from '../package.json';

const version = packageJson.version;

// Build the dist directory for global package usage
await $`CURL_RUNNER_VERSION=${version} bun build src/cli.ts --target=bun --minify --sourcemap --outdir dist`;

console.log(`\n\r✅ Built curl-runner package v${version}\n\r`);
