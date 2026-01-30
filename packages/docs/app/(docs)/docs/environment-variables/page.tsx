import {
  AlertTriangle,
  Clock,
  Eye,
  FileText,
  Layers,
  RefreshCw,
  RotateCcw,
  Save,
  Settings,
  Zap,
} from 'lucide-react';
import type { Metadata } from 'next';

export const metadata: Metadata = {
  title: 'Environment Variables',
  description:
    'Configure curl-runner behavior using environment variables for consistent settings across different environments.',
  keywords: [
    'curl-runner environment variables',
    'env vars',
    'configuration',
    'environment settings',
    'CURL_RUNNER variables',
    'runtime configuration',
    'system environment',
    'shell variables',
  ],
  openGraph: {
    title: 'Environment Variables | curl-runner Documentation',
    description:
      'Configure curl-runner behavior using environment variables for consistent settings across different environments.',
    url: 'https://www.curl-runner.com/docs/environment-variables',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Environment Variables | curl-runner Documentation',
    description: 'Configure curl-runner behavior using environment variables.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/environment-variables',
  },
};

import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

const basicUsageExample = `# Set verbose output
export CURL_RUNNER_VERBOSE=true
curl-runner tests/

# Set parallel execution
export CURL_RUNNER_EXECUTION=parallel
curl-runner api-tests.yaml

# Set multiple variables
export CURL_RUNNER_VERBOSE=true
export CURL_RUNNER_EXECUTION=parallel
export CURL_RUNNER_CONTINUE_ON_ERROR=true
curl-runner tests/

# Inline environment variables (one-time use)
CURL_RUNNER_VERBOSE=true curl-runner tests/`;

const outputConfigExample = `# Set output format
export CURL_RUNNER_OUTPUT_FORMAT=json
curl-runner tests/

# Set pretty level for detailed output
export CURL_RUNNER_OUTPUT_FORMAT=pretty
export CURL_RUNNER_PRETTY_LEVEL=detailed
curl-runner tests/

# Save output to file
export CURL_RUNNER_OUTPUT_FILE=results.json
curl-runner tests/

# Enable verbose logging
export CURL_RUNNER_VERBOSE=true
curl-runner tests/`;

const requestConfigExample = `# Set global timeout (milliseconds)
export CURL_RUNNER_TIMEOUT=10000
curl-runner tests/

# Configure retry behavior
export CURL_RUNNER_RETRIES=3
export CURL_RUNNER_RETRY_DELAY=2000
curl-runner tests/

# Disable retries
export CURL_RUNNER_RETRIES=0
curl-runner tests/`;

const executionConfigExample = `# Set execution mode
export CURL_RUNNER_EXECUTION=parallel
curl-runner tests/

# Continue on errors
export CURL_RUNNER_CONTINUE_ON_ERROR=true
curl-runner tests/

# Limit concurrent requests in parallel mode
export CURL_RUNNER_MAX_CONCURRENCY=5
curl-runner tests/

# Combine execution settings
export CURL_RUNNER_EXECUTION=parallel
export CURL_RUNNER_CONTINUE_ON_ERROR=true
export CURL_RUNNER_MAX_CONCURRENCY=10
curl-runner tests/`;

const dockerExample = `# Dockerfile example
FROM node:20-alpine
ENV CURL_RUNNER_VERBOSE=false
ENV CURL_RUNNER_OUTPUT_FORMAT=json
ENV CURL_RUNNER_EXECUTION=sequential
WORKDIR /app
RUN npm install -g curl-runner
CMD ["curl-runner", "/tests"]`;

const ciExample = `# GitHub Actions
- name: Run API Tests
  env:
    CURL_RUNNER_VERBOSE: true
    CURL_RUNNER_OUTPUT_FORMAT: json
    CURL_RUNNER_CONTINUE_ON_ERROR: true
  run: curl-runner tests/

# GitLab CI
test:
  variables:
    CURL_RUNNER_EXECUTION: parallel
    CURL_RUNNER_TIMEOUT: "30000"
  script:
    - curl-runner api-tests.yaml

# Jenkins Pipeline
pipeline {
  environment {
    CURL_RUNNER_VERBOSE = 'true'
    CURL_RUNNER_OUTPUT_FILE = 'test-results.json'
  }
  stages {
    stage('Test') {
      steps {
        sh 'curl-runner tests/'
      }
    }
  }
}`;

const variables = [
  {
    name: 'CURL_RUNNER_VERBOSE',
    description: 'Enable verbose output with detailed request/response information',
    type: 'boolean',
    default: 'false',
    example: 'true',
    icon: Eye,
    color: { bg: 'bg-green-500/10', text: 'text-green-600 dark:text-green-400' },
  },
  {
    name: 'CURL_RUNNER_EXECUTION',
    description: 'Set execution mode: sequential or parallel',
    type: 'string',
    default: 'sequential',
    example: 'parallel',
    icon: Layers,
    color: { bg: 'bg-purple-500/10', text: 'text-purple-600 dark:text-purple-400' },
  },
  {
    name: 'CURL_RUNNER_CONTINUE_ON_ERROR',
    description: 'Continue execution when individual requests fail',
    type: 'boolean',
    default: 'false',
    example: 'true',
    icon: AlertTriangle,
    color: { bg: 'bg-orange-500/10', text: 'text-orange-600 dark:text-orange-400' },
  },
  {
    name: 'CURL_RUNNER_MAX_CONCURRENCY',
    description: 'Maximum number of concurrent requests in parallel mode',
    type: 'number',
    default: 'unlimited',
    example: '5',
    icon: Zap,
    color: { bg: 'bg-teal-500/10', text: 'text-teal-600 dark:text-teal-400' },
  },
  {
    name: 'CURL_RUNNER_DRY_RUN',
    description: 'Show curl commands without executing them (dry run mode)',
    type: 'boolean',
    default: 'false',
    example: 'true',
    icon: Eye,
    color: { bg: 'bg-cyan-500/10', text: 'text-cyan-600 dark:text-cyan-400' },
  },
  {
    name: 'CURL_RUNNER_OUTPUT_FORMAT',
    description: 'Set output format: json, pretty, or raw',
    type: 'string',
    default: 'pretty',
    example: 'json',
    icon: FileText,
    color: { bg: 'bg-cyan-500/10', text: 'text-cyan-600 dark:text-cyan-400' },
  },
  {
    name: 'CURL_RUNNER_PRETTY_LEVEL',
    description: 'Set pretty format detail level: minimal, standard, or detailed',
    type: 'string',
    default: 'minimal',
    example: 'detailed',
    icon: Layers,
    color: { bg: 'bg-pink-500/10', text: 'text-pink-600 dark:text-pink-400' },
  },
  {
    name: 'CURL_RUNNER_OUTPUT_FILE',
    description: 'Save execution results to specified file',
    type: 'string',
    default: 'undefined',
    example: 'results.json',
    icon: Save,
    color: { bg: 'bg-indigo-500/10', text: 'text-indigo-600 dark:text-indigo-400' },
  },
  {
    name: 'CURL_RUNNER_TIMEOUT',
    description: 'Set global timeout for all requests in milliseconds',
    type: 'number',
    default: 'undefined',
    example: '10000',
    icon: Clock,
    color: { bg: 'bg-red-500/10', text: 'text-red-600 dark:text-red-400' },
  },
  {
    name: 'CURL_RUNNER_RETRIES',
    description: 'Set maximum number of retries for failed requests',
    type: 'number',
    default: '0',
    example: '3',
    icon: RotateCcw,
    color: { bg: 'bg-yellow-500/10', text: 'text-yellow-600 dark:text-yellow-500' },
  },
  {
    name: 'CURL_RUNNER_RETRY_DELAY',
    description: 'Set delay between retry attempts in milliseconds',
    type: 'number',
    default: '1000',
    example: '2000',
    icon: Clock,
    color: { bg: 'bg-amber-500/10', text: 'text-amber-600 dark:text-amber-500' },
  },
  {
    name: 'CURL_RUNNER_WATCH',
    description: 'Enable watch mode to automatically re-run when files change',
    type: 'boolean',
    default: 'false',
    example: 'true',
    icon: RefreshCw,
    color: { bg: 'bg-sky-500/10', text: 'text-sky-600 dark:text-sky-400' },
  },
  {
    name: 'CURL_RUNNER_WATCH_DEBOUNCE',
    description: 'Debounce delay for watch mode in milliseconds',
    type: 'number',
    default: '300',
    example: '500',
    icon: Clock,
    color: { bg: 'bg-sky-500/10', text: 'text-sky-600 dark:text-sky-400' },
  },
  {
    name: 'CURL_RUNNER_WATCH_CLEAR',
    description: 'Clear screen between watch mode runs',
    type: 'boolean',
    default: 'true',
    example: 'false',
    icon: Eye,
    color: { bg: 'bg-sky-500/10', text: 'text-sky-600 dark:text-sky-400' },
  },
];

export default function EnvironmentVariablesPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Environment Variables"
          text="Configure curl-runner behavior using environment variables for consistent settings across different environments."
        />

        <div className="space-y-12">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Environment variables provide a way to configure{' '}
              <code className="font-mono">curl-runner</code> without modifying YAML files or using
              command-line options. This is particularly useful for CI/CD pipelines, containerized
              environments, and maintaining consistent settings across teams.
            </p>

            <div className="rounded-lg border bg-card p-4">
              <div className="flex items-start gap-3">
                <div className="rounded-full bg-blue-500/10 p-2">
                  <Settings className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                </div>
                <div className="flex-1">
                  <h4 className="font-medium mb-2">Configuration Precedence</h4>
                  <p className="text-sm text-muted-foreground mb-3">
                    Settings are applied in the following order (later overrides earlier):
                  </p>
                  <ol className="list-decimal list-inside text-sm text-muted-foreground space-y-1">
                    <li>Default values</li>
                    <li>Configuration file (curl-runner.yaml)</li>
                    <li>Environment variables</li>
                    <li>Command-line options</li>
                  </ol>
                </div>
              </div>
            </div>
          </section>

          {/* Basic Usage */}
          <section>
            <H2 id="basic-usage">Basic Usage</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Set environment variables in your shell or include them inline when running commands.
            </p>

            <CodeBlockServer language="bash" filename="Environment Variable Examples">
              {basicUsageExample}
            </CodeBlockServer>
          </section>

          {/* Available Variables */}
          <section>
            <H2 id="available-variables">Available Variables</H2>
            <p className="text-muted-foreground text-lg mb-6">
              All curl-runner environment variables follow the{' '}
              <code className="font-mono">CURL_RUNNER_*</code> naming convention.
            </p>

            <div className="grid gap-4 md:grid-cols-1">
              {variables.map((variable, index) => {
                const Icon = variable.icon;

                return (
                  <div key={`${index}-${variable.name}`} className="rounded-lg border bg-card p-4">
                    <div className="flex items-start gap-3">
                      <div className={`rounded-full ${variable.color.bg} p-2`}>
                        <Icon className={`h-4 w-4 ${variable.color.text}`} />
                      </div>
                      <div className="flex-1">
                        <div className="flex items-start justify-between mb-2">
                          <code className="bg-primary/10 text-primary px-1.5 py-0.5 rounded text-sm font-mono">
                            {variable.name}
                          </code>
                          <div className="flex items-center space-x-2 ml-2">
                            <Badge variant="outline" className="text-xs">
                              {variable.type}
                            </Badge>
                            <Badge variant="secondary" className="text-xs">
                              default: {variable.default}
                            </Badge>
                          </div>
                        </div>
                        <p className="text-sm text-muted-foreground mb-2">{variable.description}</p>
                        <div className="bg-muted rounded px-2 py-1">
                          <code className="text-xs">
                            export {variable.name}={variable.example}
                          </code>
                        </div>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </section>

          {/* Output Configuration */}
          <section>
            <H2 id="output-configuration">Output Configuration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Control how results are displayed and saved using output-related environment
              variables.
            </p>

            <CodeBlockServer language="bash" filename="Output Configuration">
              {outputConfigExample}
            </CodeBlockServer>
          </section>

          {/* Request Configuration */}
          <section>
            <H2 id="request-configuration">Request Configuration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Configure default request behavior including timeouts and retry logic.
            </p>

            <CodeBlockServer language="bash" filename="Request Configuration">
              {requestConfigExample}
            </CodeBlockServer>
          </section>

          {/* Execution Configuration */}
          <section>
            <H2 id="execution-configuration">Execution Configuration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Control how requests are executed and how errors are handled.
            </p>

            <CodeBlockServer language="bash" filename="Execution Configuration">
              {executionConfigExample}
            </CodeBlockServer>
          </section>

          {/* Docker & Containers */}
          <section>
            <H2 id="docker-containers">Docker & Containers</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Environment variables are ideal for containerized deployments where configuration
              should be injected at runtime.
            </p>

            <CodeBlockServer language="dockerfile" filename="Docker Example">
              {dockerExample}
            </CodeBlockServer>
          </section>

          {/* CI/CD Integration */}
          <section>
            <H2 id="ci-cd-integration">CI/CD Integration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Examples of using environment variables in popular CI/CD platforms.
            </p>

            <CodeBlockServer language="yaml" filename="CI/CD Examples">
              {ciExample}
            </CodeBlockServer>
          </section>

          {/* Best Practices */}
          <section>
            <H2 id="best-practices">Best Practices</H2>

            <div className="grid gap-6 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Zap className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Use for Environment-Specific Settings</h4>
                    <p className="text-sm text-muted-foreground">
                      Environment variables are perfect for settings that change between
                      development, staging, and production environments.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Settings className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Keep Secrets Secure</h4>
                    <p className="text-sm text-muted-foreground">
                      While curl-runner doesn't directly handle secrets, avoid logging verbose
                      output in production when sensitive data might be exposed.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <FileText className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Document Your Variables</h4>
                    <p className="text-sm text-muted-foreground">
                      Create a .env.example file in your project to document required environment
                      variables for your team.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <AlertTriangle className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Prefer Explicit Over Implicit</h4>
                    <p className="text-sm text-muted-foreground">
                      For critical settings like timeouts and retries, consider using explicit
                      command-line options for clarity in scripts and CI pipelines.
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
