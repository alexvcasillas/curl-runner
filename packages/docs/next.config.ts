import { createMDX } from 'fumadocs-mdx/next';
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
  // Every docs page as Markdown: /docs.md and /docs/<page>.md, rendered by app/llms.mdx.
  async rewrites() {
    return [
      { source: '/docs.md', destination: '/llms.mdx' },
      { source: '/docs/:path*.md', destination: '/llms.mdx/:path*' },
    ];
  },
};

const withMDX = createMDX();

export default withMDX(nextConfig);
