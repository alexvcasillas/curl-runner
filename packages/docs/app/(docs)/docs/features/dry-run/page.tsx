import { AlertCircle, CheckCircle, Code, Eye, FileCode, Terminal } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'Dry Run Mode',
  description:
    'Preview curl commands without executing them. Validate request configurations and debug issues before making actual API calls.',
  keywords: [
    'curl-runner dry run',
    'preview curl commands',
    'debug requests',
    'validate configuration',
    'command preview',
    'test curl commands',
    'dry-run mode',
    'curl preview',
    'request debugging',
    'API testing',
  ],
  openGraph: {
    title: 'Dry Run Mode | curl-runner Documentation',
    description:
      'Preview curl commands without executing them. Validate request configurations and debug issues before making actual API calls.',
    url: 'https://www.curl-runner.com/docs/features/dry-run',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Dry Run Mode | curl-runner Documentation',
    description: 'Preview curl commands without executing them with curl-runner.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/dry-run',
  },
};

const basicExample = `# Basic dry run
curl-runner api.yaml --dry-run

# Short form
curl-runner api.yaml -n

# Dry run a directory of tests
curl-runner tests/ --dry-run

# Dry run with verbose output
curl-runner api.yaml -n -v`;

const yamlExample = `# api.yaml
request:
  name: Create User
  url: https://api.example.com/users
  method: POST
  headers:
    Authorization: Bearer \${API_TOKEN}
    Content-Type: application/json
  body:
    name: John Doe
    email: john@example.com`;

const outputExample = `$ curl-runner api.yaml --dry-run

  Command:
    curl -X POST -w '__CURL_METRICS_START__%{json}__CURL_METRICS_END__' \\
      -H 'Authorization: Bearer token123' \\
      -H 'Content-Type: application/json' \\
      -d '{"name":"John Doe","email":"john@example.com"}' \\
      -L -s -S https://api.example.com/users

✓ Create User [api]
   ├─ POST: https://api.example.com/users
   ├─ ✓ Status: DRY-RUN
   └─ Duration: 0ms | 0 B

Summary: 1 request completed successfully`;

const jsonOutputExample = `$ curl-runner api.yaml --dry-run --output-format json

{
  "request": {
    "name": "Create User",
    "url": "https://api.example.com/users",
    "method": "POST"
  },
  "success": true,
  "dryRun": true
}`;

const envVarsExample = `# Enable dry run via environment variable
CURL_RUNNER_DRY_RUN=true curl-runner api.yaml

# Useful in scripts for conditional dry runs
if [ "$DEBUG" = "true" ]; then
  curl-runner api.yaml --dry-run
else
  curl-runner api.yaml
fi`;

const useCasesExample = `# Verify configuration before deployment
curl-runner production.yaml --dry-run

# Debug authentication headers
curl-runner auth-test.yaml -n -v

# Validate variable interpolation
curl-runner \${ENV}.yaml --dry-run

# Review generated commands for complex requests
curl-runner file-upload.yaml --dry-run`;

export default function DryRunPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Dry Run Mode"
          text="Preview the exact curl commands that would be executed without making actual API calls."
        />

        <div className="space-y-12">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Dry run mode shows you exactly what curl commands curl-runner would execute, without
              actually making the HTTP requests. This is invaluable for debugging configurations,
              validating variable interpolation, and reviewing complex request setups.
            </p>

            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Eye className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Preview Commands</h4>
                    <p className="text-sm text-muted-foreground">
                      See the exact curl command before execution
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Code className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Validate Config</h4>
                    <p className="text-sm text-muted-foreground">
                      Verify headers, body, and variable interpolation
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Terminal className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Safe Testing</h4>
                    <p className="text-sm text-muted-foreground">
                      No API calls made, no side effects
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
              Enable dry run mode with the <code className="font-mono">-n</code> or{' '}
              <code className="font-mono">--dry-run</code> flag.
            </p>

            <CodeBlockServer language="bash" filename="Basic Dry Run">
              {basicExample}
            </CodeBlockServer>
          </section>

          {/* Example YAML */}
          <section>
            <H2 id="example">Example</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Given this YAML configuration, dry run mode shows the generated curl command.
            </p>

            <div className="space-y-4">
              <CodeBlockServer language="yaml" filename="api.yaml">
                {yamlExample}
              </CodeBlockServer>

              <CodeBlockServer language="text" filename="Dry Run Output">
                {outputExample}
              </CodeBlockServer>
            </div>
          </section>

          {/* Output Formats */}
          <section>
            <H2 id="output-formats">Output Formats</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Dry run works with all output formats. The{' '}
              <code className="font-mono">dryRun: true</code> field indicates no request was made.
            </p>

            <div className="grid gap-4 mb-6">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <FileCode className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --output-format pretty
                      </code>
                      <Badge variant="secondary" className="text-xs">
                        default
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Shows Status: <code className="font-mono text-cyan-600">DRY-RUN</code> in cyan
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <FileCode className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --output-format json
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Includes <code className="font-mono">"dryRun": true</code> in the response
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <FileCode className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --output-format raw
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Shows only the curl command (no response body)
                    </p>
                  </div>
                </div>
              </div>
            </div>

            <CodeBlockServer language="json" filename="JSON Output">
              {jsonOutputExample}
            </CodeBlockServer>
          </section>

          {/* Environment Variables */}
          <section>
            <H2 id="environment-variables">Environment Variables</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Enable dry run mode via environment variable for scripted workflows.
            </p>

            <div className="grid gap-2 mb-6 text-sm">
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_DRY_RUN</code>
                <span className="text-muted-foreground">Enable dry run mode (true/false)</span>
              </div>
            </div>

            <CodeBlockServer language="bash" filename="Environment Variables">
              {envVarsExample}
            </CodeBlockServer>
          </section>

          {/* Use Cases */}
          <section>
            <H2 id="use-cases">Use Cases</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Common scenarios where dry run mode is helpful.
            </p>

            <CodeBlockServer language="bash" filename="Common Use Cases">
              {useCasesExample}
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
                      <div>
                        • Use <code className="font-mono">-n</code> before running against
                        production APIs
                      </div>
                      <div>• Verify authentication headers are correctly interpolated</div>
                      <div>• Check request body structure for complex payloads</div>
                      <div>• Validate file upload configurations</div>
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
                      <div>• Response storage won't have values from dry runs</div>
                      <div>• Conditional execution evaluates but doesn't execute</div>
                      <div>• Snapshot/diff modes show DRY-RUN status</div>
                      <div>• All requests show success (no actual validation)</div>
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
