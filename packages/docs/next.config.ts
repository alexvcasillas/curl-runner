import type { NextConfig } from 'next';

const nextConfig: NextConfig = {
  // /changelog and /rss.xml read the CLI changelog from disk; ship it with their functions
  // so hourly regenerations can read it too.
  outputFileTracingIncludes: {
    '/changelog': ['../cli/CHANGELOG.md'],
    '/rss.xml': ['../cli/CHANGELOG.md'],
  },
  images: {
    remotePatterns: [
      {
        protocol: 'https',
        hostname: 'github.com',
        port: '',
        pathname: '/**',
      },
    ],
  },
};

export default nextConfig;
