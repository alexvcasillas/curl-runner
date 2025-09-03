import { AlertTriangle, Clock, FileText, Settings, Target, Zap } from 'lucide-react';
import { CodeBlockServer } from '@/components/code-block-server';
import { DocsPageHeader } from '@/components/docs-page-header';
import { H2 } from '@/components/mdx-heading';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

const basicGlobalExample = `# Basic global configuration
global:
  # Execution settings
  execution: sequential
  continueOnError: false
  
  # Timeout settings
  timeout: 5000
  retries: 3
  
  # Output settings
  output:
    verbose: true
    saveToFile: "results.json"
    
  # Default request settings
  defaults:
    headers:
      User-Agent: "curl-runner/1.0.0"
      Accept: "application/json"
    timeout: 10000

requests:
  - name: Example Request
    url: https://api.example.com/data
    method: GET`;

const executionModeExample = `# Sequential execution (default)
global:
  execution: sequential  # Requests run one after another
  
# Parallel execution  
global:
  execution: parallel   # All requests run simultaneously
  
# With error handling
global:
  execution: parallel
  continueOnError: true  # Don't stop if some requests fail`;

const timeoutRetryExample = `# Global timeout and retry settings
global:
  # Global timeout for all requests (milliseconds)
  timeout: 10000
  
  # Maximum retries for failed requests
  retries: 3
  
  # Retry delay settings
  retryDelay: 1000        # Initial delay between retries (ms)
  retryBackoff: 2.0       # Exponential backoff multiplier
  
  # Individual request can override these
  defaults:
    timeout: 5000         # Default per-request timeout
    retries: 1            # Default per-request retries

requests:
  - name: Standard Request
    url: https://api.example.com/data
    method: GET
    # Uses global defaults
    
  - name: Custom Timeout Request
    url: https://slow-api.example.com/data
    method: GET
    timeout: 30000        # Overrides global default
    retries: 5            # Overrides global default`;

const outputConfigExample = `# Output configuration options
global:
  output:
    # Verbose output with request/response details
    verbose: true
    
    # Save results to JSON file
    saveToFile: "test-results.json"
    
    # Include response bodies in output
    includeResponseBody: true
    
    # Include request details in output
    includeRequestDetails: true
    
    # Format for console output
    format: "detailed"  # Options: "minimal", "standard", "detailed"
    
    # Colorize console output
    colors: true
    
    # Show progress indicators
    showProgress: true
    
    # Timestamp format
    timestampFormat: "ISO"  # Options: "ISO", "unix", "relative"`;

const defaultsExample = `# Global defaults applied to all requests
global:
  defaults:
    # Default headers for all requests
    headers:
      User-Agent: "MyApp/1.0.0"
      Accept: "application/json"
      Content-Type: "application/json"
      X-Client-Version: "1.2.3"
      
    # Default timeout and retry settings
    timeout: 8000
    retries: 2
    
    # Default validation rules
    validation:
      statusCodes: [200, 201, 202]
      maxResponseTime: 5000
      
    # Default query parameters
    query:
      api_version: "2023-01-01"
      format: "json"

requests:
  - name: Inherits All Defaults
    url: https://api.example.com/users
    method: GET
    # All global defaults are applied
    
  - name: Overrides Some Defaults
    url: https://api.example.com/posts
    method: POST
    headers:
      Content-Type: "application/xml"  # Overrides default
      # Other headers from defaults still apply
    timeout: 15000  # Overrides default timeout`;

const variablesExample = `# Global variables for reuse across requests
global:
  variables:
    # API configuration
    BASE_URL: https://api.example.com
    API_VERSION: v1
    API_KEY: your-secret-api-key
    
    # Environment-specific variables
    ENVIRONMENT: \${ENV.NODE_ENV || 'development'}
    DEBUG_MODE: \${ENVIRONMENT === 'development'}
    
    # Computed variables
    API_ENDPOINT: "\${BASE_URL}/\${API_VERSION}"
    AUTH_HEADER: "Bearer \${API_KEY}"
    TIMEOUT: \${DEBUG_MODE ? 30000 : 5000}
    
  defaults:
    headers:
      Authorization: \${AUTH_HEADER}
    timeout: \${TIMEOUT}

requests:
  - name: Get Users
    url: \${API_ENDPOINT}/users
    method: GET
    
  - name: Create User
    url: \${API_ENDPOINT}/users
    method: POST
    body:
      name: "John Doe"
      environment: \${ENVIRONMENT}`;

const advancedConfigExample = `# Advanced global configuration
global:
  # Execution control
  execution: parallel
  continueOnError: true
  maxConcurrency: 5  # Limit concurrent requests in parallel mode
  
  # Advanced timeout settings
  timeout: 10000
  connectionTimeout: 5000
  readTimeout: 15000
  retries: 3
  retryDelay: 1000
  retryBackoff: 2.0
  retryOn: ["timeout", "5xx", "network"]
  
  # SSL/TLS settings
  ssl:
    verify: true
    ca: "./certs/ca.pem"
    cert: "./certs/client.pem"
    key: "./certs/client-key.pem"
  
  # Proxy settings
  proxy:
    http: "http://proxy.company.com:8080"
    https: "https://secure-proxy.company.com:8443"
    bypass: ["localhost", "*.internal.com"]
    
  # Rate limiting
  rateLimit:
    maxRequests: 100
    perSecond: 10
    
  # Output configuration
  output:
    verbose: true
    saveToFile: "results-\${Date.now()}.json"
    format: "detailed"
    colors: true
    includeMetrics: true
    
  # Global variables
  variables:
    BASE_URL: \${ENV.API_BASE_URL || 'https://api.example.com'}
    API_KEY: \${ENV.API_KEY}
    TRACE_ID: \${crypto.randomUUID()}
    
  # Global defaults
  defaults:
    headers:
      User-Agent: "curl-runner/1.0.0"
      X-Trace-ID: \${TRACE_ID}
      Authorization: "Bearer \${API_KEY}"
    timeout: 8000
    retries: 2`;

const overrideExample = `# How individual requests override global settings
global:
  execution: sequential
  timeout: 5000
  retries: 2
  defaults:
    headers:
      Accept: "application/json"
      User-Agent: "curl-runner/1.0.0"

requests:
  - name: Uses Global Settings
    url: https://api.example.com/users
    method: GET
    # Inherits: timeout=5000, retries=2, headers from defaults
    
  - name: Overrides Global Settings  
    url: https://slow-api.example.com/data
    method: GET
    timeout: 30000     # Overrides global timeout
    retries: 5         # Overrides global retries
    headers:
      Accept: "application/xml"  # Overrides global default
      User-Agent: "MyApp/2.0.0"  # Overrides global default
      Custom-Header: "value"     # Adds new header`;

export default function GlobalSettingsPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Global Settings"
          text="Configure global execution settings, defaults, and behaviors that apply to all requests in your YAML files."
        />

        <div className="space-y-12">
          {/* Introduction */}
          <section>
            <p className="text-lg text-muted-foreground mb-6">
              Global settings allow you to configure default behavior for all requests in a YAML
              file. These settings can be overridden by individual requests when needed.
            </p>

            <CodeBlockServer language="yaml" filename="basic-global-config.yaml">
              {basicGlobalExample}
            </CodeBlockServer>
          </section>

          {/* Execution Settings */}
          <section>
            <H2>Execution Settings</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Control how requests are executed and how errors are handled.
            </p>

            <div className="grid gap-6 sm:grid-cols-2 mb-6">
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center space-x-2">
                    <FileText className="h-5 w-5 text-blue-500" />
                    <span>Sequential Mode</span>
                    <Badge variant="secondary">Default</Badge>
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <CardDescription>
                    Requests execute one after another in the order they appear. Useful for
                    dependent requests or when order matters.
                  </CardDescription>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center space-x-2">
                    <Zap className="h-5 w-5 text-green-500" />
                    <span>Parallel Mode</span>
                    <Badge variant="default">Fast</Badge>
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <CardDescription>
                    All requests execute simultaneously for maximum speed. Best for independent
                    requests that don't depend on each other.
                  </CardDescription>
                </CardContent>
              </Card>
            </div>

            <CodeBlockServer language="yaml" filename="execution-modes.yaml">
              {executionModeExample}
            </CodeBlockServer>
          </section>

          {/* Timeout and Retry Settings */}
          <section>
            <H2>Timeout & Retry Settings</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Configure global timeout and retry behavior for robust request handling.
            </p>

            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4 mb-6">
              <Card>
                <CardHeader className="pb-2">
                  <CardTitle className="text-base flex items-center space-x-2">
                    <Clock className="h-4 w-4 text-blue-500" />
                    <span>Timeout</span>
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <p className="text-sm text-muted-foreground">
                    Maximum time to wait for a request to complete (milliseconds)
                  </p>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="pb-2">
                  <CardTitle className="text-base flex items-center space-x-2">
                    <Target className="h-4 w-4 text-green-500" />
                    <span>Retries</span>
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <p className="text-sm text-muted-foreground">
                    Number of retry attempts for failed requests
                  </p>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="pb-2">
                  <CardTitle className="text-base flex items-center space-x-2">
                    <Clock className="h-4 w-4 text-purple-500" />
                    <span>Retry Delay</span>
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <p className="text-sm text-muted-foreground">
                    Wait time between retry attempts (milliseconds)
                  </p>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="pb-2">
                  <CardTitle className="text-base flex items-center space-x-2">
                    <Zap className="h-4 w-4 text-orange-500" />
                    <span>Backoff</span>
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <p className="text-sm text-muted-foreground">
                    Exponential backoff multiplier for retry delays
                  </p>
                </CardContent>
              </Card>
            </div>

            <CodeBlockServer language="yaml" filename="timeout-retry-config.yaml">
              {timeoutRetryExample}
            </CodeBlockServer>
          </section>

          {/* Output Configuration */}
          <section>
            <H2>Output Configuration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Customize how <code className="font-mono">curl-runner</code> displays results and saves output data.
            </p>

            <CodeBlockServer language="yaml" filename="output-config.yaml">
              {outputConfigExample}
            </CodeBlockServer>
          </section>

          {/* Global Defaults */}
          <section>
            <H2>Global Defaults</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Set default values that are automatically applied to all requests unless overridden.
            </p>

            <div className="grid gap-4 sm:grid-cols-2 mb-6">
              <Card>
                <CardHeader className="pb-2">
                  <CardTitle className="text-base">Common Defaults</CardTitle>
                </CardHeader>
                <CardContent className="space-y-2 text-sm">
                  <div>
                    • <strong>Headers:</strong> Authentication, content types, user agents
                  </div>
                  <div>
                    • <strong>Timeouts:</strong> Request and connection timeouts
                  </div>
                  <div>
                    • <strong>Validation:</strong> Expected status codes, response times
                  </div>
                  <div>
                    • <strong>Query Params:</strong> API versions, formats
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="pb-2">
                  <CardTitle className="text-base">Override Behavior</CardTitle>
                </CardHeader>
                <CardContent className="space-y-2 text-sm">
                  <div>• Individual requests can override any default</div>
                  <div>• Headers are merged (new ones added, existing ones replaced)</div>
                  <div>• Scalar values (timeout, retries) are completely replaced</div>
                  <div>• Arrays and objects are merged by default</div>
                </CardContent>
              </Card>
            </div>

            <CodeBlockServer language="yaml" filename="global-defaults.yaml">
              {defaultsExample}
            </CodeBlockServer>
          </section>

          {/* Global Variables */}
          <section>
            <H2>Global Variables</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Define reusable variables that can be referenced throughout your request
              configurations.
            </p>

            <CodeBlockServer language="yaml" filename="global-variables.yaml">
              {variablesExample}
            </CodeBlockServer>
          </section>

          {/* Advanced Configuration */}
          <section>
            <H2>Advanced Configuration</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Advanced global settings for complex scenarios including SSL, proxies, and rate
              limiting.
            </p>

            <Card className="border-amber-200 bg-amber-50/50 dark:border-amber-800 dark:bg-amber-950/50 mb-6">
              <CardHeader>
                <CardTitle className="flex items-center space-x-2 text-amber-700 dark:text-amber-300">
                  <AlertTriangle className="h-5 w-5" />
                  <span>Advanced Features</span>
                </CardTitle>
              </CardHeader>
              <CardContent className="text-amber-700 dark:text-amber-300">
                <p>
                  Some advanced features like SSL certificates, proxies, and rate limiting may
                  require additional setup or may not be available in all environments.
                </p>
              </CardContent>
            </Card>

            <CodeBlockServer language="yaml" filename="advanced-global-config.yaml">
              {advancedConfigExample}
            </CodeBlockServer>
          </section>

          {/* Setting Precedence */}
          <section>
            <H2>Setting Precedence</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Understanding how individual request settings override global settings.
            </p>

            <Card className="mb-6">
              <CardHeader>
                <CardTitle>Override Rules</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                <div className="flex items-center space-x-3">
                  <Badge variant="default">1</Badge>
                  <span className="font-medium">Individual Request Settings</span>
                  <span className="text-sm text-muted-foreground">Always take precedence</span>
                </div>
                <div className="flex items-center space-x-3">
                  <Badge variant="secondary">2</Badge>
                  <span className="font-medium">Collection-Level Defaults</span>
                  <span className="text-sm text-muted-foreground">Override global defaults</span>
                </div>
                <div className="flex items-center space-x-3">
                  <Badge variant="outline">3</Badge>
                  <span className="font-medium">Global Defaults</span>
                  <span className="text-sm text-muted-foreground">Apply when not overridden</span>
                </div>
              </CardContent>
            </Card>

            <CodeBlockServer language="yaml" filename="setting-precedence.yaml">
              {overrideExample}
            </CodeBlockServer>
          </section>

          {/* Best Practices */}
          <section>
            <H2>Best Practices</H2>

            <div className="grid gap-6 sm:grid-cols-1 md:grid-cols-2">
              <Card className="border-green-200 bg-green-50/50 dark:border-green-800 dark:bg-green-950/50">
                <CardHeader>
                  <CardTitle className="flex items-center space-x-2 text-green-700 dark:text-green-300">
                    <Settings className="h-5 w-5" />
                    <span>Recommended Practices</span>
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-2 text-sm text-green-700 dark:text-green-300">
                  <p>• Set reasonable global timeouts (5-10 seconds)</p>
                  <p>• Use environment variables for sensitive data</p>
                  <p>• Group related settings logically</p>
                  <p>• Define common headers in global defaults</p>
                  <p>• Use descriptive variable names</p>
                  <p>• Document complex configurations</p>
                </CardContent>
              </Card>

              <Card className="border-amber-200 bg-amber-50/50 dark:border-amber-800 dark:bg-amber-950/50">
                <CardHeader>
                  <CardTitle className="flex items-center space-x-2 text-amber-700 dark:text-amber-300">
                    <AlertTriangle className="h-5 w-5" />
                    <span>Common Pitfalls</span>
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-2 text-sm text-amber-700 dark:text-amber-300">
                  <p>• Don't set timeouts too low (causes false failures)</p>
                  <p>• Avoid too many concurrent requests in parallel mode</p>
                  <p>• Don't hard-code sensitive information</p>
                  <p>• Be careful with global retry settings</p>
                  <p>• Test configurations thoroughly</p>
                  <p>• Consider server rate limits</p>
                </CardContent>
              </Card>
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
