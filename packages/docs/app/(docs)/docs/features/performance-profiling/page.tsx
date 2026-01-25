import { BarChart3, Clock, FileJson, Gauge, Terminal, Zap } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'Performance Profiling',
  description:
    'Run HTTP requests multiple times to collect p50/p95/p99 latency statistics. Lightweight performance testing without full load testing tools.',
  keywords: [
    'curl-runner profiling',
    'latency testing',
    'p50 p95 p99',
    'performance testing',
    'API benchmarking',
    'response time stats',
    'lightweight load testing',
    'percentile latency',
  ],
  openGraph: {
    title: 'Performance Profiling | curl-runner Documentation',
    description:
      'Run HTTP requests multiple times to collect p50/p95/p99 latency statistics for lightweight performance testing.',
    url: 'https://www.curl-runner.com/docs/features/performance-profiling',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Performance Profiling | curl-runner Documentation',
    description: 'Learn how to profile API latency with curl-runner.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/performance-profiling',
  },
};

const basicProfileExample = `# Run request 100 times
curl-runner api.yaml -P 100

# Short form equivalent
curl-runner api.yaml --profile 100

# Profile all requests in a directory
curl-runner tests/ -P 50`;

const advancedProfileExample = `# With warmup iterations (exclude first 5 from stats)
curl-runner api.yaml --profile 100 --profile-warmup 5

# Concurrent iterations (10 parallel)
curl-runner api.yaml -P 100 --profile-concurrency 10

# Show histogram distribution
curl-runner api.yaml -P 100 --profile-histogram

# Export raw timings to file
curl-runner api.yaml -P 100 --profile-export results.json
curl-runner api.yaml -P 100 --profile-export results.csv`;

const yamlConfigExample = `# curl-runner.yaml or in your request file
global:
  profile:
    iterations: 100     # Run each request 100 times
    warmup: 5           # Exclude first 5 from stats
    concurrency: 1      # Sequential (default)
    histogram: true     # Show distribution
    exportFile: results.json

requests:
  - name: Health Check
    url: https://api.example.com/health
    method: GET
    expect:
      status: 200`;

const envVarsExample = `# Enable profile mode via environment
CURL_RUNNER_PROFILE=100 curl-runner api.yaml

# Configure all options
CURL_RUNNER_PROFILE=50 \\
CURL_RUNNER_PROFILE_WARMUP=5 \\
CURL_RUNNER_PROFILE_CONCURRENCY=10 \\
CURL_RUNNER_PROFILE_HISTOGRAM=true \\
curl-runner api.yaml`;

const outputExample = `⚡ PROFILING Health Check
   100 iterations, 5 warmup, concurrency: 1

✓ Health Check
   ┌─────────────────────────────────────┐
   │ p50        45.2ms │ min       38.1ms │
   │ p95        89.4ms │ max      142.3ms │
   │ p99       128.7ms │ mean      52.3ms │
   └─────────────────────────────────────┘
   σ 18.42ms | 95 samples | 0 failures (0%)

   Distribution:
    38ms -    51ms │████████████████████████████ 42
    51ms -    64ms │██████████████████████ 33
    64ms -    77ms │████████ 12
    77ms -    90ms │████ 6
    90ms -   103ms │█ 1
   103ms -   116ms │ 0
   116ms -   129ms │█ 1
   129ms -   142ms │ 0`;

const jsonExportExample = `{
  "request": "Health Check",
  "summary": {
    "iterations": 95,
    "warmup": 5,
    "failures": 0,
    "failureRate": 0,
    "min": 38.1,
    "max": 142.3,
    "mean": 52.3,
    "median": 45.2,
    "p50": 45.2,
    "p95": 89.4,
    "p99": 128.7,
    "stdDev": 18.42
  },
  "timings": [42.1, 45.3, 44.8, ...]
}`;

export default function PerformanceProfilingPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Performance Profiling"
          text="Run requests multiple times to collect latency statistics. Lightweight performance testing without full load testing tools."
        />

        <div className="space-y-12">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Profile mode runs each request N times and calculates p50/p95/p99 percentile
              latencies. Ideal for quick performance checks without setting up dedicated load
              testing infrastructure.
            </p>

            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Gauge className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Percentile Stats</h4>
                    <p className="text-sm text-muted-foreground">p50, p95, p99 latency metrics</p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Zap className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Warmup Support</h4>
                    <p className="text-sm text-muted-foreground">Exclude cold-start iterations</p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <BarChart3 className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Histogram</h4>
                    <p className="text-sm text-muted-foreground">Visual latency distribution</p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Basic Usage */}
          <section>
            <H2 id="basic-usage">Basic Usage</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Use the <code className="font-mono">-P</code> or{' '}
              <code className="font-mono">--profile</code> flag followed by the number of
              iterations.
            </p>

            <CodeBlockServer language="bash" filename="Basic Profile Mode">
              {basicProfileExample}
            </CodeBlockServer>
          </section>

          {/* Advanced Options */}
          <section>
            <H2 id="advanced-options">Advanced Options</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Fine-tune profiling behavior with additional flags.
            </p>

            <div className="grid gap-4 mb-6">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-yellow-500/10 p-2">
                    <Clock className="h-4 w-4 text-yellow-600 dark:text-yellow-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --profile-warmup &lt;n&gt;
                      </code>
                      <Badge variant="secondary" className="text-xs">
                        default: 1
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Exclude first N iterations from statistics. Useful for eliminating cold-start
                      latency spikes.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Zap className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --profile-concurrency &lt;n&gt;
                      </code>
                      <Badge variant="secondary" className="text-xs">
                        default: 1
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Run N iterations in parallel. Higher values simulate concurrent load.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <BarChart3 className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --profile-histogram
                      </code>
                      <Badge variant="secondary" className="text-xs">
                        default: off
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Display ASCII histogram showing latency distribution across buckets.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <FileJson className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --profile-export &lt;file&gt;
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Export raw timings and stats to JSON or CSV file for further analysis.
                    </p>
                  </div>
                </div>
              </div>
            </div>

            <CodeBlockServer language="bash" filename="Advanced Options">
              {advancedProfileExample}
            </CodeBlockServer>
          </section>

          {/* YAML Configuration */}
          <section>
            <H2 id="yaml-configuration">YAML Configuration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Configure profiling in your YAML files for repeatable benchmarks.
            </p>

            <CodeBlockServer language="yaml" filename="curl-runner.yaml">
              {yamlConfigExample}
            </CodeBlockServer>
          </section>

          {/* Environment Variables */}
          <section>
            <H2 id="environment-variables">Environment Variables</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Control profiling via environment variables for CI/CD integration.
            </p>

            <div className="grid gap-2 mb-6 text-sm">
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_PROFILE</code>
                <span className="text-muted-foreground">Number of iterations</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_PROFILE_WARMUP</code>
                <span className="text-muted-foreground">Warmup iterations to exclude</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_PROFILE_CONCURRENCY</code>
                <span className="text-muted-foreground">Parallel iteration count</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_PROFILE_HISTOGRAM</code>
                <span className="text-muted-foreground">Show histogram (true/false)</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_PROFILE_EXPORT</code>
                <span className="text-muted-foreground">Export file path</span>
              </div>
            </div>

            <CodeBlockServer language="bash" filename="Environment Variables">
              {envVarsExample}
            </CodeBlockServer>
          </section>

          {/* Output Example */}
          <section>
            <H2 id="output-example">Output Example</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Profile mode output includes percentile stats, standard deviation, and optional
              histogram.
            </p>

            <CodeBlockServer language="text" filename="Profile Output">
              {outputExample}
            </CodeBlockServer>
          </section>

          {/* Export Format */}
          <section>
            <H2 id="export-format">Export Format</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Export results to JSON for programmatic analysis or CSV for spreadsheets.
            </p>

            <CodeBlockServer language="json" filename="results.json">
              {jsonExportExample}
            </CodeBlockServer>
          </section>

          {/* Best Practices */}
          <section>
            <H2 id="best-practices">Best Practices</H2>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Terminal className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Recommended</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <div>• Use 5-10 warmup iterations to exclude cold starts</div>
                      <div>• Run 50-100 iterations for statistically meaningful results</div>
                      <div>• Export results for tracking over time</div>
                      <div>• Profile individual endpoints, not entire test suites</div>
                    </div>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <Clock className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Considerations</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <div>• Not a replacement for dedicated load testing tools</div>
                      <div>• High concurrency may trigger rate limiting</div>
                      <div>• Results vary with network conditions</div>
                      <div>• Cannot be combined with watch mode</div>
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
