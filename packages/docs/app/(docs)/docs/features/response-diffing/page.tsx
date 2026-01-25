import {
  AlertCircle,
  CheckCircle,
  Code,
  FileJson,
  GitCompare,
  Layers,
  Search,
  Terminal,
} from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'Response Diffing',
  description:
    'Compare API responses between environments or runs. Detect API drift and breaking changes.',
  keywords: [
    'curl-runner diffing',
    'API comparison',
    'response comparison',
    'environment diff',
    'API drift detection',
    'API testing',
    'staging vs production',
    'API monitoring',
    'breaking changes',
    'contract testing',
  ],
  openGraph: {
    title: 'Response Diffing | curl-runner Documentation',
    description:
      'Compare API responses between environments or runs. Detect API drift and breaking changes.',
    url: 'https://www.curl-runner.com/docs/features/response-diffing',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Response Diffing | curl-runner Documentation',
    description: 'Learn how to compare API responses between environments with curl-runner.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/response-diffing',
  },
};

const basicExample = `# Save current run as baseline
curl-runner api.yaml --diff-save --diff-label staging

# Compare against staging baseline
curl-runner api.yaml --diff --diff-compare staging

# Auto mode: creates baseline on first run, compares on subsequent
curl-runner api.yaml --diff`;

const subcommandExample = `# Compare two stored baselines (offline, no requests)
curl-runner diff staging production api.yaml

# Compare with JSON output for CI
curl-runner diff staging production api.yaml --diff-output json`;

const basicYamlExample = `# Simple diffing - compare against baseline
global:
  diff:
    exclude:
      - "body.timestamp"
      - "headers.date"

request:
  name: Get User
  url: https://api.example.com/users/1
  method: GET`;

const advancedYamlExample = `# Advanced diff configuration
global:
  diff:
    enabled: true
    dir: "__baselines__"
    exclude:
      - "*.timestamp"
      - "*.Date"
      - "body.headers.X-Request-Id"
    match:
      "body.origin": "*"  # IP can change
    includeTimings: true  # Track response time changes

requests:
  - name: Get Users
    url: https://api.example.com/users
    method: GET
    diff:
      exclude:
        - "body[*].lastLogin"

  - name: Create User
    url: https://api.example.com/users
    method: POST
    body:
      name: Test User
    diff:
      match:
        "body.id": "*"  # New ID each time`;

const workflowExample = `# 1. Save staging baseline
curl-runner api.yaml --diff-save --diff-label staging

# 2. Save production baseline
curl-runner api.yaml --diff-save --diff-label production

# 3. Compare environments (offline)
curl-runner diff staging production api.yaml

# 4. Compare new deployment against baseline
curl-runner api.yaml --diff --diff-compare staging --diff-label v2.0`;

const ciExample = `# CI Pipeline - detect breaking changes
curl-runner api.yaml --diff --diff-compare baseline --diff-output json

# Environment variables
CURL_RUNNER_DIFF=true
CURL_RUNNER_DIFF_COMPARE=baseline
CURL_RUNNER_DIFF_OUTPUT=json
CURL_RUNNER_DIFF_DIR=__baselines__`;

const baselineFileExample = `// __baselines__/api.staging.baseline.json
{
  "version": 1,
  "label": "staging",
  "capturedAt": "2024-01-15T10:30:00Z",
  "baselines": {
    "Get User": {
      "status": 200,
      "body": {
        "id": 1,
        "name": "John Doe",
        "email": "john@example.com"
      },
      "headers": {
        "content-type": "application/json"
      },
      "timing": 150,
      "hash": "a1b2c3d4",
      "capturedAt": "2024-01-15T10:30:00Z"
    }
  }
}`;

const outputExample = `Response Diff: staging → production

✗ GET /users
  body.total:
    - 150
    + 152
  body.users[0].email:
    - "john@example.com"
    + "john.doe@example.com"
  timing: 150ms → 280ms (+87%)

✓ GET /config (no changes)

✗ POST /auth
  status:
    - 200
    + 201

Summary: 1 unchanged, 2 changed (3 total)`;

const markdownOutputExample = `# Response Diff: staging → production

| Metric | Count |
|--------|-------|
| Total Requests | 3 |
| Unchanged | 1 |
| Changed | 2 |
| New Baselines | 0 |

## Changes

### \`GET /users\`

\`\`\`diff
# body.total:
- 150
+ 152
\`\`\``;

export default function ResponseDiffingPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Response Diffing"
          text="Compare API responses between environments or runs. Detect API drift, breaking changes, and unexpected behavior before they reach production."
        />

        <div className="space-y-12">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Response diffing captures API responses as baselines and compares them across
              environments (staging vs production) or over time. Perfect for detecting API drift and
              validating deployments.
            </p>

            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Layers className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Environment Comparison</h4>
                    <p className="text-sm text-muted-foreground">
                      Compare staging vs production responses
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Search className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Drift Detection</h4>
                    <p className="text-sm text-muted-foreground">
                      Detect unexpected API changes over time
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <GitCompare className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Deployment Validation</h4>
                    <p className="text-sm text-muted-foreground">
                      Verify APIs before and after deployments
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
              Save responses as baselines and compare later runs against them.
            </p>

            <CodeBlockServer language="bash" filename="CLI Usage">
              {basicExample}
            </CodeBlockServer>

            <p className="text-muted-foreground mt-6 mb-4">Configure diffing in your YAML file:</p>

            <CodeBlockServer language="yaml" filename="Basic Diff Config">
              {basicYamlExample}
            </CodeBlockServer>
          </section>

          {/* Diff Subcommand */}
          <section>
            <H2 id="subcommand">Diff Subcommand</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Compare two stored baselines without making new requests.
            </p>

            <CodeBlockServer language="bash" filename="Offline Comparison">
              {subcommandExample}
            </CodeBlockServer>
          </section>

          {/* CLI Options */}
          <section>
            <H2 id="cli-options">CLI Options</H2>

            <div className="grid gap-4 mb-6">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <GitCompare className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        -d, --diff
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Enable diff mode. Creates baseline on first run, compares on subsequent runs.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <FileJson className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --diff-save
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Save current run as a baseline. Use with --diff-label.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-yellow-500/10 p-2">
                    <Code className="h-4 w-4 text-yellow-600 dark:text-yellow-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --diff-label &lt;name&gt;
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Label for current run (e.g., 'staging', 'production', 'v1.0').
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Search className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --diff-compare &lt;label&gt;
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Compare current run against this baseline label.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <FileJson className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --diff-dir &lt;dir&gt;
                      </code>
                      <Badge variant="secondary" className="text-xs">
                        default: __baselines__
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Directory to store baseline files.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-cyan-500/10 p-2">
                    <Terminal className="h-4 w-4 text-cyan-600 dark:text-cyan-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --diff-output &lt;format&gt;
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Output format: <code>terminal</code> (default), <code>json</code>, or{' '}
                      <code>markdown</code>.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Configuration */}
          <section>
            <H2 id="configuration">Configuration Options</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Configure exclusions, match rules, and timing tracking.
            </p>

            <CodeBlockServer language="yaml" filename="Advanced Configuration">
              {advancedYamlExample}
            </CodeBlockServer>

            <div className="mt-6 grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <Code className="h-4 w-4" />
                  exclude
                </h4>
                <p className="text-sm text-muted-foreground mb-2">
                  Paths to ignore during comparison:
                </p>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>
                    • <code className="font-mono">body.timestamp</code> - Exact path
                  </li>
                  <li>
                    • <code className="font-mono">*.createdAt</code> - Wildcard
                  </li>
                  <li>
                    • <code className="font-mono">body[*].id</code> - Array wildcard
                  </li>
                </ul>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <Code className="h-4 w-4" />
                  match
                </h4>
                <p className="text-sm text-muted-foreground mb-2">Accept dynamic values:</p>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>
                    • <code className="font-mono">"body.id": "*"</code> - Any value
                  </li>
                  <li>
                    •{' '}
                    <code className="font-mono">
                      "body.uuid": "regex:^[a-f0-9-]&#123;36&#125;$"
                    </code>
                  </li>
                </ul>
              </div>
            </div>
          </section>

          {/* Workflow */}
          <section>
            <H2 id="workflow">Typical Workflow</H2>
            <p className="text-muted-foreground text-lg mb-6">
              A common workflow for comparing staging and production environments.
            </p>

            <CodeBlockServer language="bash" filename="Environment Comparison Workflow">
              {workflowExample}
            </CodeBlockServer>
          </section>

          {/* Baseline Files */}
          <section>
            <H2 id="baseline-files">Baseline File Format</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Baselines are stored as JSON files in the{' '}
              <code className="font-mono">__baselines__</code> directory.
            </p>

            <CodeBlockServer language="json" filename="__baselines__/api.staging.baseline.json">
              {baselineFileExample}
            </CodeBlockServer>
          </section>

          {/* CI/CD Integration */}
          <section>
            <H2 id="ci-integration">CI/CD Integration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Use response diffing in CI/CD pipelines to detect breaking changes.
            </p>

            <CodeBlockServer language="bash" filename="CI Configuration">
              {ciExample}
            </CodeBlockServer>

            <div className="mt-4 p-4 bg-muted/50 rounded-lg">
              <p className="text-sm text-muted-foreground">
                <strong>Exit Codes:</strong> Returns exit code 1 if differences are found, making it
                easy to fail CI builds on unexpected changes.
              </p>
            </div>
          </section>

          {/* Output Examples */}
          <section>
            <H2 id="output">Output Formats</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Diff results in terminal, JSON, or markdown format.
            </p>

            <h3 className="font-medium mb-4">Terminal Output</h3>
            <CodeBlockServer language="text" filename="Terminal Output">
              {outputExample}
            </CodeBlockServer>

            <h3 className="font-medium mt-8 mb-4">Markdown Output</h3>
            <CodeBlockServer language="markdown" filename="Markdown Output">
              {markdownOutputExample}
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
                      <div>• Exclude timestamps and request IDs</div>
                      <div>• Use descriptive labels (staging, prod, v1.0)</div>
                      <div>• Commit baselines to version control</div>
                      <div>• Use JSON output for CI pipelines</div>
                      <div>• Compare before deploying to production</div>
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
                      <div>• Exclude all non-deterministic fields</div>
                      <div>• Update baselines when changes are intentional</div>
                      <div>• Don't compare across different API versions</div>
                      <div>• Keep baselines up to date</div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Environment Variables */}
          <section>
            <H2 id="environment-variables">Environment Variables</H2>

            <div className="grid gap-2 text-sm">
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_DIFF</code>
                <span className="text-muted-foreground">Enable diffing (true/false)</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_DIFF_SAVE</code>
                <span className="text-muted-foreground">Save as baseline (true/false)</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_DIFF_LABEL</code>
                <span className="text-muted-foreground">Label for current run</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_DIFF_COMPARE</code>
                <span className="text-muted-foreground">Baseline to compare against</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_DIFF_DIR</code>
                <span className="text-muted-foreground">Baseline directory</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_DIFF_OUTPUT</code>
                <span className="text-muted-foreground">
                  Output format (terminal/json/markdown)
                </span>
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
