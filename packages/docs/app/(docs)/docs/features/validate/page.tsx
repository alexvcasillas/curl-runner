import {
  AlertCircle,
  AlertTriangle,
  CheckCircle,
  Code,
  FileSearch,
  Info,
  Key,
  Link,
  Settings,
  ShieldAlert,
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
    'security validation',
    'url validation',
    'header validation',
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

const strictExample = `# Treat warnings as errors (for CI)
curl-runner validate --strict

# Short form
curl-runner validate -s

# Combine with quiet mode
curl-runner validate --strict --quiet`;

const outputExample = `curl-runner validate

Files: 5 found

✓ api/users.yaml
✓ api/posts.yaml
✗ api/invalid.yaml
  ● request.url: URL should start with http:// or https://
    fix: Prepend https://
  ● request.method: Method should be uppercase: "GET"
    fix: Change to "GET"
  ▲ request.headers: Header "content-type" should be "Content-Type"
    fix: Change to "Content-Type"
  ○ request.ssl: Using insecure mode disables certificate verification
✓ api/products.yaml
✓ api/orders.yaml

Summary: 1 file(s) with 4 issue(s) (1 error, 2 warnings, 1 info)`;

const fixedOutputExample = `curl-runner validate --fix

Files: 5 found

✓ api/users.yaml
✓ api/posts.yaml
✗ api/invalid.yaml
  ● request.url: URL should start with http:// or https://
  ● request.method: Method should be uppercase: "GET"
  ▲ request.headers: Header "content-type" should be "Content-Type"
  ○ request.ssl: Using insecure mode disables certificate verification
  ✓ Fixed 3 issue(s)
✓ api/products.yaml
✓ api/orders.yaml

Summary: 1 file(s) with 4 issue(s)
Fixed: 3 issue(s)`;

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

const urlValidationExamples = `# URL Validations

# Missing protocol (warning, auto-fixable)
request:
  url: api.example.com/users
  # Warning: URL should start with http:// or https://
  # Fix: Prepend https://

# Protocol typo (warning, auto-fixable)
request:
  url: htps://api.example.com
  # Warning: URL contains typo "htps", did you mean "https"?
  # Fix: Change "htps" to "https"

# Hostname typo (warning, auto-fixable)
request:
  url: https://locahost:3000/api
  # Warning: URL contains typo "locahost", did you mean "localhost"?
  # Fix: Change "locahost" to "localhost"

# Double slash in path (warning, auto-fixable)
request:
  url: https://api.example.com//users
  # Warning: URL contains double slash in path
  # Fix: Replace "//" with "/"

# Spaces in URL (warning, auto-fixable)
request:
  url: https://api.example.com/users /list
  # Warning: URL contains spaces
  # Fix: Encode spaces as %20

# Invalid port (error)
request:
  url: https://api.example.com:99999/api
  # Error: Invalid port number: 99999`;

const headerValidationExamples = `# Header Validations

# Wrong casing (warning, auto-fixable)
request:
  url: https://api.example.com
  headers:
    content-type: application/json
    authorization: Bearer token
  # Warning: Header "content-type" should be "Content-Type"
  # Warning: Header "authorization" should be "Authorization"

# Content-Type mismatch (info)
request:
  url: https://api.example.com
  method: POST
  headers:
    Content-Type: application/json
  formData:
    field: value
  # Info: Using formData but Content-Type is not multipart/form-data

# Duplicate headers (warning)
request:
  url: https://api.example.com
  headers:
    Content-Type: application/json
    content-type: text/plain
  # Warning: Duplicate header detected (case-insensitive)`;

const securityValidationExamples = `# Security Validations

# Hardcoded credentials (warning)
request:
  url: https://api.example.com
  auth:
    type: basic
    username: admin
    password: secret123
  # Warning: Possible hardcoded password detected
  # Suggestion: Use environment variables

# JWT in header (warning)
request:
  url: https://api.example.com
  headers:
    Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature
  # Warning: Possible hardcoded JWT token
  # Suggestion: Use \${env.TOKEN} instead

# API key patterns (warning)
request:
  url: https://api.example.com
  headers:
    X-API-Key: sk-live-xxxxxxxxxxxxx
  # Warning: Possible hardcoded API key
  # Suggestion: Use environment variables

# Insecure mode (info)
request:
  url: https://api.example.com
  ssl:
    insecure: true
  # Info: Using insecure mode disables certificate verification`;

const unknownKeyExamples = `# Unknown Key Detection with Suggestions

# Typo in key name
request:
  url: https://api.example.com
  methood: GET
  # Error: Unknown key "methood". Did you mean "method"?

# Typo in nested key
request:
  url: https://api.example.com
  auth:
    tpye: bearer
    token: \${TOKEN}
  # Error: Unknown key "tpye" in auth. Did you mean "type"?

# Global config typo
global:
  excution: parallel
  # Error: Unknown key "excution". Did you mean "execution"?`;

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
  # Error: Cannot use both "body" and "formData"

# Body with GET (warning)
request:
  url: https://api.example.com
  method: GET
  body: { "test": "value" }
  # Warning: GET requests should not have a body

# Duplicate request names (warning)
requests:
  - name: GetUser
    url: https://api.example.com/users/1
  - name: GetUser
    url: https://api.example.com/users/2
  # Warning: Duplicate request name "GetUser"`;

const ciExample = `# GitHub Actions
- name: Validate YAML configs
  run: curl-runner validate tests/ --quiet

# GitLab CI (strict mode fails on warnings)
validate:
  script:
    - curl-runner validate tests/ --strict

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
              The <code className="font-mono">curl-runner validate</code> command provides
              comprehensive validation of your YAML configuration files. It checks structure,
              required fields, valid values, security issues, and common typos—with intelligent
              suggestions and auto-fix capabilities.
            </p>

            <div className="grid gap-4 md:grid-cols-3 lg:grid-cols-4">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <FileSearch className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Schema Validation</h4>
                    <p className="text-sm text-muted-foreground">
                      Full curl-runner schema validation
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
                    <p className="text-sm text-muted-foreground">15+ auto-fixable issue types</p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <AlertCircle className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Typo Detection</h4>
                    <p className="text-sm text-muted-foreground">Smart suggestions for misspellings</p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-red-500/10 p-2">
                    <ShieldAlert className="h-4 w-4 text-red-600 dark:text-red-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Security Checks</h4>
                    <p className="text-sm text-muted-foreground">
                      Detects hardcoded secrets & keys
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
                      Automatically fix issues where possible. Fixes URL issues, method casing,
                      header casing, and more.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-red-500/10 p-2">
                    <AlertTriangle className="h-4 w-4 text-red-600 dark:text-red-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        -s, --strict
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Treat warnings as errors. Useful for CI pipelines where you want stricter
                      validation.
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

            <CodeBlockServer language="bash" filename="Strict Mode">
              {strictExample}
            </CodeBlockServer>
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
                    {'\u2022'} Missing <code className="font-mono">https://</code> prefix
                  </li>
                  <li>
                    {'\u2022'} Protocol typos (htps, htpp, etc.)
                  </li>
                  <li>
                    {'\u2022'} Hostname typos (locahost, etc.)
                  </li>
                  <li>
                    {'\u2022'} Double slashes in URL paths
                  </li>
                  <li>
                    {'\u2022'} Spaces in URLs → %20
                  </li>
                  <li>
                    {'\u2022'} Header casing (content-type → Content-Type)
                  </li>
                  <li>
                    {'\u2022'} Lowercase execution mode
                  </li>
                  <li>
                    {'\u2022'} Lowercase output format
                  </li>
                  <li>
                    {'\u2022'} Lowercase prettyLevel
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
                  <li>{'\u2022'} Missing required fields (url)</li>
                  <li>{'\u2022'} Invalid status codes</li>
                  <li>{'\u2022'} Schema conflicts (body + formData)</li>
                  <li>{'\u2022'} Unknown keys with no suggestion</li>
                  <li>{'\u2022'} Invalid port numbers</li>
                  <li>{'\u2022'} Hardcoded credentials</li>
                  <li>{'\u2022'} Invalid JSON body</li>
                </ul>
              </div>
            </div>
          </section>

          {/* Output Examples */}
          <section>
            <H2 id="output">Output Examples</H2>
            <p className="text-muted-foreground text-lg mb-6">
              The validate command provides clear, colorized output showing what passed, what
              failed, and the severity of each issue.
            </p>

            <CodeBlockServer language="text" filename="Validation Output">
              {outputExample}
            </CodeBlockServer>

            <p className="text-muted-foreground mt-6 mb-4">With auto-fix enabled:</p>

            <CodeBlockServer language="text" filename="Output with --fix">
              {fixedOutputExample}
            </CodeBlockServer>
          </section>

          {/* URL Validations */}
          <section>
            <H2 id="url-validations">URL Validations</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Comprehensive URL validation with typo detection and auto-fix capabilities.
            </p>

            <CodeBlockServer language="yaml" filename="URL Validations">
              {urlValidationExamples}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-card p-4">
              <h4 className="font-medium mb-2 flex items-center gap-2">
                <Link className="h-4 w-4" />
                Detected URL Typos
              </h4>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-2 text-sm">
                <div>
                  <code className="font-mono">htpp</code> → http
                </div>
                <div>
                  <code className="font-mono">htps</code> → https
                </div>
                <div>
                  <code className="font-mono">htp</code> → http
                </div>
                <div>
                  <code className="font-mono">httpss</code> → https
                </div>
                <div>
                  <code className="font-mono">locahost</code> → localhost
                </div>
                <div>
                  <code className="font-mono">localhsot</code> → localhost
                </div>
                <div>
                  <code className="font-mono">localhos</code> → localhost
                </div>
                <div>
                  <code className="font-mono">lcoalhost</code> → localhost
                </div>
              </div>
            </div>
          </section>

          {/* Header Validations */}
          <section>
            <H2 id="header-validations">Header Validations</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Validates header names, detects duplicates, and checks Content-Type consistency.
            </p>

            <CodeBlockServer language="yaml" filename="Header Validations">
              {headerValidationExamples}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-card p-4">
              <h4 className="font-medium mb-2">Standard Header Casing</h4>
              <div className="grid grid-cols-2 md:grid-cols-3 gap-2 text-sm text-muted-foreground">
                <div>Content-Type</div>
                <div>Authorization</div>
                <div>Accept</div>
                <div>Cache-Control</div>
                <div>Content-Length</div>
                <div>User-Agent</div>
                <div>X-Requested-With</div>
                <div>X-API-Key</div>
                <div>Accept-Encoding</div>
              </div>
            </div>
          </section>

          {/* Security Validations */}
          <section>
            <H2 id="security-validations">Security Validations</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Detects potential security issues like hardcoded credentials and API keys.
            </p>

            <CodeBlockServer language="yaml" filename="Security Validations">
              {securityValidationExamples}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-card p-4">
              <h4 className="font-medium mb-2 flex items-center gap-2">
                <Key className="h-4 w-4" />
                Detected Patterns
              </h4>
              <ul className="text-sm text-muted-foreground space-y-1">
                <li>{'\u2022'} JWT tokens (Bearer eyJ...)</li>
                <li>{'\u2022'} Stripe API keys (sk_live_*, sk_test_*)</li>
                <li>{'\u2022'} AWS access keys (AKIA...)</li>
                <li>
                  {'\u2022'} Common password field names (password, passwd, secret, api_key, etc.)
                </li>
                <li>{'\u2022'} Hardcoded bearer tokens</li>
                <li>{'\u2022'} Insecure SSL mode</li>
              </ul>
            </div>
          </section>

          {/* Unknown Key Detection */}
          <section>
            <H2 id="unknown-keys">Unknown Key Detection</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Detects unknown configuration keys and suggests corrections using edit distance
              matching.
            </p>

            <CodeBlockServer language="yaml" filename="Unknown Key Detection">
              {unknownKeyExamples}
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
                  <li>{'\u2022'} Unknown keys are flagged with suggestions</li>
                  <li>{'\u2022'} Valid YAML syntax</li>
                  <li>{'\u2022'} Duplicate request names detected</li>
                </ul>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <Code className="h-4 w-4" />
                  Request Fields
                </h4>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>
                    {'\u2022'} <code className="font-mono">url</code> - Required, validated format
                  </li>
                  <li>
                    {'\u2022'} <code className="font-mono">method</code> - GET, POST, PUT, DELETE,
                    PATCH, HEAD, OPTIONS
                  </li>
                  <li>
                    {'\u2022'} <code className="font-mono">headers</code> - Casing, duplicates
                  </li>
                  <li>
                    {'\u2022'} <code className="font-mono">body</code> - JSON validation, method
                    compatibility
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
                  <li>{'\u2022'} Hardcoded credential detection</li>
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
                  <li>{'\u2022'} Insecure mode warning</li>
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
                  <li>{'\u2022'} Duration format (e.g., &lt;500ms)</li>
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

              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <Settings className="h-4 w-4" />
                  Timeouts & Retries
                </h4>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>{'\u2022'} timeout must be positive number</li>
                  <li>{'\u2022'} retry.count must be non-negative</li>
                  <li>{'\u2022'} retry.delay must be non-negative</li>
                  <li>{'\u2022'} retry.maxDelay must be non-negative</li>
                </ul>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <Settings className="h-4 w-4" />
                  Proxy Config
                </h4>
                <ul className="text-sm text-muted-foreground space-y-1">
                  <li>{'\u2022'} proxy.url must be valid URL</li>
                  <li>{'\u2022'} proxy.auth requires username</li>
                  <li>{'\u2022'} Proxy URL protocol validation</li>
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
                  {'\u2022'} <code className="font-mono">0</code> - All files valid (no errors)
                </li>
                <li>
                  {'\u2022'} <code className="font-mono">1</code> - One or more files have errors
                </li>
                <li>
                  {'\u2022'} <code className="font-mono">1</code> (with --strict) - Files have
                  errors OR warnings
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
                      <div>{'\u2022'} Use --strict in CI pipelines</div>
                      <div>{'\u2022'} Use --quiet in automated scripts</div>
                      <div>{'\u2022'} Review auto-fixes before committing</div>
                      <div>{'\u2022'} Use environment variables for credentials</div>
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
                      <div>{'\u2022'} Warnings don{"'"}t fail validation (unless --strict)</div>
                      <div>
                        {'\u2022'} Variables (${'{'}{'{'}VAR{'}'}) are not resolved during validation
                      </div>
                      <div>{'\u2022'} Security warnings suggest using env vars</div>
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
                  <span className="font-mono">● Red</span>
                </div>
                <span className="text-muted-foreground">Must be fixed. Fails validation.</span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <div className="flex items-center gap-2">
                  <Badge variant="secondary" className="bg-yellow-500/20 text-yellow-600">
                    Warning
                  </Badge>
                  <span className="font-mono">▲ Yellow</span>
                </div>
                <span className="text-muted-foreground">
                  Should be fixed. Often auto-fixable. Fails with --strict.
                </span>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <div className="flex items-center gap-2">
                  <Badge variant="secondary" className="bg-blue-500/20 text-blue-600">
                    Info
                  </Badge>
                  <span className="font-mono">○ Blue</span>
                </div>
                <span className="text-muted-foreground">
                  For awareness. Does not affect exit code.
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
