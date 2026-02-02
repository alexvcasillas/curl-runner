import { AlertCircle, CheckCircle, FileText, Lock, Shield } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';

export const metadata: Metadata = {
  title: 'Environment Files & Secret Redaction',
  description:
    'Load variables from .env files with environment-specific overrides and automatic secret redaction in outputs.',
  keywords: [
    'curl-runner env files',
    'dotenv support',
    'environment variables',
    'secret redaction',
    'api key protection',
    'sensitive data',
    'environment overrides',
    '.env files',
    'secret masking',
  ],
  openGraph: {
    title: 'Environment Files & Secret Redaction | curl-runner Documentation',
    description:
      'Load variables from .env files with environment-specific overrides and automatic secret redaction in outputs.',
    url: 'https://www.curl-runner.com/docs/features/env-files',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Environment Files & Secret Redaction | curl-runner Documentation',
    description: 'Load .env files with automatic secret redaction in curl-runner.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/env-files',
  },
};

const basicEnvExample = `# .env
API_URL=https://api.example.com
API_VERSION=v1

# Secrets (automatically redacted in output)
SECRET_API_KEY=sk_live_abc123def456
SECRET_TOKEN=ghp_xxxxxxxxxxxx`;

const envOverrideExample = `# .env (base defaults)
API_URL=https://api.example.com
DEBUG=false

# .env.local (local overrides, gitignored)
DEBUG=true

# .env.staging (environment-specific)
API_URL=https://staging.api.example.com

# .env.staging.local (local staging overrides)
API_URL=https://my-staging.api.example.com`;

const priorityChainExample = `# Priority (lowest to highest):
# 1. .env              - Base defaults
# 2. .env.local        - Local overrides (gitignored)
# 3. .env.{env}        - Environment-specific
# 4. .env.{env}.local  - Local environment overrides

# Example: with --env staging
# Loads: .env -> .env.local -> .env.staging -> .env.staging.local`;

const cliUsageExample = `# Use staging environment
curl-runner api.yaml --env staging
curl-runner api.yaml -e staging

# Use production environment
curl-runner api.yaml --env production

# Disable secret redaction (not recommended)
curl-runner api.yaml --no-redact`;

const yamlConfigExample = `# curl-runner.yaml
global:
  env:
    environment: staging    # Default environment
    redactSecrets: true     # Enable redaction (default)
  variables:
    # These override .env values
    CUSTOM_VAR: "from-config"`;

const secretPrefixExample = `# Variables starting with SECRET_ are automatically redacted
SECRET_API_KEY=sk_live_abc123
SECRET_DATABASE_URL=postgres://user:pass@host/db
SECRET_JWT_TOKEN=eyJhbGciOiJIUzI1NiIs...

# Regular variables are NOT redacted
API_KEY=public_key_123
DATABASE_HOST=localhost`;

const patternDetectionExample = `# These patterns are automatically detected and redacted:

# Stripe keys
sk_live_... / sk_test_... / pk_live_... / rk_live_...

# AWS credentials
AKIA... (AWS Access Key IDs)

# GitHub tokens
ghp_... / gho_... / ghu_... / ghs_... / ghr_...

# NPM tokens
npm_...

# Slack tokens
xoxb-... / xoxp-...

# Paddle keys
pdl_...

# OpenAI keys
sk-...

# Anthropic keys
sk-ant-api03-...

# Bearer tokens
Bearer <40+ char token>`;

const outputExample = `# Without redaction (--no-redact)
$ curl-runner api.yaml --dry-run --no-redact
  curl -H 'Authorization: Bearer sk_live_abc123def456' ...

# With redaction (default)
$ curl-runner api.yaml --dry-run
  curl -H 'Authorization: Bearer [REDACTED]' ...`;

const requestYamlExample = `# api.yaml
request:
  url: \${API_URL}/users
  method: GET
  headers:
    Authorization: Bearer \${SECRET_API_KEY}
    X-Api-Version: \${API_VERSION}`;

const gitignoreExample = `# .gitignore
.env.local
.env.*.local
.env.production`;

export default function EnvFilesPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Environment Files & Secret Redaction"
          text="Load variables from .env files with automatic secret protection in CLI output."
        />

        <div className="space-y-12">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground text-lg mb-6">
              curl-runner automatically loads variables from <code className="font-mono">.env</code>{' '}
              files and redacts sensitive values in output. This keeps secrets safe when sharing
              logs or terminal output.
            </p>

            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <FileText className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">.env Support</h4>
                    <p className="text-sm text-muted-foreground">
                      Auto-load variables from .env files
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Lock className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Secret Redaction</h4>
                    <p className="text-sm text-muted-foreground">
                      Automatically mask sensitive values
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
                    <h4 className="font-medium mb-1">Pattern Detection</h4>
                    <p className="text-sm text-muted-foreground">
                      Auto-detect common API key formats
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
              Create a <code className="font-mono">.env</code> file in your project root. Variables
              are automatically loaded and available via{' '}
              <code className="font-mono">
                {'$'}
                {'{VAR_NAME}'}
              </code>{' '}
              syntax.
            </p>

            <div className="space-y-4">
              <CodeBlockServer language="bash" filename=".env">
                {basicEnvExample}
              </CodeBlockServer>

              <CodeBlockServer language="yaml" filename="api.yaml">
                {requestYamlExample}
              </CodeBlockServer>
            </div>
          </section>

          {/* Environment Overrides */}
          <section>
            <H2 id="environment-overrides">Environment Overrides</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Use environment-specific files to override values for different deployments.
            </p>

            <CodeBlockServer language="bash" filename="Environment Files">
              {envOverrideExample}
            </CodeBlockServer>

            <div className="mt-6">
              <H3 id="priority-chain">Priority Chain</H3>
              <p className="text-muted-foreground mb-4">
                Files are loaded in order, with later files overriding earlier values.
              </p>

              <CodeBlockServer language="bash" filename="Load Priority">
                {priorityChainExample}
              </CodeBlockServer>
            </div>
          </section>

          {/* CLI Options */}
          <section>
            <H2 id="cli-options">CLI Options</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Use the <code className="font-mono">--env</code> flag to select an environment.
            </p>

            <div className="grid gap-4 mb-6">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">-e</code>
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --env &lt;name&gt;
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Select environment (loads .env.{'{name}'} files)
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        --no-redact
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Disable secret redaction in output (not recommended)
                    </p>
                  </div>
                </div>
              </div>
            </div>

            <CodeBlockServer language="bash" filename="CLI Usage">
              {cliUsageExample}
            </CodeBlockServer>
          </section>

          {/* Configuration */}
          <section>
            <H2 id="configuration">Configuration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Set defaults in <code className="font-mono">curl-runner.yaml</code>.
            </p>

            <CodeBlockServer language="yaml" filename="curl-runner.yaml">
              {yamlConfigExample}
            </CodeBlockServer>
          </section>

          {/* Secret Redaction */}
          <section>
            <H2 id="secret-redaction">Secret Redaction</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Secrets are automatically redacted in URLs and curl command output.
            </p>

            <div className="mb-6">
              <H3 id="secret-prefix">SECRET_ Prefix</H3>
              <p className="text-muted-foreground mb-4">
                Variables starting with <code className="font-mono">SECRET_</code> are always
                redacted.
              </p>

              <CodeBlockServer language="bash" filename="Secret Variables">
                {secretPrefixExample}
              </CodeBlockServer>
            </div>

            <div className="mb-6">
              <H3 id="pattern-detection">Pattern Detection</H3>
              <p className="text-muted-foreground mb-4">
                Common API key patterns are automatically detected and redacted, even without the
                SECRET_ prefix.
              </p>

              <CodeBlockServer language="text" filename="Detected Patterns">
                {patternDetectionExample}
              </CodeBlockServer>
            </div>

            <div>
              <H3 id="redaction-output">Redaction in Output</H3>
              <p className="text-muted-foreground mb-4">
                Secrets appear as <code className="font-mono">[REDACTED]</code> in output.
              </p>

              <CodeBlockServer language="bash" filename="Output Comparison">
                {outputExample}
              </CodeBlockServer>
            </div>
          </section>

          {/* Git Ignore */}
          <section>
            <H2 id="gitignore">Git Ignore</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Always gitignore local and production env files to prevent accidental commits.
            </p>

            <CodeBlockServer language="text" filename=".gitignore">
              {gitignoreExample}
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
                        • Prefix secrets with <code className="font-mono">SECRET_</code>
                      </div>
                      <div>
                        • Use <code className="font-mono">.env.local</code> for personal overrides
                      </div>
                      <div>
                        • Keep <code className="font-mono">.env</code> in git with safe defaults
                      </div>
                      <div>• Gitignore all .local and production files</div>
                      <div>• Use environment-specific files for staging/production</div>
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
                    <h4 className="font-medium mb-2">Avoid</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <div>
                        • Using <code className="font-mono">--no-redact</code> in shared terminals
                      </div>
                      <div>• Committing .env.local or .env.production</div>
                      <div>• Storing production secrets in base .env</div>
                      <div>• Sharing terminal output without reviewing for secrets</div>
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
