import { DocsPageHeader } from "@/components/docs-page-header"
import { CodeBlockServer } from "@/components/code-block-server"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { TableOfContents } from "@/components/toc"
import { H2, H3, H4 } from "@/components/mdx-heading"
import { Flag, Info, AlertCircle, CheckCircle } from "lucide-react"

const optionGroups = [
  {
    title: "Help & Information",
    icon: Info,
    options: [
      {
        short: "-h",
        long: "--help",
        type: "boolean",
        default: "false",
        description: "Display help information with usage examples and exit.",
        example: "curl-runner --help"
      },
      {
        short: null,
        long: "--version",
        type: "boolean", 
        default: "false",
        description: "Show the curl-runner version number and exit.",
        example: "curl-runner --version"
      }
    ]
  },
  {
    title: "Execution Control",
    icon: Flag,
    options: [
      {
        short: "-p",
        long: "--execution parallel",
        type: "string",
        default: "sequential", 
        description: "Set execution mode. Options: 'sequential' (default) or 'parallel'.",
        example: "curl-runner tests/ --execution parallel"
      },
      {
        short: "-c", 
        long: "--continue-on-error",
        type: "boolean",
        default: "false",
        description: "Continue executing remaining requests even if some fail.",
        example: "curl-runner tests/ --continue-on-error"
      },
      {
        short: null,
        long: "--timeout <milliseconds>",
        type: "number",
        default: "5000",
        description: "Set global timeout for all requests in milliseconds.",
        example: "curl-runner tests/ --timeout 10000"
      },
      {
        short: null,
        long: "--retries <count>", 
        type: "number",
        default: "0",
        description: "Maximum number of retries for failed requests.",
        example: "curl-runner tests/ --retries 3"
      },
      {
        short: null,
        long: "--retry-delay <milliseconds>",
        type: "number", 
        default: "1000",
        description: "Delay between retry attempts in milliseconds.",
        example: "curl-runner tests/ --retries 3 --retry-delay 2000"
      },
      {
        short: null,
        long: "--no-retry",
        type: "boolean",
        default: "false",
        description: "Disable retry mechanism completely.",
        example: "curl-runner tests/ --no-retry"
      }
    ]
  },
  {
    title: "File Discovery",
    icon: CheckCircle,
    options: [
      {
        short: null,
        long: "--all",
        type: "boolean",
        default: "false", 
        description: "Recursively search for YAML files in directories.",
        example: "curl-runner tests/ --all"
      }
    ]
  },
  {
    title: "Output Control", 
    icon: AlertCircle,
    options: [
      {
        short: "-v",
        long: "--verbose",
        type: "boolean",
        default: "false",
        description: "Enable verbose output with detailed request/response information.",
        example: "curl-runner tests/ --verbose"
      },
      {
        short: null,
        long: "--output <file>",
        type: "string", 
        default: "none",
        description: "Save execution results to a JSON file.",
        example: "curl-runner tests/ --output results.json"
      },
      {
        short: null,
        long: "--quiet",
        type: "boolean",
        default: "false", 
        description: "Suppress all output except errors (opposite of --verbose).",
        example: "curl-runner tests/ --quiet"
      },
      {
        short: null,
        long: "--output-format <format>",
        type: "string",
        default: "pretty",
        description: "Set output format. Options: 'json', 'pretty', or 'raw'.",
        example: "curl-runner tests/ --output-format json"
      },
      {
        short: null,
        long: "--pretty-level <level>",
        type: "string",
        default: "standard",
        description: "Set pretty format detail level. Options: 'minimal', 'standard', or 'detailed'.",
        example: "curl-runner tests/ --pretty-level detailed"
      },
      {
        short: null,
        long: "--show-headers",
        type: "boolean",
        default: "false",
        description: "Include response headers in output.",
        example: "curl-runner tests/ --show-headers"
      },
      {
        short: null,
        long: "--show-body",
        type: "boolean",
        default: "false",
        description: "Include response body in output.",
        example: "curl-runner tests/ --show-body"
      },
      {
        short: null,
        long: "--show-metrics",
        type: "boolean",
        default: "false",
        description: "Include performance metrics in output.",
        example: "curl-runner tests/ --show-metrics"
      }
    ]
  }
]

const combinationExamples = `# Basic combinations
curl-runner tests/ -v                    # Verbose output
curl-runner tests/ -p                    # Parallel execution
curl-runner tests/ -c                    # Continue on error
curl-runner tests/ -pv                   # Parallel + verbose
curl-runner tests/ -pc                   # Parallel + continue on error
curl-runner tests/ -pvc                  # Parallel + verbose + continue on error

# With long options
curl-runner tests/ --execution parallel --verbose --continue-on-error

# Output format combinations
curl-runner tests/ --output-format json --show-metrics
curl-runner tests/ --output-format pretty --pretty-level detailed
curl-runner tests/ --show-headers --show-body --show-metrics

# Advanced combinations
curl-runner tests/ \\
  --execution parallel \\
  --continue-on-error \\
  --output-format pretty \\
  --pretty-level detailed \\
  --show-headers \\
  --show-metrics \\
  --timeout 30000 \\
  --retries 3 \\
  --retry-delay 2000 \\
  --output results.json \\
  --all

# Retry configurations
curl-runner tests/ --retries 5 --retry-delay 1500  # Custom retry settings
curl-runner tests/ --no-retry                      # Disable retries completely`

const environmentExamples = `# Environment variables override CLI options
CURL_RUNNER_TIMEOUT=10000 curl-runner tests/
CURL_RUNNER_RETRIES=3 curl-runner tests/
CURL_RUNNER_RETRY_DELAY=2000 curl-runner tests/
CURL_RUNNER_VERBOSE=true curl-runner tests/
CURL_RUNNER_EXECUTION=parallel curl-runner tests/
CURL_RUNNER_CONTINUE_ON_ERROR=true curl-runner tests/

# Output format environment variables
CURL_RUNNER_OUTPUT_FORMAT=json curl-runner tests/
CURL_RUNNER_PRETTY_LEVEL=detailed curl-runner tests/
CURL_RUNNER_OUTPUT_FILE=results.json curl-runner tests/

# Multiple environment variables
CURL_RUNNER_TIMEOUT=15000 \\
CURL_RUNNER_RETRIES=2 \\
CURL_RUNNER_RETRY_DELAY=1500 \\
CURL_RUNNER_VERBOSE=true \\
CURL_RUNNER_OUTPUT_FORMAT=pretty \\
CURL_RUNNER_PRETTY_LEVEL=detailed \\
curl-runner tests/ --execution parallel

# Mix environment variables and CLI options
CURL_RUNNER_TIMEOUT=10000 \\
CURL_RUNNER_OUTPUT_FORMAT=json \\
curl-runner tests/ --verbose --show-metrics --output results.json`

const outputFormatExamples = `# JSON output format (when using --output)
{
  "summary": {
    "total": 10,
    "successful": 8,
    "failed": 2,
    "duration": 2345
  },
  "requests": [
    {
      "name": "Get Users",
      "status": "success",
      "statusCode": 200,
      "duration": 234,
      "url": "https://api.example.com/users"
    },
    {
      "name": "Invalid Request", 
      "status": "failed",
      "error": "Request timeout after 5000ms",
      "url": "https://slow-api.example.com/endpoint"
    }
  ]
}`

const configFileExample = `# curl-runner.yaml (configuration file)
global:
  execution: parallel
  continueOnError: true
  output:
    verbose: false
    format: pretty
    prettyLevel: detailed
    showHeaders: true
    showBody: true
    showMetrics: true
    saveToFile: results.json
  defaults:
    timeout: 10000
    retry:
      count: 2
      delay: 1000
  variables:
    API_BASE: "https://api.example.com"
    API_VERSION: "v2"

# CLI options override config file settings
curl-runner tests/ --verbose --pretty-level minimal  # Overrides config file settings`

export default function CLIOptionsPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="CLI Options"
          text="Comprehensive reference for all command-line options available in curl-runner, including examples and best practices."
        />

        <div className="space-y-12">
          {/* Introduction */}
          <section>
            <p className="text-lg text-muted-foreground mb-6">
              <code className="font-mono">curl-runner</code> provides numerous CLI options to customize execution behavior, output formatting, and error handling. 
              Options can be combined and have both short and long forms.
            </p>

            <div className="grid gap-6">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Info className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Option Syntax</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Understanding <code className="font-mono">curl-runner</code> option formats
                    </p>
                    <div className="space-y-2 text-sm">
                      <div>• Short options: <code className="bg-muted px-2 py-0.5 rounded font-mono">-v -p -c</code> (can be combined as <code className="bg-muted px-2 py-0.5 rounded font-mono">-vpc</code>)</div>
                      <div>• Long options: <code className="bg-muted px-2 py-0.5 rounded font-mono">--verbose --parallel</code></div>
                      <div>• Options with values: <code className="bg-muted px-2 py-0.5 rounded font-mono">--timeout 5000</code></div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Option Groups */}
          <section>
            <H2>Available Options</H2>
            <p className="text-muted-foreground text-lg mb-8">
              Options are grouped by functionality for easier reference.
            </p>

            <div className="space-y-8">
              {optionGroups.map((group) => {
                const Icon = group.icon
                return (
                  <div key={group.title}>
                    <H3 className="flex items-center space-x-2">
                      <Icon className="h-5 w-5 text-primary" />
                      <span>{group.title}</span>
                    </H3>
                    
                    <div className="grid gap-4 md:grid-cols-2 mt-6">
                      {group.options.map((option, index) => (
                        <div key={index} className="rounded-lg border bg-card p-4">
                          <div className="flex items-start gap-3">
                            <div className="rounded-full bg-blue-500/10 p-2">
                              <Icon className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                            </div>
                            <div className="flex-1">
                              <div className="flex items-center gap-2 mb-2">
                                {option.short && (
                                  <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                                    {option.short}
                                  </code>
                                )}
                                <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                                  {option.long}
                                </code>
                              </div>
                              <p className="text-sm text-muted-foreground mb-3">
                                {option.description}
                              </p>
                              <div className="flex items-center gap-2 mb-3">
                                <Badge variant="outline" className="text-xs">
                                  {option.type}
                                </Badge>
                                <Badge variant="secondary" className="text-xs">
                                  default: {option.default}
                                </Badge>
                              </div>
                              <div>
                                <h5 className="font-medium text-sm mb-2">Example:</h5>
                                <code className="bg-muted px-3 py-2 rounded text-sm block font-mono">
                                  {option.example}
                                </code>
                              </div>
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )
              })}
            </div>
          </section>

          {/* Option Combinations */}
          <section>
            <H2>Combining Options</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Short options can be combined, and multiple options can be used together for powerful configurations.
            </p>

            <CodeBlockServer language="bash" filename="Option Combinations">
              {combinationExamples}
            </CodeBlockServer>
          </section>

          {/* Environment Variables */}
          <section>
            <H2>Environment Variables</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Many CLI options can be set via environment variables, which take precedence over default values but are overridden by explicit CLI options.
            </p>

            <div className="mb-6">
              <H3>Supported Environment Variables</H3>
              <div className="mt-6 space-y-6">
                <div>
                  <h4 className="font-medium mb-3 flex items-center gap-2">
                    <Flag className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    Execution Options
                  </h4>
                  <div className="grid gap-2 text-sm">
                    <div className="flex items-center justify-between py-2">
                      <code className="bg-muted px-2 py-1 rounded font-mono">CURL_RUNNER_EXECUTION</code>
                      <span className="text-muted-foreground">execution mode (sequential/parallel)</span>
                    </div>
                    <div className="flex items-center justify-between py-2">
                      <code className="bg-muted px-2 py-1 rounded font-mono">CURL_RUNNER_CONTINUE_ON_ERROR</code>
                      <span className="text-muted-foreground">continue on error (true/false)</span>
                    </div>
                    <div className="flex items-center justify-between py-2">
                      <code className="bg-muted px-2 py-1 rounded font-mono">CURL_RUNNER_TIMEOUT</code>
                      <span className="text-muted-foreground">global timeout in milliseconds</span>
                    </div>
                    <div className="flex items-center justify-between py-2">
                      <code className="bg-muted px-2 py-1 rounded font-mono">CURL_RUNNER_RETRIES</code>
                      <span className="text-muted-foreground">maximum retry attempts</span>
                    </div>
                  </div>
                </div>
                
                <div>
                  <h4 className="font-medium mb-3 flex items-center gap-2">
                    <AlertCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                    Output Options
                  </h4>
                  <div className="grid gap-2 text-sm">
                    <div className="flex items-center justify-between py-2">
                      <code className="bg-muted px-2 py-1 rounded font-mono">CURL_RUNNER_VERBOSE</code>
                      <span className="text-muted-foreground">enable verbose output (true/false)</span>
                    </div>
                    <div className="flex items-center justify-between py-2">
                      <code className="bg-muted px-2 py-1 rounded font-mono">CURL_RUNNER_OUTPUT</code>
                      <span className="text-muted-foreground">output file path</span>
                    </div>
                    <div className="flex items-center justify-between py-2">
                      <code className="bg-muted px-2 py-1 rounded font-mono">CURL_RUNNER_QUIET</code>
                      <span className="text-muted-foreground">suppress output (true/false)</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <CodeBlockServer language="bash" filename="Environment Variable Examples">
              {environmentExamples}
            </CodeBlockServer>
          </section>

          {/* Configuration File */}
          <section>
            <H2>Configuration File</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Create a <code>curl-runner.yaml</code> file in your project root to set default options.
            </p>

            <CodeBlockServer language="yaml" filename="curl-runner.yaml">
              {configFileExample}
            </CodeBlockServer>
          </section>

          {/* Output Format */}
          <section>
            <H2>Output Format</H2>
            <p className="text-muted-foreground text-lg mb-6">
              When using <code>--output</code>, results are saved in structured JSON format.
            </p>

            <CodeBlockServer language="json" filename="results.json">
              {outputFormatExamples}
            </CodeBlockServer>
          </section>

          {/* Option Precedence */}
          <section>
            <H2>Option Precedence</H2>
            <p className="text-muted-foreground text-lg mb-6">
              When options are specified in multiple ways, curl-runner follows this precedence order:
            </p>

            <div className="space-y-4">
              <div className="flex items-center gap-4 p-4 border rounded-lg">
                <Badge variant="default" className="text-sm font-bold">1</Badge>
                <div className="flex-1">
                  <h4 className="font-medium">CLI Options</h4>
                  <p className="text-sm text-muted-foreground">Command-line flags take highest precedence</p>
                </div>
                <code className="text-sm bg-muted px-2 py-1 rounded font-mono">--verbose --timeout 5000</code>
              </div>
              
              <div className="flex items-center gap-4 p-4 border rounded-lg">
                <Badge variant="secondary" className="text-sm font-bold">2</Badge>
                <div className="flex-1">
                  <h4 className="font-medium">Environment Variables</h4>
                  <p className="text-sm text-muted-foreground">Override defaults but yield to CLI flags</p>
                </div>
                <code className="text-sm bg-muted px-2 py-1 rounded font-mono">CURL_RUNNER_VERBOSE=true</code>
              </div>
              
              <div className="flex items-center gap-4 p-4 border rounded-lg">
                <Badge variant="outline" className="text-sm font-bold">3</Badge>
                <div className="flex-1">
                  <h4 className="font-medium">Configuration File</h4>
                  <p className="text-sm text-muted-foreground">Project-specific defaults from curl-runner.yaml</p>
                </div>
                <code className="text-sm bg-muted px-2 py-1 rounded font-mono">curl-runner.yaml</code>
              </div>
              
              <div className="flex items-center gap-4 p-4 border rounded-lg">
                <Badge variant="outline" className="text-sm font-bold">4</Badge>
                <div className="flex-1">
                  <h4 className="font-medium">Built-in Defaults</h4>
                  <p className="text-sm text-muted-foreground">Fallback values when nothing else is specified</p>
                </div>
                <code className="text-sm bg-muted px-2 py-1 rounded font-mono">sequential, timeout: 5000</code>
              </div>
            </div>
          </section>

          {/* Best Practices */}
          <section>
            <H2>Best Practices</H2>
            
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Recommended Practices</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Best practices for optimal usage
                    </p>
                    <div className="space-y-2 text-sm">
                      <div>• Use configuration files for team consistency</div>
                      <div>• Use <code className="bg-muted px-1.5 py-0.5 rounded font-mono">--verbose</code> during development</div>
                      <div>• Set appropriate timeouts for your APIs</div>
                      <div>• Use <code className="bg-muted px-1.5 py-0.5 rounded font-mono">--continue-on-error</code> for test suites</div>
                      <div>• Save results with <code className="bg-muted px-1.5 py-0.5 rounded font-mono">--output</code> for analysis</div>
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
                    <h4 className="font-medium mb-2">Important Cautions</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Things to be mindful of when using options
                    </p>
                    <div className="space-y-2 text-sm">
                      <div>• Parallel execution may overwhelm servers</div>
                      <div>• High retry counts can cause rate limiting</div>
                      <div>• Very low timeouts may cause false failures</div>
                      <div>• Verbose output can be overwhelming for large test suites</div>
                      <div>• Environment variables affect all executions</div>
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
  )
}