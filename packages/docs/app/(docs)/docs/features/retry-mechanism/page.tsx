import {
  AlertTriangle,
  CheckCircle,
  Clock,
  Server,
  Timer,
  TrendingUp,
  Wifi,
  XCircle,
} from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'Retry Mechanism',
  description:
    'Implement robust retry logic for HTTP requests with curl-runner. Learn retry strategies, backoff patterns, and transient failure handling.',
  keywords: [
    'curl-runner retry mechanism',
    'HTTP request retries',
    'retry logic',
    'request failure handling',
    'retry strategies',
    'backoff patterns',
    'transient failure recovery',
    'resilient HTTP requests',
    'retry configuration',
    'automatic retries',
    'error recovery',
    'request reliability',
  ],
  openGraph: {
    title: 'Retry Mechanism | curl-runner Documentation',
    description:
      'Implement robust retry logic for HTTP requests with curl-runner. Learn about retry strategies, backoff patterns, and handling transient failures.',
    url: 'https://www.curl-runner.com/docs/features/retry-mechanism',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Retry Mechanism | curl-runner Documentation',
    description: 'Learn how to implement robust retry logic for HTTP requests with curl-runner.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/retry-mechanism',
  },
};

const basicRetry = `# Basic retry configuration
request:
  name: Flaky API Endpoint
  url: https://api.example.com/unstable
  method: GET
  retry:
    count: 3      # Retry up to 3 times
    delay: 1000   # Wait 1 second between retries`;

const advancedRetry = `# Advanced retry scenarios
requests:
  - name: Critical API Call
    url: https://api.example.com/important
    method: POST
    headers:
      Content-Type: application/json
    body:
      data: "important"
    retry:
      count: 5      # More retries for critical requests
      delay: 2000   # 2 second delay
    timeout: 10000  # 10 second timeout per attempt
    
  - name: Fast Retry
    url: https://api.example.com/quick
    method: GET
    retry:
      count: 10     # Many quick retries
      delay: 100    # Very short delay
    timeout: 1000   # Short timeout`;

const globalRetry = `# Global retry configuration
global:
  defaults:
    retry:
      count: 2      # Default retry count for all requests
      delay: 500    # Default delay between retries
  variables:
    API_URL: https://api.example.com

requests:
  - name: Uses Global Retry
    url: \${API_URL}/endpoint1
    method: GET
    # Will use global retry settings
    
  - name: Override Retry
    url: \${API_URL}/endpoint2
    method: GET
    retry:
      count: 5      # Override global setting
      delay: 1000   # Override global delay
      
  - name: No Retry
    url: \${API_URL}/stable
    method: GET
    retry:
      count: 0      # Disable retries for this request`;

const retryWithValidation = `# Retry with validation rules
request:
  name: Eventually Consistent API
  url: https://api.example.com/resource
  method: GET
  retry:
    count: 5
    delay: 3000   # Wait 3 seconds between attempts
  expect:
    status: 200
    body:
      status: "ready"  # Keep retrying until status is "ready"
      
# Retry on specific status codes
requests:
  - name: Handle Rate Limiting
    url: https://api.example.com/rate-limited
    method: GET
    retry:
      count: 3
      delay: 5000   # Back off for 5 seconds
    expect:
      status: [200, 201]  # Retry if not 200 or 201`;

const exponentialBackoff = `# Exponential backoff with backoff multiplier
request:
  name: API with Exponential Backoff
  url: https://api.example.com/endpoint
  method: GET
  retry:
    count: 4        # Retry up to 4 times
    delay: 1000     # Initial delay: 1 second
    backoff: 2      # Double the delay each retry

# Retry delays will be:
# Attempt 1: 1000ms (1s)
# Attempt 2: 2000ms (2s)
# Attempt 3: 4000ms (4s)
# Attempt 4: 8000ms (8s)

---

# Gentler backoff with 1.5x multiplier
request:
  name: Gentle Backoff
  url: https://api.example.com/rate-limited
  method: GET
  retry:
    count: 5
    delay: 1000
    backoff: 1.5    # 1.5x multiplier

# Retry delays: 1000ms, 1500ms, 2250ms, 3375ms, 5063ms`;

const cliCommands = `# Override retry count globally
curl-runner api-tests.yaml --retries 5

# Disable all retries
curl-runner api-tests.yaml --no-retry

# Set retry delay
curl-runner api-tests.yaml --retries 3 --retry-delay 2000`;

const fixedDelayExample = `# Fixed delay retry strategy
request:
  name: Reliable API Call
  url: https://api.example.com/data
  method: GET
  retry:
    count: 3      # Retry up to 3 times
    delay: 1000   # Wait exactly 1 second between retries
    
# Will attempt sequence:
# 1. Initial request
# 2. Wait 1s → Retry attempt 1  
# 3. Wait 1s → Retry attempt 2
# 4. Wait 1s → Retry attempt 3`;

export default function RetryMechanismPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Retry Mechanism"
          text="Automatically retry failed requests with configurable delays and attempts."
        />

        <div className="space-y-8">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground mb-6">
              The retry mechanism in <code className="font-mono">curl-runner</code> allows you to
              automatically retry failed requests, making your API tests more resilient to temporary
              failures, network issues, and rate limiting.
            </p>

            <div className="grid gap-4 sm:grid-cols-2">
              <div className="rounded-lg border p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Badge className="bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20">
                    Automatic
                  </Badge>
                </div>
                <h4 className="font-medium mb-1">Smart Retries</h4>
                <p className="text-sm text-muted-foreground">
                  Automatically retry on network errors, timeouts, and 5xx status codes
                </p>
              </div>

              <div className="rounded-lg border p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Badge className="bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20">
                    Configurable
                  </Badge>
                </div>
                <h4 className="font-medium mb-1">Flexible Settings</h4>
                <p className="text-sm text-muted-foreground">
                  Control retry count, delays, and conditions per request or globally
                </p>
              </div>
            </div>
          </section>

          {/* Basic Usage */}
          <section>
            <H2 id="basic-usage">Basic Usage</H2>
            <p className="text-muted-foreground mb-6">
              Configure retries using the{' '}
              <code className="text-sm bg-muted px-1 py-0.5 rounded">retry</code> field in your
              request configuration.
            </p>

            <CodeBlockServer language="yaml" filename="basic-retry.yaml">
              {basicRetry}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-muted/50 p-4">
              <h4 className="text-sm font-medium mb-2">How it works:</h4>
              <ol className="text-sm text-muted-foreground space-y-1 list-decimal list-inside">
                <li>First attempt is made immediately</li>
                <li>If it fails, wait for the specified delay</li>
                <li>Retry up to the specified count</li>
                <li>Stop on success or after all retries exhausted</li>
              </ol>
            </div>
          </section>

          {/* Configuration Options */}
          <section>
            <H2 id="configuration">Configuration Options</H2>
            <p className="text-muted-foreground mb-6">
              Fine-tune retry behavior with these configuration options.
            </p>

            <div className="border rounded-lg overflow-hidden">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Option</th>
                    <th className="text-left p-3 font-medium">Type</th>
                    <th className="text-left p-3 font-medium">Default</th>
                    <th className="text-left p-3 font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">count</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">number</td>
                    <td className="p-3 text-sm text-muted-foreground">0</td>
                    <td className="p-3 text-sm text-muted-foreground">Number of retry attempts</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">delay</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">number</td>
                    <td className="p-3 text-sm text-muted-foreground">1000</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Initial delay between retries (ms)
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">backoff</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">number</td>
                    <td className="p-3 text-sm text-muted-foreground">1</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Exponential backoff multiplier. Delay increases as: delay ×
                      backoff^(attempt-1)
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Advanced Scenarios */}
          <section>
            <H2 id="advanced-scenarios">Advanced Scenarios</H2>
            <p className="text-muted-foreground mb-6">
              Handle complex retry requirements with different configurations per request.
            </p>

            <CodeBlockServer language="yaml" filename="advanced-retry.yaml">
              {advancedRetry}
            </CodeBlockServer>
          </section>

          {/* Global Configuration */}
          <section>
            <H2 id="global-configuration">Global Configuration</H2>
            <p className="text-muted-foreground mb-6">
              Set default retry behavior for all requests and override as needed.
            </p>

            <CodeBlockServer language="yaml" filename="global-retry.yaml">
              {globalRetry}
            </CodeBlockServer>
          </section>

          {/* Retry with Validation */}
          <section>
            <H2 id="retry-with-validation">Retry with Validation</H2>
            <p className="text-muted-foreground mb-6">
              Combine retries with validation rules to handle eventually consistent APIs.
            </p>

            <CodeBlockServer language="yaml" filename="retry-validation.yaml">
              {retryWithValidation}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-yellow-500/5 dark:bg-yellow-500/10 border-yellow-500/20 p-4">
              <p className="text-sm">
                <strong className="text-yellow-600 dark:text-yellow-400">Important:</strong> Retries
                occur when the request fails (network error, timeout) or when validation rules are
                not met. This is useful for polling APIs that may take time to update.
              </p>
            </div>
          </section>

          {/* Retry Strategies */}
          <section>
            <H2 id="retry-strategies">Retry Strategies</H2>
            <div className="space-y-6">
              <div>
                <H3 id="fixed-delay">Fixed Delay</H3>
                <p className="text-muted-foreground mb-4">
                  Use a consistent delay between all retry attempts.
                </p>
                <CodeBlockServer language="yaml" filename="fixed-delay.yaml">
                  {fixedDelayExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3 id="exponential-backoff">Exponential Backoff</H3>
                <p className="text-muted-foreground mb-4">
                  Use the <code className="text-sm bg-muted px-1 py-0.5 rounded">backoff</code>{' '}
                  multiplier to increase delays exponentially between retries. This is ideal for
                  rate-limited APIs and helps reduce server load during outages.
                </p>
                <CodeBlockServer language="yaml" filename="exponential-backoff.yaml">
                  {exponentialBackoff}
                </CodeBlockServer>
              </div>
            </div>
          </section>

          {/* When to Retry */}
          <section>
            <H2 id="when-to-retry">When to Retry</H2>
            <div className="space-y-3">
              <H3 id="automatic-retry-conditions">Automatic Retry Conditions</H3>
              <p className="text-muted-foreground mb-6">
                <code className="font-mono">curl-runner</code> automatically retries on these
                conditions:
              </p>

              <div className="grid gap-4 md:grid-cols-2">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-red-500/10 p-2">
                      <Wifi className="h-4 w-4 text-red-600 dark:text-red-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Network Errors</h4>
                      <p className="text-sm text-muted-foreground">
                        Connection refused, DNS failures, and network connectivity issues
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-orange-500/10 p-2">
                      <Timer className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Timeouts</h4>
                      <p className="text-sm text-muted-foreground">
                        Request exceeds the configured timeout limit
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-purple-500/10 p-2">
                      <Server className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">5xx Errors</h4>
                      <p className="text-sm text-muted-foreground">
                        Server errors like 500, 502, 503, and 504 status codes
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-yellow-500/10 p-2">
                      <XCircle className="h-4 w-4 text-yellow-600 dark:text-yellow-500" />
                    </div>
                    <div>
                      <h4 className="font-medium mb-2">Validation Failures</h4>
                      <p className="text-sm text-muted-foreground">
                        Response doesn't meet the configured validation expectations
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
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
                  <div>
                    <h4 className="font-medium mb-2">Set Reasonable Limits</h4>
                    <p className="text-sm text-muted-foreground">
                      Don't retry indefinitely. Set a reasonable count (3-5) for most scenarios.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Clock className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Use Appropriate Delays</h4>
                    <p className="text-sm text-muted-foreground">
                      Avoid overwhelming servers with rapid retries. Use at least 1-second delays
                      for production APIs.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <TrendingUp className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Consider Exponential Backoff</h4>
                    <p className="text-sm text-muted-foreground">
                      For rate-limited APIs, increase delay between retries to avoid hitting limits
                      repeatedly.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-yellow-500/10 p-2">
                    <AlertTriangle className="h-4 w-4 text-yellow-600 dark:text-yellow-500" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Monitor Total Time</h4>
                    <p className="text-sm text-muted-foreground">
                      With retries, requests can take much longer. Set appropriate timeouts to avoid
                      hanging tests.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* CLI Options */}
          <section>
            <H2 id="cli-options">CLI Options</H2>
            <p className="text-muted-foreground mb-6">
              Control retry behavior from the command line.
            </p>

            <CodeBlockServer language="bash" filename="terminal">
              {cliCommands}
            </CodeBlockServer>
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
