import path from 'node:path';
import type { NextConfig } from 'next';

/** The dashboard is never framed: a click on it from another site is a click we didn't ask for. */
export const FRAME_HEADERS = [
  { key: 'X-Frame-Options', value: 'DENY' },
  { key: 'Content-Security-Policy', value: "frame-ancestors 'none'" },
];

const nextConfig: NextConfig = {
  // The Docker image runs `.next/standalone`. Tracing from the repo root keeps the
  // pnpm workspace's hoisted dependencies inside it.
  output: 'standalone',
  outputFileTracingRoot: path.join(__dirname, '../..'),
  poweredByHeader: false,
  reactStrictMode: true,
  async headers() {
    return [{ source: '/:path*', headers: FRAME_HEADERS }];
  },
};

export default nextConfig;
