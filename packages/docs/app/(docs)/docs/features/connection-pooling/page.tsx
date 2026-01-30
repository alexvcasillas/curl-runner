import { AlertCircle, CheckCircle, Layers, Network, Timer, Zap } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';

export const metadata: Metadata = {
  title: 'Connection Pooling',
  description:
    'Reuse TCP connections and leverage HTTP/2 multiplexing for faster, more efficient API requests with curl-runner.',
  keywords: [
    'curl-runner connection pooling',
    'TCP connection reuse',
    'HTTP/2 multiplexing',
    'keep-alive connections',
    'connection optimization',
    'parallel requests',
    'batch requests',
    'API performance',
    'curl parallel',
    'stream multiplexing',
  ],
  openGraph: {
    title: 'Connection Pooling | curl-runner Documentation',
    description: 'Reuse TCP connections and leverage HTTP/2 multiplexing for faster API requests.',
    url: 'https://www.curl-runner.com/docs/features/connection-pooling',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Connection Pooling | curl-runner Documentation',
    description: 'Reuse TCP connections with HTTP/2 multiplexing for faster API requests.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/connection-pooling',
  },
};

const cliExample = `# Enable connection pooling via CLI
curl-runner api.yaml -p --connection-pool

# With custom settings
curl-runner api.yaml -p --connection-pool --max-streams 20

# Via environment variables
CURL_RUNNER_CONNECTION_POOL=true curl-runner api.yaml -p

# Full example with env vars
CURL_RUNNER_CONNECTION_POOL=true \\
CURL_RUNNER_MAX_STREAMS_PER_HOST=15 \\
CURL_RUNNER_KEEPALIVE_TIME=120 \\
curl-runner api.yaml -p`;

const basicExample = `global:
  execution: parallel
  connectionPool:
    enabled: true

requests:
  - name: Get Users
    url: https://api.example.com/users
  - name: Get Posts
    url: https://api.example.com/posts
  - name: Get Comments
    url: https://api.example.com/comments`;

const fullConfigExample = `global:
  execution: parallel
  connectionPool:
    enabled: true
    maxStreamsPerHost: 10    # Max concurrent streams per connection
    keepaliveTime: 60        # TCP keepalive in seconds
    connectTimeout: 30       # Connection timeout in seconds

requests:
  - name: Get Users
    url: https://api.example.com/users
    method: GET
  - name: Get Posts
    url: https://api.example.com/posts
    method: GET
  - name: Create Order
    url: https://api.example.com/orders
    method: POST
    body:
      productId: 123
  - name: Get Inventory
    url: https://api.example.com/inventory
    method: GET`;

const multiHostExample = `# Requests are automatically grouped by host
# Each host group uses its own pooled connection

global:
  execution: parallel
  connectionPool:
    enabled: true
    maxStreamsPerHost: 5

requests:
  # Group 1: api.example.com (1 connection, 3 streams)
  - url: https://api.example.com/users
  - url: https://api.example.com/posts
  - url: https://api.example.com/comments

  # Group 2: auth.example.com (1 connection, 2 streams)
  - url: https://auth.example.com/token
  - url: https://auth.example.com/validate

  # Group 3: cdn.example.com (1 connection, 1 stream)
  - url: https://cdn.example.com/assets`;

const beforeAfterComparison = `# WITHOUT Connection Pooling (default)
# Each request = new process + new TCP + new TLS

Request 1 → curl ... → TCP handshake → TLS → HTTP
Request 2 → curl ... → TCP handshake → TLS → HTTP
Request 3 → curl ... → TCP handshake → TLS → HTTP
# Total: 3 TCP handshakes, 3 TLS negotiations

# WITH Connection Pooling
# All same-host requests share ONE connection

Requests 1,2,3 → curl -Z --http2 → TCP → TLS → [stream1, stream2, stream3]
# Total: 1 TCP handshake, 1 TLS negotiation`;

const configOptionsTable = [
  {
    option: 'enabled',
    type: 'boolean',
    default: 'false',
    description: 'Enable connection pooling and HTTP/2 multiplexing',
  },
  {
    option: 'maxStreamsPerHost',
    type: 'number',
    default: '10',
    description: 'Maximum concurrent HTTP/2 streams per host connection',
  },
  {
    option: 'keepaliveTime',
    type: 'number',
    default: '60',
    description: 'TCP keepalive probe interval in seconds',
  },
  {
    option: 'connectTimeout',
    type: 'number',
    default: '30',
    description: 'Connection establishment timeout in seconds',
  },
];

const cliOptionsTable = [
  {
    option: '--connection-pool',
    description: 'Enable TCP connection pooling with HTTP/2 multiplexing',
  },
  {
    option: '--max-streams <n>',
    description: 'Max concurrent streams per host (default: 10)',
  },
  {
    option: '--keepalive-time <sec>',
    description: 'TCP keepalive time in seconds (default: 60)',
  },
  {
    option: '--connect-timeout <sec>',
    description: 'Connection timeout in seconds (default: 30)',
  },
];

const envVarsTable = [
  {
    variable: 'CURL_RUNNER_CONNECTION_POOL',
    description: 'Enable connection pooling (true/false)',
  },
  {
    variable: 'CURL_RUNNER_MAX_STREAMS_PER_HOST',
    description: 'Max concurrent streams per host',
  },
  {
    variable: 'CURL_RUNNER_KEEPALIVE_TIME',
    description: 'TCP keepalive time in seconds',
  },
  {
    variable: 'CURL_RUNNER_CONNECT_TIMEOUT',
    description: 'Connection timeout in seconds',
  },
];

export default function ConnectionPoolingPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Connection Pooling"
          text="Reuse TCP connections and leverage HTTP/2 multiplexing for dramatically faster parallel requests."
        />

        <div className="space-y-12">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Connection pooling groups requests by host and executes them through a single curl
              process with HTTP/2 multiplexing. This eliminates redundant TCP handshakes and TLS
              negotiations, dramatically improving performance for parallel requests to the same
              host.
            </p>

            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-amber-500/10 p-2">
                    <Network className="h-4 w-4 text-amber-600 dark:text-amber-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">1 TCP Handshake</h4>
                    <p className="text-sm text-muted-foreground">Per host instead of per request</p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">1 TLS Negotiation</h4>
                    <p className="text-sm text-muted-foreground">Single handshake for HTTPS</p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Layers className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">HTTP/2 Streams</h4>
                    <p className="text-sm text-muted-foreground">
                      Multiplexed on single connection
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Zap className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">1 DNS Lookup</h4>
                    <p className="text-sm text-muted-foreground">Cached for all requests</p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* How It Works */}
          <section>
            <H2 id="how-it-works">How It Works</H2>
            <p className="text-muted-foreground text-lg mb-6">
              When connection pooling is enabled, curl-runner groups requests by host and executes
              each group in a single batched curl process using{' '}
              <code className="font-mono">curl -Z --parallel --http2</code>.
            </p>

            <CodeBlockServer language="bash" filename="Before vs After">
              {beforeAfterComparison}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-muted/50 p-4">
              <div className="flex items-start gap-3">
                <Timer className="h-5 w-5 text-amber-600 dark:text-amber-400 mt-0.5" />
                <div>
                  <h4 className="font-medium mb-1">Performance Impact</h4>
                  <p className="text-sm text-muted-foreground">
                    For 10 requests to the same host over HTTPS, connection pooling can save ~200ms
                    of TCP/TLS overhead per request, resulting in up to 2 seconds of total time
                    saved.
                  </p>
                </div>
              </div>
            </div>
          </section>

          {/* Basic Usage */}
          <section>
            <H2 id="basic-usage">Basic Usage</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Enable connection pooling with{' '}
              <code className="font-mono">connectionPool.enabled</code> in your global
              configuration.
            </p>

            <CodeBlockServer language="yaml" filename="Basic Connection Pooling">
              {basicExample}
            </CodeBlockServer>
          </section>

          {/* CLI Options */}
          <section>
            <H2 id="cli-options">CLI Options</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Enable and configure connection pooling directly from the command line.
            </p>

            <div className="rounded-lg border overflow-hidden mb-6">
              <table className="w-full text-sm">
                <thead className="bg-muted/50">
                  <tr>
                    <th className="px-4 py-3 text-left font-medium">Option</th>
                    <th className="px-4 py-3 text-left font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  {cliOptionsTable.map((row, idx) => (
                    <tr key={idx} className="border-t">
                      <td className="px-4 py-3 font-mono text-amber-600 dark:text-amber-400">
                        {row.option}
                      </td>
                      <td className="px-4 py-3 text-muted-foreground">{row.description}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            <CodeBlockServer language="bash" filename="CLI Examples">
              {cliExample}
            </CodeBlockServer>
          </section>

          {/* Environment Variables */}
          <section>
            <H2 id="environment-variables">Environment Variables</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Configure connection pooling using environment variables for CI/CD pipelines.
            </p>

            <div className="rounded-lg border overflow-hidden">
              <table className="w-full text-sm">
                <thead className="bg-muted/50">
                  <tr>
                    <th className="px-4 py-3 text-left font-medium">Variable</th>
                    <th className="px-4 py-3 text-left font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  {envVarsTable.map((row, idx) => (
                    <tr key={idx} className="border-t">
                      <td className="px-4 py-3 font-mono text-amber-600 dark:text-amber-400">
                        {row.variable}
                      </td>
                      <td className="px-4 py-3 text-muted-foreground">{row.description}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>

          {/* Configuration Options */}
          <section>
            <H2 id="configuration">Configuration Options</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Fine-tune connection pooling behavior with these configuration options.
            </p>

            <div className="rounded-lg border overflow-hidden mb-6">
              <table className="w-full text-sm">
                <thead className="bg-muted/50">
                  <tr>
                    <th className="px-4 py-3 text-left font-medium">Option</th>
                    <th className="px-4 py-3 text-left font-medium">Type</th>
                    <th className="px-4 py-3 text-left font-medium">Default</th>
                    <th className="px-4 py-3 text-left font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  {configOptionsTable.map((row, idx) => (
                    <tr key={idx} className="border-t">
                      <td className="px-4 py-3 font-mono text-amber-600 dark:text-amber-400">
                        {row.option}
                      </td>
                      <td className="px-4 py-3 text-muted-foreground">{row.type}</td>
                      <td className="px-4 py-3 font-mono">{row.default}</td>
                      <td className="px-4 py-3 text-muted-foreground">{row.description}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            <CodeBlockServer language="yaml" filename="Full Configuration">
              {fullConfigExample}
            </CodeBlockServer>
          </section>

          {/* Multi-Host Behavior */}
          <section>
            <H2 id="multi-host">Multi-Host Behavior</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Requests are automatically grouped by host (scheme + hostname + port). Each host group
              gets its own pooled connection, and groups execute in parallel.
            </p>

            <CodeBlockServer language="yaml" filename="Multi-Host Grouping">
              {multiHostExample}
            </CodeBlockServer>
          </section>

          {/* When to Use */}
          <section>
            <H2 id="when-to-use">When to Use</H2>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Recommended For</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <div>Multiple parallel requests to the same host</div>
                      <div>Performance-critical test suites</div>
                      <div>Load testing with many concurrent requests</div>
                      <div>APIs with high TLS negotiation overhead</div>
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
                    <h4 className="font-medium mb-2">Not Needed For</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <div>Sequential execution (requests run one at a time)</div>
                      <div>Single request executions</div>
                      <div>Requests to many different hosts</div>
                      <div>When request isolation is required</div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Limitations */}
          <section>
            <H2 id="limitations">Limitations</H2>

            <div className="rounded-lg border bg-muted/50 p-4">
              <div className="space-y-3 text-sm">
                <div className="flex items-start gap-2">
                  <AlertCircle className="h-4 w-4 text-orange-500 mt-0.5 flex-shrink-0" />
                  <span>
                    <strong>Parallel mode only:</strong> Connection pooling only works with{' '}
                    <code className="font-mono">execution: parallel</code>. Sequential mode needs
                    ordered results for store variables.
                  </span>
                </div>
                <div className="flex items-start gap-2">
                  <AlertCircle className="h-4 w-4 text-orange-500 mt-0.5 flex-shrink-0" />
                  <span>
                    <strong>Retry handling:</strong> Individual request retries within a batch are
                    not supported. Failed requests in a batch report the batch error.
                  </span>
                </div>
                <div className="flex items-start gap-2">
                  <AlertCircle className="h-4 w-4 text-orange-500 mt-0.5 flex-shrink-0" />
                  <span>
                    <strong>Curl version:</strong> Requires curl 7.66+ for parallel mode support (
                    <code className="font-mono">-Z</code> flag).
                  </span>
                </div>
              </div>
            </div>
          </section>

          {/* Related Features */}
          <section>
            <H2 id="related">Related Features</H2>
            <div className="grid gap-2 text-sm">
              <a
                href="/docs/features/http2"
                className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded hover:bg-muted transition-colors"
              >
                <span>HTTP/2 Support</span>
                <span className="text-muted-foreground">Enable HTTP/2 protocol</span>
              </a>
              <a
                href="/docs/features/parallel-execution"
                className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded hover:bg-muted transition-colors"
              >
                <span>Parallel Execution</span>
                <span className="text-muted-foreground">Run requests simultaneously</span>
              </a>
              <a
                href="/docs/features/performance-profiling"
                className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded hover:bg-muted transition-colors"
              >
                <span>Performance Profiling</span>
                <span className="text-muted-foreground">Measure connection timing metrics</span>
              </a>
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
