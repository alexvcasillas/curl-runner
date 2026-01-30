import { AlertCircle, CheckCircle, Network, Zap } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';

export const metadata: Metadata = {
  title: 'HTTP/2 Support',
  description:
    'Enable HTTP/2 protocol with multiplexing for faster, more efficient API requests with curl-runner.',
  keywords: [
    'curl-runner http2',
    'HTTP/2 protocol',
    'multiplexing',
    'http2 curl',
    'fast API requests',
    'connection pooling',
    'header compression',
    'modern HTTP',
    'API performance',
    'curl http2 flag',
  ],
  openGraph: {
    title: 'HTTP/2 Support | curl-runner Documentation',
    description:
      'Enable HTTP/2 protocol with multiplexing for faster, more efficient API requests.',
    url: 'https://www.curl-runner.com/docs/features/http2',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'HTTP/2 Support | curl-runner Documentation',
    description: 'Enable HTTP/2 protocol with multiplexing for faster API requests.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/http2',
  },
};

const basicExample = `# Enable HTTP/2 for all requests
curl-runner api.yaml --http2

# Combine with other flags
curl-runner api.yaml --http2 -p -v

# Run a directory with HTTP/2
curl-runner tests/ --http2`;

const yamlGlobalExample = `# Enable HTTP/2 globally via defaults
global:
  defaults:
    http2: true

requests:
  - name: Get Users
    url: https://api.example.com/users
    method: GET

  - name: Get Posts
    url: https://api.example.com/posts
    method: GET`;

const yamlPerRequestExample = `# Enable HTTP/2 per request
requests:
  - name: HTTP/2 Request
    url: https://api.example.com/data
    http2: true

  - name: HTTP/1.1 Request
    url: https://legacy-api.example.com/data
    http2: false`;

const envVarsExample = `# Enable HTTP/2 via environment variable
CURL_RUNNER_HTTP2=true curl-runner api.yaml

# Useful in scripts or CI/CD
export CURL_RUNNER_HTTP2=true
curl-runner tests/`;

const parallelExample = `# Combine HTTP/2 with parallel execution for maximum performance
curl-runner api.yaml --http2 -p --max-concurrent 10

# YAML configuration
global:
  execution: parallel
  maxConcurrency: 10
  defaults:
    http2: true

requests:
  - name: Request 1
    url: https://api.example.com/endpoint1
  - name: Request 2
    url: https://api.example.com/endpoint2
  - name: Request 3
    url: https://api.example.com/endpoint3`;

export default function Http2Page() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="HTTP/2 Support"
          text="Enable HTTP/2 protocol with multiplexing for faster, more efficient API requests."
        />

        <div className="space-y-12">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground text-lg mb-6">
              HTTP/2 offers significant performance improvements over HTTP/1.1 including
              multiplexing, header compression, and better connection utilization. Enable it with
              the <code className="font-mono">--http2</code> flag to take advantage of modern server
              infrastructure.
            </p>

            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-sky-500/10 p-2">
                    <Network className="h-4 w-4 text-sky-600 dark:text-sky-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Multiplexing</h4>
                    <p className="text-sm text-muted-foreground">
                      Multiple requests over a single connection
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Zap className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Lower Latency</h4>
                    <p className="text-sm text-muted-foreground">
                      Reduced connection overhead and faster responses
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Header Compression</h4>
                    <p className="text-sm text-muted-foreground">
                      Automatic HPACK compression for headers
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Basic Usage */}
          <section>
            <H2 id="basic-usage">Basic Usage</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Enable HTTP/2 with the <code className="font-mono">--http2</code> CLI flag.
            </p>

            <CodeBlockServer language="bash" filename="CLI Usage">
              {basicExample}
            </CodeBlockServer>
          </section>

          {/* YAML Configuration */}
          <section>
            <H2 id="yaml-configuration">YAML Configuration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Configure HTTP/2 in your YAML files globally or per-request.
            </p>

            <div className="space-y-6">
              <div>
                <h3 className="text-lg font-semibold mb-3">Global Configuration</h3>
                <CodeBlockServer language="yaml" filename="Global HTTP/2">
                  {yamlGlobalExample}
                </CodeBlockServer>
              </div>

              <div>
                <h3 className="text-lg font-semibold mb-3">Per-Request Configuration</h3>
                <CodeBlockServer language="yaml" filename="Per-Request HTTP/2">
                  {yamlPerRequestExample}
                </CodeBlockServer>
              </div>
            </div>
          </section>

          {/* Environment Variables */}
          <section>
            <H2 id="environment-variables">Environment Variables</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Enable HTTP/2 via environment variable for scripted workflows and CI/CD pipelines.
            </p>

            <div className="grid gap-2 mb-6 text-sm">
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_HTTP2</code>
                <span className="text-muted-foreground">Enable HTTP/2 protocol (true/false)</span>
              </div>
            </div>

            <CodeBlockServer language="bash" filename="Environment Variables">
              {envVarsExample}
            </CodeBlockServer>
          </section>

          {/* With Parallel Execution */}
          <section>
            <H2 id="parallel-execution">With Parallel Execution</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Combine HTTP/2 with parallel execution for maximum performance. HTTP/2 multiplexing
              works particularly well with concurrent requests.
            </p>

            <CodeBlockServer language="bash" filename="HTTP/2 + Parallel">
              {parallelExample}
            </CodeBlockServer>
          </section>

          {/* Best Practices */}
          <section>
            <H2 id="best-practices">Best Practices</H2>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Recommended</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <div>• Use HTTP/2 for modern APIs that support it</div>
                      <div>• Combine with parallel execution for best results</div>
                      <div>• Enable globally when testing HTTP/2 servers</div>
                      <div>• Use environment variable in CI/CD pipelines</div>
                    </div>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <AlertCircle className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Notes</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <div>• Server must support HTTP/2 (most modern servers do)</div>
                      <div>• Falls back to HTTP/1.1 if server doesn't support it</div>
                      <div>• Requires HTTPS for most servers (HTTP/2 over TLS)</div>
                      <div>• Per-request config overrides global setting</div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>

      {/* Table of Contents */}
      <div className="hidden text-sm xl:block">
        <div className="sticky top-16 -mt-10 pt-4">
          <TableOfContents />
        </div>
      </div>
    </main>
  );
}
