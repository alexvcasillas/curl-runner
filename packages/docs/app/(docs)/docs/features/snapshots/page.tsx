import {
  AlertCircle,
  Camera,
  CheckCircle,
  Code,
  FileJson,
  GitCompare,
  RefreshCw,
  Shield,
} from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'Snapshot Testing',
  description:
    'Save HTTP response snapshots and compare future runs against them. Perfect for regression testing APIs.',
  keywords: [
    'curl-runner snapshots',
    'API snapshot testing',
    'response comparison',
    'regression testing',
    'API testing',
    'snapshot testing',
    'golden files',
    'response validation',
    'API monitoring',
    'jest snapshots',
  ],
  openGraph: {
    title: 'Snapshot Testing | curl-runner Documentation',
    description:
      'Save HTTP response snapshots and compare future runs against them. Perfect for regression testing APIs.',
    url: 'https://www.curl-runner.com/docs/features/snapshots',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Snapshot Testing | curl-runner Documentation',
    description: 'Learn how to use snapshot testing for API regression testing with curl-runner.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/snapshots',
  },
};

const basicExample = `# Enable snapshot testing with CLI flag
curl-runner api.yaml --snapshot

# Short form
curl-runner api.yaml -s

# Update all snapshots
curl-runner api.yaml --snapshot --update-snapshots

# Short form for update
curl-runner api.yaml -su`;

const basicYamlExample = `# Simple snapshot - saves and compares response body
request:
  name: Get User
  url: https://api.example.com/users/1
  method: GET
  snapshot: true`;

const advancedYamlExample = `# Advanced snapshot configuration
request:
  name: Get User Details
  url: https://api.example.com/users/1
  method: GET
  snapshot:
    include:
      - status     # Include HTTP status code
      - headers    # Include response headers
      - body       # Include response body
    exclude:
      - body.timestamp        # Ignore this field
      - body.lastLogin        # Ignore this field
      - headers.date          # Ignore Date header
      - headers.x-request-id  # Ignore request ID
    match:
      body.id: "*"                        # Accept any value
      body.version: "regex:^v\\\\d+\\\\.\\\\d+"  # Match pattern`;

const globalConfigExample = `# curl-runner.yaml - Global snapshot config
global:
  snapshot:
    dir: "__snapshots__"    # Snapshot directory
    updateMode: "none"       # none | all | failing
    include:
      - body
    exclude:
      - "*.createdAt"        # Exclude all createdAt fields
      - "*.updatedAt"        # Exclude all updatedAt fields

requests:
  - name: Get Users
    url: https://api.example.com/users
    snapshot: true  # Uses global config`;

const collectionExample = `# Collection with snapshot testing
global:
  snapshot:
    exclude:
      - "*.timestamp"

collection:
  name: User API Tests
  requests:
    - name: List Users
      url: https://api.example.com/users
      snapshot: true

    - name: Get User by ID
      url: https://api.example.com/users/1
      snapshot:
        include: [status, body]
        match:
          body.id: "*"

    - name: Search Users
      url: https://api.example.com/users?q=john
      snapshot:
        exclude:
          - "body[*].score"  # Exclude search scores`;

const ciExample = `# CI/CD - Fail if snapshots are missing
curl-runner tests/ --snapshot --ci-snapshot --strict-exit

# Environment variables
CURL_RUNNER_SNAPSHOT=true
CURL_RUNNER_SNAPSHOT_CI=true
CURL_RUNNER_SNAPSHOT_DIR=__snapshots__`;

const snapshotFileExample = `// __snapshots__/api.snap.json
{
  "version": 1,
  "snapshots": {
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
      "hash": "a1b2c3d4",
      "updatedAt": "2024-01-15T10:30:00Z"
    }
  }
}`;

const outputExample = `# Snapshot match
  PASS Snapshot matches for "Get User"

# Snapshot mismatch
  FAIL Snapshot mismatch for "Get User"

  - Expected
  + Received

  body.email:
    - "john@example.com"
    + "john.doe@example.com"

  Run with --update-snapshots (-u) to update

# New snapshot created
  NEW Snapshot created for "Get User"

# Snapshot updated
  UPDATED Snapshot updated for "Get User"`;

export default function SnapshotsPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Snapshot Testing"
          text="Save HTTP response snapshots and compare future runs against them. Perfect for regression testing APIs - like Jest snapshots for HTTP requests."
        />

        <div className="space-y-12">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Snapshot testing captures API responses and compares them against stored baselines.
              When responses change unexpectedly, you'll know immediately. This is ideal for
              regression testing and API contract validation.
            </p>

            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Camera className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Capture Responses</h4>
                    <p className="text-sm text-muted-foreground">
                      Save status, headers, and body as snapshots
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <GitCompare className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Detect Changes</h4>
                    <p className="text-sm text-muted-foreground">
                      Automatically compare against stored snapshots
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Shield className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Regression Testing</h4>
                    <p className="text-sm text-muted-foreground">
                      Catch unexpected API changes in CI/CD
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
              Enable snapshot testing with the <code className="font-mono">-s</code> or{' '}
              <code className="font-mono">--snapshot</code> flag.
            </p>

            <CodeBlockServer language="bash" filename="CLI Usage">
              {basicExample}
            </CodeBlockServer>

            <p className="text-muted-foreground mt-6 mb-4">
              Enable snapshots per-request in your YAML file:
            </p>

            <CodeBlockServer language="yaml" filename="Basic Snapshot">
              {basicYamlExample}
            </CodeBlockServer>
          </section>

          {/* Snapshot Options */}
          <section>
            <H2 id="cli-options">CLI Options</H2>

            <div className="grid gap-4 mb-6">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Camera className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        -s, --snapshot
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Enable snapshot testing. Creates new snapshots if they don't exist, compares
                      against existing ones.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-yellow-500/10 p-2">
                    <RefreshCw className="h-4 w-4 text-yellow-600 dark:text-yellow-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        -u, --update-snapshots
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Update all snapshots with current responses. Use when API changes are
                      intentional.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <AlertCircle className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --update-failing
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Only update snapshots that are currently failing. Useful for targeted updates.
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
                        --snapshot-dir &lt;dir&gt;
                      </code>
                      <Badge variant="secondary" className="text-xs">
                        default: __snapshots__
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Custom directory for snapshot files.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-red-500/10 p-2">
                    <Shield className="h-4 w-4 text-red-600 dark:text-red-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --ci-snapshot
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      CI mode: fail if snapshot is missing instead of creating it. Ensures all
                      snapshots are committed.
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
              Configure what to include in snapshots and what to exclude from comparison.
            </p>

            <CodeBlockServer language="yaml" filename="Advanced Configuration">
              {advancedYamlExample}
            </CodeBlockServer>

            <div className="mt-6 grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <Code className="h-4 w-4" />
                  include
                </h4>
                <p className="text-sm text-muted-foreground mb-2">
                  What to capture in the snapshot:
                </p>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>
                    • <code className="font-mono">status</code> - HTTP status code
                  </li>
                  <li>
                    • <code className="font-mono">headers</code> - Response headers
                  </li>
                  <li>
                    • <code className="font-mono">body</code> - Response body (default)
                  </li>
                </ul>
              </div>

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

              <div className="rounded-lg border bg-card p-4 md:col-span-2">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <Code className="h-4 w-4" />
                  match
                </h4>
                <p className="text-sm text-muted-foreground mb-2">
                  Custom matching rules for dynamic values:
                </p>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>
                    • <code className="font-mono">"body.id": "*"</code> - Accept any value
                  </li>
                  <li>
                    • <code className="font-mono">"body.email": "regex:^[\\w.-]+@[\\w.-]+$"</code> -
                    Match pattern
                  </li>
                </ul>
              </div>
            </div>
          </section>

          {/* Global Configuration */}
          <section>
            <H2 id="global-configuration">Global Configuration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Configure defaults in your <code className="font-mono">curl-runner.yaml</code> file.
            </p>

            <CodeBlockServer language="yaml" filename="curl-runner.yaml">
              {globalConfigExample}
            </CodeBlockServer>
          </section>

          {/* Collection Example */}
          <section>
            <H2 id="collections">Collection Testing</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Test multiple endpoints with snapshots in a single collection.
            </p>

            <CodeBlockServer language="yaml" filename="Collection with Snapshots">
              {collectionExample}
            </CodeBlockServer>
          </section>

          {/* Snapshot File Format */}
          <section>
            <H2 id="snapshot-files">Snapshot File Format</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Snapshots are stored as JSON files in the{' '}
              <code className="font-mono">__snapshots__</code> directory.
            </p>

            <CodeBlockServer language="json" filename="__snapshots__/api.snap.json">
              {snapshotFileExample}
            </CodeBlockServer>
          </section>

          {/* CI/CD Integration */}
          <section>
            <H2 id="ci-integration">CI/CD Integration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Use snapshots in CI/CD pipelines for automated regression testing.
            </p>

            <CodeBlockServer language="bash" filename="CI Configuration">
              {ciExample}
            </CodeBlockServer>
          </section>

          {/* Output Examples */}
          <section>
            <H2 id="output">Output Examples</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Snapshot testing provides clear, colorized output showing what changed.
            </p>

            <CodeBlockServer language="text" filename="Snapshot Output">
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
                      <div>• Commit snapshot files to version control</div>
                      <div>• Exclude dynamic fields (timestamps, IDs)</div>
                      <div>
                        • Use <code className="font-mono">--ci-snapshot</code> in CI pipelines
                      </div>
                      <div>• Review snapshot changes in code reviews</div>
                      <div>• Use meaningful request names</div>
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
                      <div>• Don't snapshot volatile data without exclusions</div>
                      <div>• Keep snapshots focused and readable</div>
                      <div>• Update snapshots intentionally, not automatically</div>
                      <div>
                        • Use <code className="font-mono">--update-failing</code> for targeted fixes
                      </div>
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
                <code className="font-mono">CURL_RUNNER_SNAPSHOT</code>
                <span className="text-muted-foreground">Enable snapshot testing (true/false)</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_SNAPSHOT_UPDATE</code>
                <span className="text-muted-foreground">Update mode (none/all/failing)</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_SNAPSHOT_DIR</code>
                <span className="text-muted-foreground">Snapshot directory</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <code className="font-mono">CURL_RUNNER_SNAPSHOT_CI</code>
                <span className="text-muted-foreground">CI mode (true/false)</span>
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
