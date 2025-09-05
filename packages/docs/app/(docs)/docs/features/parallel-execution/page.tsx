import { AlertCircle, CheckCircle, Clock, Gauge, Shield, TrendingUp } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'Parallel Execution',
  description:
    'Execute HTTP requests in parallel for faster performance. Learn about concurrent request execution, performance optimization, and best practices.',
  keywords: [
    'curl-runner parallel execution',
    'concurrent HTTP requests',
    'parallel request processing',
    'performance optimization',
    'async HTTP requests',
    'concurrent API calls',
    'parallel testing',
    'fast request execution',
    'simultaneous requests',
    'parallel mode',
    'concurrent processing',
    'HTTP performance',
  ],
  openGraph: {
    title: 'Parallel Execution | curl-runner Documentation',
    description:
      'Execute HTTP requests in parallel for faster performance. Learn about concurrent request execution, performance optimization, and best practices.',
    url: 'https://curl-runner.com/docs/features/parallel-execution',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Parallel Execution | curl-runner Documentation',
    description:
      'Learn how to execute HTTP requests in parallel for faster performance with curl-runner.',
  },
  alternates: {
    canonical: 'https://curl-runner.com/docs/features/parallel-execution',
  },
};

const parallelExample = `# Run multiple requests in parallel
global:
  execution: parallel
  variables:
    API_URL: https://api.example.com
  output:
    showMetrics: true

requests:
  - name: Get Users
    url: \${API_URL}/users
    method: GET
    
  - name: Get Posts
    url: \${API_URL}/posts
    method: GET
    
  - name: Get Comments
    url: \${API_URL}/comments
    method: GET
    
  - name: Get Products
    url: \${API_URL}/products
    method: GET`;

const concurrencyExample = `# Control parallel execution
global:
  execution: parallel
  continueOnError: true  # Continue even if some requests fail
  variables:
    BASE_URL: https://api.example.com
  output:
    verbose: true
    showMetrics: true

requests:
  # These will all run at the same time
  - name: Request 1
    url: \${BASE_URL}/endpoint1
    timeout: 5000
    
  - name: Request 2
    url: \${BASE_URL}/endpoint2
    timeout: 5000
    
  - name: Request 3
    url: \${BASE_URL}/endpoint3
    timeout: 5000`;

const performanceExample = `# Performance testing with parallel requests
global:
  execution: parallel
  variables:
    API_URL: https://api.example.com
    AUTH_TOKEN: your-token-here
  output:
    showMetrics: true
    saveToFile: performance-results.json

# Test multiple endpoints simultaneously
requests:
  - name: Health Check 1
    url: \${API_URL}/health
    
  - name: Health Check 2
    url: \${API_URL}/health
    
  - name: Health Check 3
    url: \${API_URL}/health
    
  - name: Health Check 4
    url: \${API_URL}/health
    
  - name: Health Check 5
    url: \${API_URL}/health`;

const cliUsage = `# Run with parallel execution
curl-runner api-tests.yaml --parallel

# Override file setting to run sequentially
curl-runner api-tests.yaml --sequential

# Run parallel with verbose output
curl-runner api-tests.yaml --parallel --verbose`;

export default function ParallelExecutionPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Parallel Execution"
          text="Execute multiple HTTP requests simultaneously for improved performance."
        />

        <div className="space-y-8">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground mb-6">
              Parallel execution allows curl-runner to send multiple HTTP requests simultaneously,
              significantly reducing the total execution time when testing multiple endpoints or
              performing load testing.
            </p>

            <div className="rounded-lg border bg-muted/50 p-4">
              <p className="text-sm">
                <strong>Note:</strong> By default, curl-runner executes requests sequentially.
                Enable parallel execution using the{' '}
                <code className="text-xs bg-background px-1 py-0.5 rounded">
                  {'execution: parallel'}
                </code>{' '}
                setting in your YAML file or the{' '}
                <code className="text-xs bg-background px-1 py-0.5 rounded">--parallel</code> CLI
                flag.
              </p>
            </div>
          </section>

          {/* Basic Usage */}
          <section>
            <H2 id="basic-usage">Basic Usage</H2>
            <p className="text-muted-foreground mb-6">
              Configure parallel execution in your YAML file using the global execution setting.
            </p>

            <CodeBlockServer language="yaml" filename="parallel-requests.yaml">
              {parallelExample}
            </CodeBlockServer>
          </section>

          {/* Configuration Options */}
          <section>
            <H2 id="configuration">Configuration Options</H2>
            <p className="text-muted-foreground mb-6">
              Control how parallel execution behaves with these settings:
            </p>

            <div className="border rounded-lg overflow-hidden">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Setting</th>
                    <th className="text-left p-3 font-medium">Type</th>
                    <th className="text-left p-3 font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">execution</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">string</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      "parallel" or "sequential" (default: "sequential")
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">continueOnError</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">boolean</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Continue executing remaining requests if one fails
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">timeout</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">number</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Global timeout for each request in milliseconds
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Error Handling */}
          <section>
            <H2 id="error-handling">Error Handling</H2>
            <p className="text-muted-foreground mb-6">
              When running requests in parallel, you can control how errors are handled using the{' '}
              <code className="text-sm bg-muted px-1 py-0.5 rounded">continueOnError</code> setting.
            </p>

            <CodeBlockServer language="yaml" filename="parallel-with-error-handling.yaml">
              {concurrencyExample}
            </CodeBlockServer>

            <div className="mt-6 grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <AlertCircle className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div>
                    <p className="font-medium">Stop on Error</p>
                    <p className="text-sm text-muted-foreground mt-1">
                      By default, if any request fails, curl-runner will cancel remaining requests
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <p className="font-medium">
                      Continue on Error{' '}
                      <Badge className="ml-2 bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20">
                        Recommended
                      </Badge>
                    </p>
                    <p className="text-sm text-muted-foreground mt-1">
                      Set{' '}
                      <code className="text-xs bg-muted px-1 py-0.5 rounded">
                        {'continueOnError: true'}
                      </code>{' '}
                      to complete all requests regardless of failures
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Performance Testing */}
          <section>
            <H2 id="performance-testing">Performance Testing</H2>
            <p className="text-muted-foreground mb-6">
              Parallel execution is ideal for performance testing and load simulation. Use it to
              test how your API handles concurrent requests.
            </p>

            <CodeBlockServer language="yaml" filename="performance-test.yaml">
              {performanceExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-muted/50 p-4">
              <h4 className="text-sm font-medium mb-2">Metrics Collected</h4>
              <ul className="text-sm text-muted-foreground space-y-1">
                <li>• Total execution time for all requests</li>
                <li>• Individual request duration</li>
                <li>• Response sizes</li>
                <li>• Success/failure counts</li>
                <li>• Average response time</li>
              </ul>
            </div>
          </section>

          {/* CLI Usage */}
          <section>
            <H2 id="cli-usage">CLI Usage</H2>
            <p className="text-muted-foreground mb-6">
              Control parallel execution from the command line.
            </p>

            <CodeBlockServer language="bash" filename="terminal">
              {cliUsage}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="cli-flags">CLI Flags</H3>
              <div className="border rounded-lg overflow-hidden">
                <table className="w-full">
                  <thead className="bg-muted/50">
                    <tr className="border-b">
                      <th className="text-left p-3 font-medium">Flag</th>
                      <th className="text-left p-3 font-medium">Description</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr className="border-b">
                      <td className="p-3">
                        <code className="text-sm">--parallel</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">
                        Force parallel execution
                      </td>
                    </tr>
                    <tr className="border-b">
                      <td className="p-3">
                        <code className="text-sm">--sequential</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">
                        Force sequential execution
                      </td>
                    </tr>
                    <tr>
                      <td className="p-3">
                        <code className="text-sm">--continue-on-error</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">
                        Continue execution on failures
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </section>

          {/* Best Practices */}
          <section>
            <H2 id="best-practices">Best Practices</H2>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Shield className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Use for Independent Requests</h4>
                    <p className="text-sm text-muted-foreground">
                      Parallel execution works best when requests don't depend on each other. If
                      requests need data from previous responses, use sequential execution.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Clock className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Set Appropriate Timeouts</h4>
                    <p className="text-sm text-muted-foreground">
                      When running many requests in parallel, set reasonable timeouts to prevent
                      hanging requests from blocking completion.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-yellow-500/10 p-2">
                    <Gauge className="h-4 w-4 text-yellow-600 dark:text-yellow-500" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Monitor Resource Usage</h4>
                    <p className="text-sm text-muted-foreground">
                      Running many parallel requests can consume significant system resources. Start
                      with a small number and increase gradually.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-cyan-500/10 p-2">
                    <TrendingUp className="h-4 w-4 text-cyan-600 dark:text-cyan-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Use Metrics for Analysis</h4>
                    <p className="text-sm text-muted-foreground">
                      Enable{' '}
                      <code className="text-xs bg-muted px-1 py-0.5 rounded">
                        {'showMetrics: true'}
                      </code>{' '}
                      to collect timing data for performance analysis.
                    </p>
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
