import { BarChart3, Bug, CheckCircle, FileJson, Filter, Settings, TrendingUp } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3, H4 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';

export const metadata: Metadata = {
  title: 'Output Formats',
  description:
    'Control how curl-runner displays and saves request results with different output formats including JSON, pretty-printed, and raw.',
  keywords: [
    'curl-runner output formats',
    'JSON output format',
    'pretty print output',
    'raw output format',
    'save results to file',
    'CLI output options',
    'verbose output',
    'structured output',
    'machine readable output',
    'terminal formatting',
    'output configuration',
    'result formatting',
  ],
  openGraph: {
    title: 'Output Formats | curl-runner Documentation',
    description:
      'Control how curl-runner displays and saves request results with different output formats including JSON, pretty-printed, and raw formats.',
    url: 'https://www.curl-runner.com/docs/features/output-formats',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Output Formats | curl-runner Documentation',
    description: 'Learn how to control curl-runner output with different formats and save options.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/output-formats',
  },
};

const outputFormats = `# Different output format configurations
global:
  output:
    format: pretty      # Output format (json, pretty, raw)
    prettyLevel: standard  # Pretty format detail level (minimal, standard, detailed)
    verbose: true       # Show detailed information
    showHeaders: true   # Include response headers
    showBody: true      # Include response body (default: true)
    showMetrics: true   # Include performance metrics

requests:
  - name: API Test
    url: https://api.example.com/data
    method: GET`;

const saveToFile = `# Save results to file
global:
  output:
    format: json
    saveToFile: test-results.json  # Save all results to file
    verbose: false                  # Quiet terminal output
  variables:
    API_URL: https://api.example.com

requests:
  - name: Test Suite
    url: \${API_URL}/test
    method: GET`;

const cliOutput = `# Command line output options
# JSON format (machine-readable)
curl-runner tests.yaml --output-format json

# Pretty format with different detail levels
curl-runner tests.yaml --output-format pretty --pretty-level minimal
curl-runner tests.yaml --output-format pretty --pretty-level standard
curl-runner tests.yaml --output-format pretty --pretty-level detailed

# Raw format (response body only)
curl-runner tests.yaml --output-format raw

# Save to file
curl-runner tests.yaml --output results.json

# Verbose mode
curl-runner tests.yaml --verbose

# Show specific components
curl-runner tests.yaml --show-headers --show-metrics

# Quiet mode (errors only)
curl-runner tests.yaml --quiet`;

const jsonExample = `{
  "summary": {
    "total": 3,
    "successful": 2,
    "failed": 1,
    "duration": 1234,
    "startTime": "2024-01-15T10:30:00Z",
    "endTime": "2024-01-15T10:30:01Z"
  },
  "results": [
    {
      "request": {
        "name": "Get Users",
        "url": "https://api.example.com/users",
        "method": "GET"
      },
      "success": true,
      "status": 200,
      "headers": {
        "content-type": "application/json",
        "x-api-version": "v1"
      },
      "body": {
        "users": [
          { "id": 1, "name": "John Doe" }
        ]
      },
      "metrics": {
        "duration": 125,
        "size": 256
      }
    },
    {
      "request": {
        "name": "Create User",
        "url": "https://api.example.com/users",
        "method": "POST"
      },
      "success": false,
      "status": 400,
      "error": "Validation failed",
      "metrics": {
        "duration": 89,
        "size": 128
      }
    }
  ]
}`;

const jsonConfig = `# JSON format configuration
global:
  output:
    format: json
    showHeaders: true
    showBody: true
    showMetrics: true

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET`;

const prettyMinimal = `ℹ Found 1 YAML file(s)
ℹ Processing: api-test.yaml

✓ Get User Profile [api-test]
   ├─ GET: https://api.example.com/users/123
   ├─ ✓ Status: 200
   └─ Duration: 125ms | 256.00 B

Summary: 1 request completed successfully`;

const prettyStandard = `ℹ Found 1 YAML file(s)
ℹ Processing: api-test.yaml

Executing 1 request(s) in sequential mode

✓ Get User Profile
   ├─ URL: https://api.example.com/users/123
   ├─ Method: GET
   ├─ Status: 200
   ├─ Duration: 125ms
   └─ Response Body:
      {
        "id": 123,
        "name": "John Doe",
        "email": "john@example.com",
        "active": true
      }


Summary: 1 request completed successfully (125ms)`;

const prettyDetailed = `ℹ Found 1 YAML file(s)
ℹ Processing: api-test.yaml

Executing 1 request(s) in sequential mode

  Command:
    curl -X GET -w "\n__CURL_METRICS_START__%{json}__CURL_METRICS_END__" -L -s -S "https://api.example.com/users/123"
✓ Get User Profile
   ├─ URL: https://api.example.com/users/123
   ├─ Method: GET
   ├─ Status: 200
   ├─ Duration: 125ms
   ├─ Response Body:
   │  {
   │    "id": 123,
   │    "name": "John Doe",
   │    "email": "john@example.com",
   │    "active": true
   │  }
   └─ Metrics:
      ├─ Request Duration: 125ms
      ├─ Response Size: 256.00 B
      ├─ DNS Lookup: 5ms
      ├─ TCP Connection: 10ms
      ├─ TLS Handshake: 15ms
      └─ Time to First Byte: 95ms


Summary: 1 request completed successfully (125ms)`;

const prettyMinimalConfig = `# Pretty format - Minimal level
global:
  output:
    format: pretty
    prettyLevel: minimal  # Compact output

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET`;

const prettyStandardConfig = `# Pretty format - Standard level  
global:
  output:
    format: pretty
    prettyLevel: standard  # Balanced detail
    showBody: true

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET`;

const prettyDetailedConfig = `# Pretty format - Detailed level
global:
  output:
    format: pretty
    prettyLevel: detailed  # Full information
    # Headers and metrics are shown automatically

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET`;

const rawOutput = `{
  "id": 123,
  "name": "John Doe", 
  "email": "john@example.com",
  "active": true
}`;

const rawConfig = `# Raw format configuration
global:
  output:
    format: raw
    showBody: true

request:
  name: Get User Profile
  url: https://api.example.com/users/123
  method: GET`;

export default function OutputFormatsPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Output Formats"
          text="Control how curl-runner displays and saves request results."
        />

        <div className="space-y-8">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground mb-6">
              <code className="font-mono">curl-runner</code> provides flexible output options to
              suit different use cases, from human-readable terminal output to machine-parseable
              JSON for CI/CD pipelines.
            </p>

            <div className="grid gap-4 sm:grid-cols-3">
              <div className="rounded-lg border p-4">
                <Badge className="mb-2">JSON</Badge>
                <h4 className="font-medium mb-1">Structured Data</h4>
                <p className="text-sm text-muted-foreground">
                  Machine-readable format for automation
                </p>
              </div>

              <div className="rounded-lg border p-4">
                <Badge variant="secondary" className="mb-2">
                  Pretty
                </Badge>
                <h4 className="font-medium mb-1">Human Readable</h4>
                <p className="text-sm text-muted-foreground">
                  Formatted output for terminal viewing
                </p>
              </div>

              <div className="rounded-lg border p-4">
                <Badge variant="outline" className="mb-2">
                  Raw
                </Badge>
                <h4 className="font-medium mb-1">Unprocessed</h4>
                <p className="text-sm text-muted-foreground">
                  Raw response body without formatting
                </p>
              </div>
            </div>
          </section>

          {/* Output Configuration */}
          <section>
            <H2 id="output-configuration">Output Configuration</H2>
            <p className="text-muted-foreground mb-6">
              Configure output settings in the global section of your YAML file.
            </p>

            <CodeBlockServer language="yaml" filename="output-config.yaml">
              {outputFormats}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="configuration-options">Configuration Options</H3>
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
                        <code className="text-sm">format</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">string</td>
                      <td className="p-3 text-sm text-muted-foreground">pretty</td>
                      <td className="p-3 text-sm text-muted-foreground">
                        "json", "pretty", or "raw"
                      </td>
                    </tr>
                    <tr className="border-b">
                      <td className="p-3">
                        <code className="text-sm">verbose</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">boolean</td>
                      <td className="p-3 text-sm text-muted-foreground">false</td>
                      <td className="p-3 text-sm text-muted-foreground">Show detailed output</td>
                    </tr>
                    <tr className="border-b">
                      <td className="p-3">
                        <code className="text-sm">showHeaders</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">boolean</td>
                      <td className="p-3 text-sm text-muted-foreground">false</td>
                      <td className="p-3 text-sm text-muted-foreground">
                        Include response headers
                      </td>
                    </tr>
                    <tr className="border-b">
                      <td className="p-3">
                        <code className="text-sm">showBody</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">boolean</td>
                      <td className="p-3 text-sm text-muted-foreground">true</td>
                      <td className="p-3 text-sm text-muted-foreground">Include response body</td>
                    </tr>
                    <tr className="border-b">
                      <td className="p-3">
                        <code className="text-sm">showMetrics</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">boolean</td>
                      <td className="p-3 text-sm text-muted-foreground">false</td>
                      <td className="p-3 text-sm text-muted-foreground">
                        Include performance metrics
                      </td>
                    </tr>
                    <tr className="border-b">
                      <td className="p-3">
                        <code className="text-sm">prettyLevel</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">string</td>
                      <td className="p-3 text-sm text-muted-foreground">minimal</td>
                      <td className="p-3 text-sm text-muted-foreground">
                        "minimal", "standard", or "detailed" (for pretty format)
                      </td>
                    </tr>
                    <tr>
                      <td className="p-3">
                        <code className="text-sm">saveToFile</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">string</td>
                      <td className="p-3 text-sm text-muted-foreground">-</td>
                      <td className="p-3 text-sm text-muted-foreground">
                        File path to save results
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </section>

          {/* Format Types */}
          <section>
            <H2 id="format-types">Format Types</H2>
            <div className="space-y-6">
              <div>
                <H3 id="json-format">JSON Format</H3>
                <p className="text-muted-foreground mb-6">
                  Structured JSON output ideal for parsing and automation.
                </p>

                <h4 className="font-medium mb-3">Configuration</h4>
                <CodeBlockServer language="yaml" filename="json-config.yaml">
                  {jsonConfig}
                </CodeBlockServer>

                <h4 className="font-medium mb-3 mt-6">Output Example</h4>
                <CodeBlockServer language="json" filename="output.json">
                  {jsonExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3 id="pretty-format">Pretty Format</H3>
                <p className="text-muted-foreground mb-6">
                  Human-readable format with colors and tree-structured output for terminal viewing.
                  Three detail levels available: minimal, standard, and detailed.
                </p>

                <div className="space-y-6">
                  {/* Minimal Level */}

                  <H4 id="minimal-format">Minimal Format</H4>
                  <p className="text-sm text-muted-foreground mb-4">
                    Compact output showing only essential information
                  </p>
                  <h4 className="font-medium mb-3">Configuration</h4>
                  <CodeBlockServer language="yaml" filename="pretty-minimal.yaml">
                    {prettyMinimalConfig}
                  </CodeBlockServer>
                  <h4 className="font-medium mb-3 mt-4">Output Example</h4>
                  <CodeBlockServer language="text" filename="terminal">
                    {prettyMinimal}
                  </CodeBlockServer>

                  <Separator className="my-4" />

                  {/* Standard Level */}

                  <H4 id="standard-format">Standard Format</H4>
                  <p className="text-sm text-muted-foreground mb-4">
                    Balanced detail with response body but without headers/metrics (unless
                    specified)
                  </p>
                  <h4 className="font-medium mb-3">Configuration</h4>
                  <CodeBlockServer language="yaml" filename="pretty-standard.yaml">
                    {prettyStandardConfig}
                  </CodeBlockServer>
                  <h4 className="font-medium mb-3 mt-4">Output Example</h4>
                  <CodeBlockServer language="text" filename="terminal">
                    {prettyStandard}
                  </CodeBlockServer>

                  <Separator className="my-4" />

                  {/* Detailed Level */}

                  <H4 id="detailed-format">Detailed Format</H4>
                  <p className="text-sm text-muted-foreground mb-4">
                    Full information including headers, body, and performance metrics
                  </p>
                  <h4 className="font-medium mb-3">Configuration</h4>
                  <CodeBlockServer language="yaml" filename="pretty-detailed.yaml">
                    {prettyDetailedConfig}
                  </CodeBlockServer>
                  <h4 className="font-medium mb-3 mt-4">Output Example</h4>
                  <CodeBlockServer language="text" filename="terminal">
                    {prettyDetailed}
                  </CodeBlockServer>
                </div>
              </div>

              <div>
                <H3 id="raw-format">Raw Format</H3>
                <p className="text-muted-foreground mb-6">
                  Unprocessed response body, useful for binary data or custom formats.
                </p>

                <h4 className="font-medium mb-3">Configuration</h4>
                <CodeBlockServer language="yaml" filename="raw-config.yaml">
                  {rawConfig}
                </CodeBlockServer>

                <h4 className="font-medium mb-3 mt-6">Output Example</h4>
                <CodeBlockServer language="json" filename="response.json">
                  {rawOutput}
                </CodeBlockServer>
              </div>
            </div>
          </section>

          {/* Saving Results */}
          <section>
            <H2 id="saving-results">Saving Results</H2>
            <p className="text-muted-foreground mb-6">
              Save test results to files for later analysis or reporting.
            </p>

            <CodeBlockServer language="yaml" filename="save-to-file.yaml">
              {saveToFile}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-green-500/5 dark:bg-green-500/10 border-green-500/20 p-4">
              <p className="text-sm">
                <strong className="text-green-600 dark:text-green-400">Tip:</strong> Use{' '}
                <code className="text-xs bg-background px-1 py-0.5 rounded">saveToFile</code> with
                JSON format to create test reports that can be processed by other tools or stored
                for historical analysis.
              </p>
            </div>
          </section>

          {/* CLI Options */}
          <section>
            <H2 id="cli-options">CLI Options</H2>
            <p className="text-muted-foreground mb-6">
              Control output format from the command line.
            </p>

            <CodeBlockServer language="bash" filename="terminal">
              {cliOutput}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="cli-flags">Output Flags</H3>
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
                        <code className="text-sm">--output-format</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">
                        Set output format (json/pretty/raw)
                      </td>
                    </tr>
                    <tr className="border-b">
                      <td className="p-3">
                        <code className="text-sm">--pretty-level</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">
                        Set pretty format detail level (minimal/standard/detailed)
                      </td>
                    </tr>
                    <tr className="border-b">
                      <td className="p-3">
                        <code className="text-sm">--output, -o</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">Save results to file</td>
                    </tr>
                    <tr className="border-b">
                      <td className="p-3">
                        <code className="text-sm">--verbose, -v</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">Enable verbose output</td>
                    </tr>
                    <tr className="border-b">
                      <td className="p-3">
                        <code className="text-sm">--quiet, -q</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">
                        Suppress non-error output
                      </td>
                    </tr>
                    <tr className="border-b">
                      <td className="p-3">
                        <code className="text-sm">--show-headers</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">
                        Include response headers
                      </td>
                    </tr>
                    <tr className="border-b">
                      <td className="p-3">
                        <code className="text-sm">--show-body</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">Include response body</td>
                    </tr>
                    <tr>
                      <td className="p-3">
                        <code className="text-sm">--show-metrics</code>
                      </td>
                      <td className="p-3 text-sm text-muted-foreground">
                        Include performance metrics
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </section>

          {/* Use Cases */}
          <section>
            <H2 id="use-cases">Use Cases</H2>
            <div className="grid gap-6 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-6">
                <div className="flex items-start gap-4">
                  <div className="rounded-full bg-blue-500/10 p-3">
                    <Settings className="h-5 w-5 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-3">Continuous Integration</h4>
                    <p className="text-sm text-muted-foreground mb-4">
                      Use JSON format with file output for test reporting in CI pipelines. Perfect
                      for automated testing and build systems.
                    </p>
                    <code className="text-xs bg-muted px-3 py-2 rounded block">
                      {'curl-runner tests.yaml --output-format json -o results.json --quiet'}
                    </code>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-6">
                <div className="flex items-start gap-4">
                  <div className="rounded-full bg-green-500/10 p-3">
                    <Bug className="h-5 w-5 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-3">Local Testing</h4>
                    <p className="text-sm text-muted-foreground mb-4">
                      Use pretty format with verbose output for debugging. Ideal for development and
                      troubleshooting.
                    </p>
                    <code className="text-xs bg-muted px-3 py-2 rounded block">
                      {'curl-runner tests.yaml --output-format pretty -v --show-metrics'}
                    </code>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-6 md:col-span-2">
                <div className="flex items-start gap-4">
                  <div className="rounded-full bg-purple-500/10 p-3">
                    <BarChart3 className="h-5 w-5 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-3">Health Checks & Monitoring</h4>
                    <p className="text-sm text-muted-foreground mb-4">
                      Use raw format for simple status checks and monitoring scripts. Extract
                      specific data without extra formatting.
                    </p>
                    <code className="text-xs bg-muted px-3 py-2 rounded block">
                      {'curl-runner health.yaml --output-format raw --quiet'}
                    </code>
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
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Choose the Right Format</h4>
                    <p className="text-sm text-muted-foreground">
                      Use JSON for automation, pretty for debugging, and raw for specific data
                      formats.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <FileJson className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Save Important Results</h4>
                    <p className="text-sm text-muted-foreground">
                      Always save test results to files in CI/CD environments for audit trails.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Filter className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Use Quiet Mode in Scripts</h4>
                    <p className="text-sm text-muted-foreground">
                      Reduce noise in automated scripts by using quiet mode with file output.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-yellow-500/10 p-2">
                    <TrendingUp className="h-4 w-4 text-yellow-600 dark:text-yellow-500" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Enable Metrics for Performance Testing</h4>
                    <p className="text-sm text-muted-foreground">
                      Include metrics in output when analyzing API performance.
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
