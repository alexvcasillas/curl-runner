import path from 'node:path';
import type { NextConfig } from 'next';

const nextConfig: NextConfig = {
  // The Docker image runs `.next/standalone`. Tracing from the repo root keeps the
  // pnpm workspace's hoisted dependencies inside it.
  output: 'standalone',
  outputFileTracingRoot: path.join(__dirname, '../..'),
  poweredByHeader: false,
  reactStrictMode: true,
};

export default nextConfig;
