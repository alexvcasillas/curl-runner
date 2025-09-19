import {
  AlertTriangle,
  CheckCircle,
  Clock,
  Eye,
  FileText,
  FolderSearch,
  HelpCircle,
  Info,
  Layers,
  RotateCcw,
  Save,
  Terminal,
  XCircle,
  Zap,
} from 'lucide-react';
import type { Metadata } from 'next';

export const metadata: Metadata = {
  title: 'CLI Commands',
  description:
    'Complete command-line interface reference for curl-runner with all available commands, options, and usage examples.',
  keywords: [
    'curl-runner CLI commands',
    'command line interface',
    'curl-runner options',
    'CLI reference',
    'curl-runner help',
    'command line usage',
    'CLI parameters',
    'curl-runner flags',
    'terminal commands',
    'bash commands',
    'command line arguments',
    'curl-runner syntax',
  ],
  openGraph: {
    title: 'CLI Commands | curl-runner Documentation',
    description:
      'Complete command-line interface reference for curl-runner with all available commands, options, and usage examples.',
    url: 'https://www.curl-runner.com/docs/cli-commands',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'CLI Commands | curl-runner Documentation',
    description:
      'Complete command-line interface reference for curl-runner with all available commands and options.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/cli-commands',
  },
};

import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';
import Link from 'next/link';

const basicUsageExample = `# Run a single YAML file
curl-runner simple.yaml

# Run with verbose output
curl-runner simple.yaml --verbose

# Run multiple files
curl-runner file1.yaml file2.yaml file3.yaml

# Run all YAML files in current directory
curl-runner *.yaml

# Run all files in a directory
curl-runner tests/

# Run with short options
curl-runner tests/ -pvc`;

const filePatternExamples = `# Glob patterns
curl-runner "api-*.yaml"        # Files starting with "api-"
curl-runner "**/*.yaml"         # All YAML files recursively
curl-runner "tests/**/*.yml"    # All YML files in tests/ recursively

# Directory patterns
curl-runner tests/              # All YAML/YML files in tests/
curl-runner tests/ --all        # Recursive search in tests/
curl-runner . --all             # All files recursively from current dir

# Multiple patterns
curl-runner "api-*.yaml" "user-*.yaml" tests/`;

const executionOptionsExample = `# Sequential execution (default)
curl-runner tests/

# Parallel execution
curl-runner tests/ --execution parallel
curl-runner tests/ -p

# Continue on errors
curl-runner tests/ --continue-on-error
curl-runner tests/ -c

# Parallel + continue on error
curl-runner tests/ -pc`;

const outputOptionsExample = `# Verbose output
curl-runner tests/ --verbose
curl-runner tests/ -v

# Quiet mode (suppress output)
curl-runner tests/ --quiet
curl-runner tests/ -q

# Save results to file
curl-runner tests/ --output results.json
curl-runner tests/ -o results.json

# Output format options
curl-runner tests/ --output-format json     # JSON output
curl-runner tests/ --output-format pretty   # Pretty output (default)
curl-runner tests/ --output-format raw      # Raw response bodies only

# Pretty level options (when using pretty format)
curl-runner tests/ --pretty-level minimal   # Compact output (default)
curl-runner tests/ --pretty-level standard  # Standard detail
curl-runner tests/ --pretty-level detailed  # Full detail with metrics

# Control what's shown
curl-runner tests/ --show-headers           # Include headers
curl-runner tests/ --show-metrics           # Include metrics
curl-runner tests/ --show-body false        # Hide response body

# Combine options
curl-runner tests/ --output-format pretty --pretty-level detailed --show-metrics`;

const timeoutOptionsExample = `# Set global timeout (milliseconds)
curl-runner tests/ --timeout 10000

# Set maximum retries
curl-runner tests/ --retries 3

# Set retry delay (milliseconds)
curl-runner tests/ --retry-delay 2000

# Disable retries completely
curl-runner tests/ --no-retry

# Combine timeout and retry options
curl-runner tests/ --timeout 5000 --retries 3 --retry-delay 1000`;

const advancedExamples = `# Environment-specific execution
NODE_ENV=production curl-runner api-tests.yaml

# Using environment variables
API_KEY=secret123 curl-runner auth-tests.yaml

# Complex execution with all options
curl-runner tests/ \\
  --execution parallel \\
  --continue-on-error \\
  --verbose \\
  --timeout 30000 \\
  --retries 3 \\
  --output test-results.json \\
  --all`;

const commands = [
  {
    name: 'curl-runner',
    description: 'Main command to execute HTTP requests from YAML files',
    usage: 'curl-runner [files...] [options]',
    examples: [
      'curl-runner simple.yaml',
      'curl-runner tests/ -pv',
      'curl-runner *.yaml --output results.json',
    ],
  },
];

const options = [
  {
    short: '-h',
    long: '--help',
    description: 'Show help information and usage examples',
    type: 'boolean',
    default: 'false',
    icon: HelpCircle,
    color: { bg: 'bg-blue-500/10', text: 'text-blue-600 dark:text-blue-400' },
  },
  {
    short: '-v',
    long: '--verbose',
    description: 'Enable verbose output with detailed request/response information',
    type: 'boolean',
    default: 'false',
    icon: Eye,
    color: { bg: 'bg-green-500/10', text: 'text-green-600 dark:text-green-400' },
  },
  {
    short: '-q',
    long: '--quiet',
    description: 'Suppress non-error output (opposite of verbose)',
    type: 'boolean',
    default: 'false',
    icon: Eye,
    color: { bg: 'bg-gray-500/10', text: 'text-gray-600 dark:text-gray-400' },
  },
  {
    short: '-p',
    long: '--execution parallel',
    description: 'Execute requests in parallel instead of sequential',
    type: 'string',
    default: 'sequential',
    icon: Layers,
    color: { bg: 'bg-purple-500/10', text: 'text-purple-600 dark:text-purple-400' },
  },
  {
    short: '-c',
    long: '--continue-on-error',
    description: 'Continue execution when individual requests fail',
    type: 'boolean',
    default: 'false',
    icon: AlertTriangle,
    color: { bg: 'bg-orange-500/10', text: 'text-orange-600 dark:text-orange-400' },
  },
  {
    short: null,
    long: '--all',
    description: 'Recursively find all YAML files in directories',
    type: 'boolean',
    default: 'false',
    icon: FolderSearch,
    color: { bg: 'bg-teal-500/10', text: 'text-teal-600 dark:text-teal-400' },
  },
  {
    short: '-o',
    long: '--output <file>',
    description: 'Save execution results to a file',
    type: 'string',
    default: 'undefined',
    icon: Save,
    color: { bg: 'bg-indigo-500/10', text: 'text-indigo-600 dark:text-indigo-400' },
  },
  {
    short: null,
    long: '--output-format <format>',
    description: 'Set output format: json, pretty, or raw',
    type: 'string',
    default: 'pretty',
    icon: FileText,
    color: { bg: 'bg-cyan-500/10', text: 'text-cyan-600 dark:text-cyan-400' },
  },
  {
    short: null,
    long: '--pretty-level <level>',
    description: 'Set pretty format detail level: minimal, standard, or detailed',
    type: 'string',
    default: 'minimal',
    icon: Layers,
    color: { bg: 'bg-pink-500/10', text: 'text-pink-600 dark:text-pink-400' },
  },
  {
    short: null,
    long: '--show-headers',
    description: 'Include response headers in output',
    type: 'boolean',
    default: 'false',
    icon: FileText,
    color: { bg: 'bg-blue-500/10', text: 'text-blue-600 dark:text-blue-400' },
  },
  {
    short: null,
    long: '--show-body',
    description: 'Include response body in output',
    type: 'boolean',
    default: 'true',
    icon: FileText,
    color: { bg: 'bg-green-500/10', text: 'text-green-600 dark:text-green-400' },
  },
  {
    short: null,
    long: '--show-metrics',
    description: 'Include performance metrics in output',
    type: 'boolean',
    default: 'false',
    icon: Zap,
    color: { bg: 'bg-purple-500/10', text: 'text-purple-600 dark:text-purple-400' },
  },
  {
    short: null,
    long: '--timeout <ms>',
    description: 'Set global timeout for all requests in milliseconds',
    type: 'number',
    default: 'undefined',
    icon: Clock,
    color: { bg: 'bg-red-500/10', text: 'text-red-600 dark:text-red-400' },
  },
  {
    short: null,
    long: '--retries <count>',
    description: 'Set maximum number of retries for failed requests',
    type: 'number',
    default: '0',
    icon: RotateCcw,
    color: { bg: 'bg-yellow-500/10', text: 'text-yellow-600 dark:text-yellow-500' },
  },
  {
    short: null,
    long: '--retry-delay <ms>',
    description: 'Set delay between retry attempts in milliseconds',
    type: 'number',
    default: '1000',
    icon: Clock,
    color: { bg: 'bg-amber-500/10', text: 'text-amber-600 dark:text-amber-500' },
  },
  {
    short: null,
    long: '--no-retry',
    description: 'Disable retry mechanism completely',
    type: 'boolean',
    default: 'false',
    icon: XCircle,
    color: { bg: 'bg-red-500/10', text: 'text-red-600 dark:text-red-400' },
  },
  {
    short: null,
    long: '--version',
    description: 'Display the curl-runner version',
    type: 'boolean',
    default: 'false',
    icon: Info,
    color: { bg: 'bg-gray-500/10', text: 'text-gray-600 dark:text-gray-400' },
  },
];

export default function CLICommandsPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="CLI Commands"
          text="Complete command-line interface reference for curl-runner with all available commands, options, and usage examples."
        />

        <div className="space-y-12">
          {/* Basic Usage */}
          <section>
            <H2 id="basic-usage">Basic Usage</H2>
            <p className="text-muted-foreground text-lg mb-6">
              The <code className="font-mono">curl-runner</code> CLI provides a simple interface for
              executing HTTP requests defined in YAML files.
            </p>

            <CodeBlockServer language="bash" filename="Basic Usage Examples">
              {basicUsageExample}
            </CodeBlockServer>
          </section>

          {/* Commands */}
          <section>
            <H2 id="commands">Commands</H2>

            <div className="space-y-6">
              {commands.map((command) => (
                <div key={command.name} className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <Terminal className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">
                        <code className="text-lg">{command.name}</code>
                      </h4>
                      <p className="text-sm text-muted-foreground mb-4">{command.description}</p>

                      <div className="space-y-4">
                        <div>
                          <h5 className="font-semibold mb-2 text-sm">Usage</h5>
                          <code className="bg-muted px-2 py-1 rounded text-sm">
                            {command.usage}
                          </code>
                        </div>

                        <div>
                          <h5 className="font-semibold mb-2 text-sm">Examples</h5>
                          <div className="space-y-1">
                            {command.examples.map((example) => (
                              <code
                                key={example}
                                className="block bg-muted px-2 py-1 rounded text-sm"
                              >
                                {example}
                              </code>
                            ))}
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* Options */}
          <section>
            <H2 id="options">Options</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Available command-line options to customize{' '}
              <code className="font-mono">curl-runner</code>'s behavior.
            </p>

            <div className="grid gap-4 md:grid-cols-1">
              {options.map((option, index) => {
                const Icon = option.icon;

                return (
                  <div key={`${index}-${option.long}`} className="rounded-lg border bg-card p-4">
                    <div className="flex items-start gap-3">
                      <div className={`rounded-full ${option.color.bg} p-2`}>
                        <Icon className={`h-4 w-4 ${option.color.text}`} />
                      </div>
                      <div className="flex-1">
                        <div className="flex items-start justify-between mb-2">
                          <div className="flex items-center space-x-2">
                            {option.short && (
                              <code className="bg-primary/10 text-primary px-1.5 py-0.5 rounded text-sm font-mono">
                                {option.short}
                              </code>
                            )}
                            <code className="bg-primary/10 text-primary px-1.5 py-0.5 rounded text-sm font-mono">
                              {option.long}
                            </code>
                          </div>
                          <div className="flex items-center space-x-2 ml-2">
                            <Badge variant="outline" className="text-xs">
                              {option.type}
                            </Badge>
                            <Badge variant="secondary" className="text-xs">
                              default: {option.default}
                            </Badge>
                          </div>
                        </div>
                        <p className="text-sm text-muted-foreground">{option.description}</p>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </section>

          {/* File Patterns */}
          <section>
            <H2 id="file-patterns">File Patterns</H2>
            <p className="text-muted-foreground text-lg mb-6">
              <code className="font-mono">curl-runner</code> supports various file patterns and glob
              expressions for flexible file selection.
            </p>

            <CodeBlockServer language="bash" filename="File Pattern Examples">
              {filePatternExamples}
            </CodeBlockServer>
          </section>

          {/* Execution Modes */}
          <section>
            <H2 id="execution-modes">Execution Modes</H2>

            <div className="grid gap-6 md:grid-cols-2 mb-6">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <FileText className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      Sequential
                      <Badge variant="secondary">Default</Badge>
                    </h4>
                    <p className="text-sm text-muted-foreground">
                      Requests are executed one after another in the order they appear in the YAML
                      file. Useful for dependent requests or when order matters.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Zap className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      Parallel
                      <Badge variant="default">Fast</Badge>
                    </h4>
                    <p className="text-sm text-muted-foreground">
                      All requests are executed simultaneously. Significantly faster for independent
                      requests but may overwhelm the target server.
                    </p>
                  </div>
                </div>
              </div>
            </div>

            <CodeBlockServer language="bash" filename="Execution Mode Examples">
              {executionOptionsExample}
            </CodeBlockServer>
          </section>

          {/* Output Options */}
          <section>
            <H2 id="output-options">Output Options</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Control how <code className="font-mono">curl-runner</code> displays results and saves
              output data.
            </p>

            <CodeBlockServer language="bash" filename="Output Examples">
              {outputOptionsExample}
            </CodeBlockServer>
          </section>

          {/* Timeout and Retry Options */}
          <section>
            <H2 id="timeout-retry-options">Timeout & Retry Options</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Configure timeout and retry behavior for robust request handling.
            </p>

            <CodeBlockServer language="bash" filename="Timeout Examples">
              {timeoutOptionsExample}
            </CodeBlockServer>
          </section>

          {/* Advanced Examples */}
          <section>
            <H2 id="advanced-usage">Advanced Usage</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Complex examples combining multiple options and environment variables.
            </p>

            <CodeBlockServer language="bash" filename="Advanced Examples">
              {advancedExamples}
            </CodeBlockServer>
          </section>

          {/* Exit Codes */}
          <section>
            <H2 id="exit-codes">Exit Codes</H2>
            <p className="text-muted-foreground text-lg mb-6">
              <code className="font-mono">curl-runner</code> returns specific exit codes to indicate
              execution results.
            </p>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1 pt-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      <Badge variant="default" className="bg-green-500">
                        0
                      </Badge>
                      Success
                    </h4>
                    <p className="text-sm text-muted-foreground">
                      All requests completed successfully or <code>--continue-on-error</code> was
                      used.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-red-500/10 p-2">
                    <XCircle className="h-4 w-4 text-red-600 dark:text-red-400" />
                  </div>
                  <div className="flex-1 pt-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      <Badge variant="destructive">1</Badge>
                      Error
                    </h4>
                    <p className="text-sm text-muted-foreground">
                      One or more requests failed and <code>--continue-on-error</code> was not used.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Configuration Alternatives */}
          <section className="mb-16">
            <div className="mb-8">
              <H2 id="configuration-alternatives">Configuration Alternatives</H2>
              <p className="text-muted-foreground">
                Many CLI options can also be configured using files or environment variables
              </p>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-6">
                <h3 className="font-medium mb-2">
                  <Link href="/docs/environment-variables" className="hover:text-primary">
                    Environment Variables
                  </Link>
                </h3>
                <p className="text-sm text-muted-foreground mb-3">
                  Configure curl-runner behavior using CURL_RUNNER_* environment variables
                </p>
                <Link
                  href="/docs/environment-variables"
                  className="text-sm font-medium text-primary hover:underline"
                >
                  Environment setup →
                </Link>
              </div>

              <div className="rounded-lg border bg-card p-6">
                <h3 className="font-medium mb-2">
                  <Link href="/docs/global-settings" className="hover:text-primary">
                    Global Settings
                  </Link>
                </h3>
                <p className="text-sm text-muted-foreground mb-3">
                  Set default behavior in YAML files using global configuration
                </p>
                <Link
                  href="/docs/global-settings"
                  className="text-sm font-medium text-primary hover:underline"
                >
                  Global config →
                </Link>
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
