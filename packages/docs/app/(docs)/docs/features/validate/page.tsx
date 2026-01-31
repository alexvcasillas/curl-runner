import {
  AlertCircle,
  CheckCircle,
  Code,
  FileSearch,
  Settings,
  ShieldCheck,
  Terminal,
  Wrench,
} from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'YAML Validation',
  description:
    'Validate curl-runner YAML configuration files against the schema. Discover issues, get fix suggestions, and auto-fix common problems.',
  keywords: [
    'curl-runner validate',
    'yaml validation',
    'configuration validation',
    'schema validation',
    'yaml linter',
    'auto-fix',
    'syntax checking',
    'API testing',
    'curl validation',
    'yaml schema',
  ],
  openGraph: {
    title: 'YAML Validation | curl-runner Documentation',
    description:
      'Validate curl-runner YAML configuration files against the schema. Discover issues, get fix suggestions, and auto-fix common problems.',
    url: 'https://www.curl-runner.com/docs/features/validate',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'YAML Validation | curl-runner Documentation',
    description: 'Learn how to validate YAML files and auto-fix issues with curl-runner.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/validate',
  },
};

const basicExample = `# Validate all YAML files in current directory
curl-runner validate

# Validate specific file
curl-runner validate api.yaml

# Validate directory
curl-runner validate tests/

# Validate with glob pattern
curl-runner validate "tests/**/*.yaml"`;

const fixExample = `# Validate and auto-fix issues
curl-runner validate --fix

# Short form
curl-runner validate -f

# Fix quietly (only show errors)
curl-runner validate -fq

# Fix specific files
curl-runner validate api.yaml --fix`;

const outputExample = `curl-runner validate

Files: 5 found

✓ api/users.yaml
✓ api/posts.yaml
✗ api/invalid.yaml
  ● request.url: URL should start with http:// or https://
    fix: Prepend https://
  ● request.method: Method should be uppercase: "GET"
    fix: Change to "GET"
  ● request.auth.type: Invalid auth type "oauth". Must be: basic, bearer
✓ api/products.yaml
✓ api/orders.yaml

Summary: 1 file(s) with 3 issue(s)`;

const fixedOutputExample = `curl-runner validate --fix

Files: 5 found

✓ api/users.yaml
✓ api/posts.yaml
✗ api/invalid.yaml
  ● request.url: URL should start with http:// or https://
  ● request.method: Method should be uppercase: "GET"
  ● request.auth.type: Invalid auth type "oauth". Must be: basic, bearer
  ✓ Fixed 2 issue(s)
✓ api/products.yaml
✓ api/orders.yaml

Summary: 1 file(s) with 3 issue(s)
Fixed: 2 issue(s)`;

const validRequestExample = `# Valid request configuration
request:
  name: Get User
  url: https://api.example.com/users/1
  method: GET
  headers:
    Authorization: Bearer \${TOKEN}
    Content-Type: application/json
  timeout: 5000
  expect:
    status: 200
    headers:
      content-type: application/json
    body:
      id: 1`;

const validGlobalExample = `# Valid global configuration
global:
  execution: parallel
  maxConcurrency: 5
  continueOnError: true
  output:
    format: pretty
    prettyLevel: detailed
  ci:
    strictExit: true
    failOn: 2
    failOnPercentage: 10

requests:
  - url: https://api.example.com/users
  - url: https://api.example.com/posts`;

const invalidExamples = `# Common validation errors

# Missing URL (error)
request:
  method: GET
  # Error: URL is required

# Invalid method (error)
request:
  url: https://api.example.com
  method: INVALID
  # Error: Invalid method "INVALID". Must be: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS

# Lowercase method (warning, auto-fixable)
request:
  url: https://api.example.com
  method: get
  # Warning: Method should be uppercase: "GET"

# Missing https:// (warning, auto-fixable)
request:
  url: api.example.com/users
  # Warning: URL should start with http:// or https://

# Invalid auth type (error)
request:
  url: https://api.example.com
  auth:
    type: oauth
  # Error: Invalid auth type "oauth". Must be: basic, bearer

# Invalid status code (error)
request:
  url: https://api.example.com
  expect:
    status: 1000
  # Error: Status code must be a number between 100-599

# Body and formData conflict (error)
request:
  url: https://api.example.com
  body: { "test": "value" }
  formData:
    field: value
  # Error: Cannot use both "body" and "formData"`;

const ciExample = `# GitHub Actions
- name: Validate YAML configs
  run: curl-runner validate tests/ --quiet

# GitLab CI
validate:
  script:
    - curl-runner validate tests/

# Pre-commit hook
curl-runner validate && curl-runner tests/`;

export default function ValidatePage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="YAML Validation"
          text="Validate curl-runner YAML configuration files against the schema. Discover issues, get fix suggestions, and auto-fix common problems with a single command."
        />

        <div className="space-y-12">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground text-lg mb-6">
              The <code className="font-mono">curl-runner validate</code> command checks your YAML
              configuration files for correctness. It validates the structure, required fields,
              valid values, and consistency of your configurations before you run them.
            </p>

            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <FileSearch className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Schema Validation</h4>
                    <p className="text-sm text-muted-foreground">
                      Validates against the full curl-runner schema
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Wrench className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Auto-Fix</h4>
                    <p className="text-sm text-muted-foreground">
                      Automatically fix common issues with --fix
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <AlertCircle className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Clear Reports</h4>
                    <p className="text-sm text-muted-foreground">
                      Detailed error messages with fix suggestions
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
              Run <code className="font-mono">curl-runner validate</code> to check your YAML files.
            </p>

            <CodeBlockServer language="bash" filename="CLI Usage">
              {basicExample}
            </CodeBlockServer>
          </section>

          {/* CLI Options */}
          <section>
            <H2 id="cli-options">CLI Options</H2>

            <div className="grid gap-4 mb-6">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Wrench className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        -f, --fix
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Automatically fix issues where possible. Currently fixes lowercase HTTP
                      methods and missing URL schemes.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Terminal className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        -q, --quiet
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Only show files with issues. Valid files are not printed.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Code className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        -h, --help
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">Show help message and exit.</p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Auto-Fix */}
          <section>
            <H2 id="auto-fix">Auto-Fix</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Use <code className="font-mono">--fix</code> to automatically fix common issues.
            </p>

            <CodeBlockServer language="bash" filename="Auto-Fix Usage">
              {fixExample}
            </CodeBlockServer>

            <div className="mt-6 grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <CheckCircle className="h-4 w-4 text-green-500" />
                  Auto-Fixable Issues
                </h4>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>
                    {'\u2022'} Lowercase HTTP methods → UPPERCASE
                  </li>
                  <li>
                    {'\u2022'} Missing <code className="font-mono">https://</code> prefix on URLs
                  </li>
                </ul>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <AlertCircle className="h-4 w-4 text-orange-500" />
                  Manual Fix Required
                </h4>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>{'\u2022'} Invalid auth types</li>
                  <li>{'\u2022'} Missing required fields</li>
                  <li>{'\u2022'} Invalid status codes</li>
                  <li>{'\u2022'} Schema conflicts</li>
                </ul>
              </div>
            </div>
          </section>

          {/* Output Examples */}
          <section>
            <H2 id="output">Output Examples</H2>
            <p className="text-muted-foreground text-lg mb-6">
              The validate command provides clear, colorized output showing what passed and what
              failed.
            </p>

            <CodeBlockServer language="text" filename="Validation Output">
              {outputExample}
            </CodeBlockServer>

            <p className="text-muted-foreground mt-6 mb-4">With auto-fix enabled:</p>

            <CodeBlockServer language="text" filename="Output with --fix">
              {fixedOutputExample}
            </CodeBlockServer>
          </section>

          {/* What Gets Validated */}
          <section>
            <H2 id="validation-rules">What Gets Validated</H2>
            <p className="text-muted-foreground text-lg mb-6">
              The validator checks all aspects of your curl-runner configuration.
            </p>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <Settings className="h-4 w-4" />
                  Structure
                </h4>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>
                    {'\u2022'} Must have <code className="font-mono">request</code>,{' '}
                    <code className="font-mono">requests</code>, or{' '}
                    <code className="font-mono">collection</code>
                  </li>
                  <li>{'\u2022'} Unknown top-level keys are flagged</li>
                  <li>{'\u2022'} Valid YAML syntax</li>
                </ul>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <Code className="h-4 w-4" />
                  Request Fields
                </h4>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>
                    {'\u2022'} <code className="font-mono">url</code> - Required, valid format
                  </li>
                  <li>
                    {'\u2022'} <code className="font-mono">method</code> - GET, POST, PUT, DELETE,
                    PATCH, HEAD, OPTIONS
                  </li>
                  <li>
                    {'\u2022'} <code className="font-mono">headers</code>,{' '}
                    <code className="font-mono">params</code> - Must be objects
                  </li>
                  <li>
                    {'\u2022'} <code className="font-mono">body</code> /{' '}
                    <code className="font-mono">formData</code> - Cannot use both
                  </li>
                </ul>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <ShieldCheck className="h-4 w-4" />
                  Authentication
                </h4>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>
                    {'\u2022'} <code className="font-mono">auth.type</code> - basic or bearer
                  </li>
                  <li>{'\u2022'} basic auth requires username</li>
                  <li>{'\u2022'} bearer auth requires token</li>
                </ul>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <Settings className="h-4 w-4" />
                  SSL/TLS Config
                </h4>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>{'\u2022'} cert without key is flagged</li>
                  <li>{'\u2022'} key without cert is flagged</li>
                  <li>{'\u2022'} Path values must be strings</li>
                </ul>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <Code className="h-4 w-4" />
                  Expect Assertions
                </h4>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>{'\u2022'} Status codes must be 100-599</li>
                  <li>{'\u2022'} Response time format validation</li>
                  <li>{'\u2022'} Headers must be an object</li>
                </ul>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <Settings className="h-4 w-4" />
                  Global Config
                </h4>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>{'\u2022'} execution: sequential or parallel</li>
                  <li>{'\u2022'} output.format: json, pretty, raw</li>
                  <li>{'\u2022'} output.prettyLevel: minimal, standard, detailed</li>
                  <li>{'\u2022'} ci.failOnPercentage: 0-100</li>
                </ul>
              </div>
            </div>
          </section>

          {/* Valid Examples */}
          <section>
            <H2 id="valid-examples">Valid Configuration Examples</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Here are examples of properly configured YAML files that pass validation.
            </p>

            <CodeBlockServer language="yaml" filename="Valid Request">
              {validRequestExample}
            </CodeBlockServer>

            <p className="text-muted-foreground mt-6 mb-4">Global configuration:</p>

            <CodeBlockServer language="yaml" filename="Valid Global Config">
              {validGlobalExample}
            </CodeBlockServer>
          </section>

          {/* Common Errors */}
          <section>
            <H2 id="common-errors">Common Validation Errors</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Here are examples of common validation errors and how to fix them.
            </p>

            <CodeBlockServer language="yaml" filename="Common Errors">
              {invalidExamples}
            </CodeBlockServer>
          </section>

          {/* CI/CD Integration */}
          <section>
            <H2 id="ci-integration">CI/CD Integration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Use validation in your CI/CD pipeline to catch configuration errors before deployment.
            </p>

            <CodeBlockServer language="bash" filename="CI Integration">
              {ciExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-card p-4">
              <h4 className="font-medium mb-2">Exit Codes</h4>
              <ul className="text-sm text-muted-foreground space-y-1">
                <li>
                  {'\u2022'} <code className="font-mono">0</code> - All files valid
                </li>
                <li>
                  {'\u2022'} <code className="font-mono">1</code> - One or more files have issues
                </li>
              </ul>
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
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Recommended</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <div>{'\u2022'} Run validation before committing changes</div>
                      <div>{'\u2022'} Add validation to CI/CD pipelines</div>
                      <div>{'\u2022'} Use --quiet in automated scripts</div>
                      <div>{'\u2022'} Review auto-fixes before committing</div>
                      <div>{'\u2022'} Validate entire test directories at once</div>
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
                      <div>{'\u2022'} Auto-fix changes files in place</div>
                      <div>{'\u2022'} Some issues require manual intervention</div>
                      <div>{'\u2022'} Warnings don't fail the validation</div>
                      <div>{'\u2022'} Variables (${'{VAR}'}) are not resolved during validation</div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Severity Levels */}
          <section>
            <H2 id="severity-levels">Severity Levels</H2>

            <div className="grid gap-2 text-sm">
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <div className="flex items-center gap-2">
                  <Badge variant="destructive">Error</Badge>
                  <span className="font-mono">Red dot</span>
                </div>
                <span className="text-muted-foreground">
                  Must be fixed. Fails validation.
                </span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <div className="flex items-center gap-2">
                  <Badge variant="secondary" className="bg-yellow-500/20 text-yellow-600">
                    Warning
                  </Badge>
                  <span className="font-mono">Yellow dot</span>
                </div>
                <span className="text-muted-foreground">
                  Should be fixed. Often auto-fixable.
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
