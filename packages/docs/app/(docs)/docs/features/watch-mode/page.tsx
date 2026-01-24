import { AlertCircle, CheckCircle, Clock, Eye, RefreshCw, Terminal, Zap } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'Watch Mode',
  description:
    'Automatically re-run HTTP requests when files change. Learn about watch mode for rapid API development and testing workflows.',
  keywords: [
    'curl-runner watch mode',
    'file watching',
    'auto re-run',
    'development workflow',
    'live reload',
    'API development',
    'request watching',
    'file changes',
    'automatic execution',
    'dev mode',
  ],
  openGraph: {
    title: 'Watch Mode | curl-runner Documentation',
    description:
      'Automatically re-run HTTP requests when files change. Learn about watch mode for rapid API development and testing workflows.',
    url: 'https://www.curl-runner.com/docs/features/watch-mode',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Watch Mode | curl-runner Documentation',
    description: 'Learn how to use watch mode for rapid API development with curl-runner.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/watch-mode',
  },
};

const basicWatchExample = `# Basic watch mode
curl-runner api.yaml --watch

# Short form
curl-runner api.yaml -w

# Watch a directory
curl-runner tests/ -w

# Watch multiple files
curl-runner auth.yaml users.yaml products.yaml -w`;

const watchOptionsExample = `# Custom debounce delay (500ms)
curl-runner api.yaml -w --watch-debounce 500

# Don't clear screen between runs
curl-runner api.yaml -w --no-watch-clear

# Combine with other options
curl-runner tests/ -w -p --max-concurrent 3 -v`;

const yamlConfigExample = `# curl-runner.yaml - Enable watch mode via config
global:
  watch:
    enabled: true
    debounce: 300     # 300ms debounce delay
    clear: true       # Clear screen between runs
  variables:
    API_URL: https://api.example.com

requests:
  - name: Get Users
    url: \${API_URL}/users
    method: GET
    expect:
      status: 200`;

const envVarsExample = `# Enable watch mode via environment variable
CURL_RUNNER_WATCH=true curl-runner api.yaml

# Customize debounce
CURL_RUNNER_WATCH_DEBOUNCE=500 curl-runner tests/ -w

# Disable screen clearing
CURL_RUNNER_WATCH_CLEAR=false curl-runner api.yaml -w`;

const workflowExample = `# Development workflow example
# Terminal 1: Start watch mode
curl-runner api-tests.yaml -w -v

# Terminal 2: Edit your YAML file
# Every time you save, requests automatically re-run

# Watch with parallel execution for faster feedback
curl-runner tests/ -w -p --max-concurrent 5

# Watch with specific output format
curl-runner api.yaml -w --output-format pretty --pretty-level detailed`;

const outputExample = `Watching for changes... (press Ctrl+C to stop)
   Files: api.yaml

--------------------------------------------------
[14:32:15] File changed: api.yaml

✓ Get Users [api]
   ├─ GET https://api.example.com/users
   ├─ ✓ Status: 200
   └─ Duration: 45ms | 1.2 KB

Summary: 1 passed | 0 failed | 45ms

Watching for changes...`;

export default function WatchModePage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Watch Mode"
          text="Automatically re-run HTTP requests when YAML files change, providing instant feedback during development."
        />

        <div className="space-y-12">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Watch mode monitors your YAML files for changes and automatically re-executes requests
              when modifications are detected. This creates a tight feedback loop ideal for API
              development and testing.
            </p>

            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Zap className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Instant Feedback</h4>
                    <p className="text-sm text-muted-foreground">
                      See results immediately after saving
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <RefreshCw className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Debounced Execution</h4>
                    <p className="text-sm text-muted-foreground">
                      Smart debouncing prevents duplicate runs
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
                    <h4 className="font-medium mb-1">Clean Output</h4>
                    <p className="text-sm text-muted-foreground">
                      Automatic screen clearing for readability
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
              Enable watch mode with the <code className="font-mono">-w</code> or{' '}
              <code className="font-mono">--watch</code> flag.
            </p>

            <CodeBlockServer language="bash" filename="Basic Watch Mode">
              {basicWatchExample}
            </CodeBlockServer>
          </section>

          {/* Watch Options */}
          <section>
            <H2 id="watch-options">Watch Options</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Customize watch behavior with additional options.
            </p>

            <div className="grid gap-4 mb-6">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Clock className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --watch-debounce &lt;ms&gt;
                      </code>
                      <Badge variant="secondary" className="text-xs">
                        default: 300
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Set the debounce delay in milliseconds. This prevents multiple rapid re-runs
                      when files are saved in quick succession.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Eye className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --no-watch-clear
                      </code>
                      <Badge variant="secondary" className="text-xs">
                        default: clears screen
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Disable automatic screen clearing between runs. Useful when you want to see
                      the history of all runs.
                    </p>
                  </div>
                </div>
              </div>
            </div>

            <CodeBlockServer language="bash" filename="Watch Options">
              {watchOptionsExample}
            </CodeBlockServer>
          </section>

          {/* YAML Configuration */}
          <section>
            <H2 id="yaml-configuration">YAML Configuration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Configure watch mode in your <code className="font-mono">curl-runner.yaml</code>{' '}
              configuration file.
            </p>

            <CodeBlockServer language="yaml" filename="curl-runner.yaml">
              {yamlConfigExample}
            </CodeBlockServer>
          </section>

          {/* Environment Variables */}
          <section>
            <H2 id="environment-variables">Environment Variables</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Control watch mode via environment variables for CI/CD or scripted workflows.
            </p>

            <div className="grid gap-2 mb-6 text-sm">
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_WATCH</code>
                <span className="text-muted-foreground">Enable watch mode (true/false)</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_WATCH_DEBOUNCE</code>
                <span className="text-muted-foreground">Debounce delay in milliseconds</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_WATCH_CLEAR</code>
                <span className="text-muted-foreground">
                  Clear screen between runs (true/false)
                </span>
              </div>
            </div>

            <CodeBlockServer language="bash" filename="Environment Variables">
              {envVarsExample}
            </CodeBlockServer>
          </section>

          {/* Development Workflow */}
          <section>
            <H2 id="development-workflow">Development Workflow</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Watch mode integrates seamlessly with your development workflow.
            </p>

            <CodeBlockServer language="bash" filename="Development Workflow">
              {workflowExample}
            </CodeBlockServer>
          </section>

          {/* Output Example */}
          <section>
            <H2 id="output-example">Output Example</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Here's what watch mode output looks like when a file changes.
            </p>

            <CodeBlockServer language="text" filename="Watch Mode Output">
              {outputExample}
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
                        • Use <code className="font-mono">-v</code> for detailed feedback during
                        development
                      </div>
                      <div>
                        • Combine with <code className="font-mono">-p</code> for faster test suites
                      </div>
                      <div>• Keep YAML files focused on related requests</div>
                      <div>• Use appropriate debounce for your editor's save behavior</div>
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
                    <h4 className="font-medium mb-2">Considerations</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <div>• Watch mode keeps the process running until Ctrl+C</div>
                      <div>• Not recommended for CI/CD pipelines</div>
                      <div>• Changes during execution are queued</div>
                      <div>• Only watches explicitly specified files</div>
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
