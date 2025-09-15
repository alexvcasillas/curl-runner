import {
  AlertTriangle,
  ArrowRight,
  CheckCircle,
  Clock,
  Database,
  FileText,
  Globe,
  Layers,
  Lock,
  Monitor,
  Settings,
} from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'Global Settings',
  description:
    'Configure global settings for curl-runner including execution behavior, default headers, timeouts, and shared variables across all requests.',
  keywords: [
    'curl-runner global settings',
    'global configuration',
    'execution settings',
    'default headers',
    'global timeouts',
    'shared variables',
    'global defaults',
    'configuration options',
    'execution behavior',
    'global parameters',
    'system configuration',
    'curl-runner settings',
  ],
  openGraph: {
    title: 'Global Settings | curl-runner API Reference',
    description:
      'Configure global settings for curl-runner including execution behavior, default headers, timeouts, and shared variables across all requests.',
    url: 'https://curl-runner.com/docs/api-reference/global-settings',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Global Settings | curl-runner API Reference',
    description: 'Learn how to configure global settings for curl-runner requests and execution.',
  },
  alternates: {
    canonical: 'https://curl-runner.com/docs/api-reference/global-settings',
  },
};

const globalConfigExample = `# Complete Global Configuration Example
global:
  # Execution behavior
  execution: sequential
  continueOnError: false
  
  # Global variables available to all requests
  variables:
    BASE_URL: "https://api.example.com"
    API_VERSION: "v2"
    TIMEOUT_MS: "5000"
    USER_AGENT: "curl-runner/1.0.0"
    
  # Output configuration
  output:
    verbose: true
    showHeaders: true
    showBody: true
    showMetrics: true
    format: pretty
    prettyLevel: standard  # minimal, standard, or detailed
    saveToFile: "results/api-test-results.json"
    
  # Default settings applied to all requests
  defaults:
    timeout: \${TIMEOUT_MS}
    headers:
      User-Agent: \${USER_AGENT}
      Accept: "application/json"
      X-API-Version: \${API_VERSION}
    followRedirects: true
    maxRedirects: 3
    retry:
      count: 2
      delay: 1000
    expect:
      headers:
        content-type: "application/json"`;

const executionModesExample = `# Execution Mode Examples

# Sequential execution (default)
global:
  execution: sequential  # Requests run one after another
  continueOnError: false # Stop on first error
  
collection:
  name: "Sequential API Tests"
  requests:
    - name: "Step 1: Login"
      url: "https://api.example.com/login"
      method: POST
      # Must complete before next request
      
    - name: "Step 2: Get Data"  
      url: "https://api.example.com/data"
      method: GET
      # Uses login token from previous request
      
    - name: "Step 3: Process"
      url: "https://api.example.com/process"
      method: POST
      # Depends on data from step 2

---

# Parallel execution for performance
global:
  execution: parallel    # All requests run simultaneously
  continueOnError: true  # Continue even if some fail
  
collection:
  name: "Load Testing"
  requests:
    - name: "Load Test 1"
      url: "https://api.example.com/endpoint1"
      
    - name: "Load Test 2"
      url: "https://api.example.com/endpoint2"
      
    - name: "Load Test 3"
      url: "https://api.example.com/endpoint3"
      # All three run at the same time`;

const errorHandlingExample = `# Error Handling Configuration

# Stop on first error (strict mode)
global:
  continueOnError: false  # Default behavior
  execution: sequential
  
collection:
  requests:
    - name: "Critical Setup"
      url: "https://api.example.com/setup"
      method: POST
      # If this fails, execution stops
      
    - name: "Dependent Operation"
      url: "https://api.example.com/operation"
      method: POST
      # Only runs if setup succeeds

---

# Continue on error (resilient mode)  
global:
  continueOnError: true   # Keep going despite failures
  execution: sequential
  
collection:
  requests:
    - name: "Optional Check 1"
      url: "https://api.example.com/check1"
      # If fails, continue to next
      
    - name: "Optional Check 2"
      url: "https://api.example.com/check2"
      # Runs regardless of check1 result
      
    - name: "Important Operation"
      url: "https://api.example.com/important"
      # Runs regardless of previous failures`;

const variablesExample = `# Global Variables Configuration

global:
  variables:
    # Environment configuration
    BASE_URL: "https://api.production.com"
    API_VERSION: "v2"
    ENVIRONMENT: "production"
    
    # Authentication
    API_KEY: "\${API_KEY}"        # From environment
    CLIENT_ID: "\${CLIENT_ID}"    # From environment
    
    # Timeouts and limits
    DEFAULT_TIMEOUT: "5000"
    MAX_RETRIES: "3"
    RATE_LIMIT: "100"
    
    # Common values
    CONTENT_TYPE: "application/json"
    USER_AGENT: "MyApp/1.0.0 (production)"
    
    # Dynamic values (can be overridden)
    REQUEST_ID: "req_\${TIMESTAMP}"
    TRACE_ID: "trace_\${UUID}"

collection:
  variables:
    # Override global variables at collection level
    ENVIRONMENT: "staging"  # Override global
    COLLECTION_ID: "api_tests_v2"
    
  requests:
    - name: "Use Global Variables"
      url: "\${BASE_URL}/\${API_VERSION}/users"
      headers:
        Authorization: "Bearer \${API_KEY}"
        User-Agent: "\${USER_AGENT}"
        X-Environment: "\${ENVIRONMENT}"
        X-Request-ID: "\${REQUEST_ID}"
      timeout: \${DEFAULT_TIMEOUT}
      
    - name: "Override Variables"
      url: "\${BASE_URL}/\${API_VERSION}/slow-endpoint"
      variables:
        # Local override for this request only
        DEFAULT_TIMEOUT: "30000"  # 30 seconds for slow endpoint
      timeout: \${DEFAULT_TIMEOUT}`;

const outputConfigurationExample = `# Output Configuration Options

# Minimal output - only basic status
global:
  output:
    format: pretty
    prettyLevel: minimal  # Only show status and errors
    showHeaders: false
    showBody: false
    showMetrics: false

---

# Standard output - body and metrics when enabled
global:
  output:
    format: pretty
    prettyLevel: standard  # Show body/metrics if enabled
    showHeaders: false
    showBody: true        # Will show body with standard level
    showMetrics: true     # Will show basic metrics

---

# Detailed output - everything visible
global:
  output:
    format: pretty
    prettyLevel: detailed # Always show all information
    # prettyLevel: detailed overrides individual flags

---

# JSON output for CI/CD
global:
  output:
    format: json      # Structured JSON output
    showHeaders: true
    showBody: true
    showMetrics: true
    saveToFile: "test-results/\${DATE}-api-tests.json"

---

# Raw output for data processing
global:
  output:
    format: raw       # Raw response content only
    showHeaders: false
    showBody: true
    saveToFile: "data.txt"`;

const defaultsExample = `# Default Request Settings

global:
  defaults:
    # Timeouts and retries
    timeout: 10000        # 10 seconds default
    followRedirects: true
    maxRedirects: 5
    retry:
      count: 3
      delay: 2000
      
    # Common headers for all requests
    headers:
      User-Agent: "curl-runner/1.0.0"
      Accept: "application/json"
      Content-Type: "application/json"
      X-Client-Version: "1.2.3"
      
    # Security settings
    insecure: false       # Verify SSL certificates
    
    # Response validation
    expect:
      headers:
        content-type: "application/json"  # Expect JSON responses
        
collection:
  name: "API Tests with Defaults"
  
  # Collection-level defaults override global
  defaults:
    headers:
      Authorization: "Bearer \${API_TOKEN}"  # Add auth to all
      X-Collection: "api-tests"
    timeout: 15000  # Override global timeout
    
  requests:
    - name: "Quick Request"
      url: "https://api.example.com/quick"
      # Uses all defaults from global and collection
      
    - name: "Custom Request"
      url: "https://api.example.com/custom"
      timeout: 30000  # Override timeout just for this request
      headers:
        X-Special: "custom-header"  # Adds to default headers
        Authorization: "Basic xyz"  # Overrides collection default
      
    - name: "Slow Request"
      url: "https://api.example.com/slow"
      retry:
        count: 5      # Override default retry count
        delay: 5000   # Override default retry delay`;

const complexConfigExample = `# Complex Production Configuration

global:
  # Execution settings
  execution: sequential
  continueOnError: false
  
  # Environment variables
  variables:
    # Load balancer endpoints
    API_PRIMARY: "https://api-primary.company.com"
    API_BACKUP: "https://api-backup.company.com"
    
    # Authentication
    SERVICE_ACCOUNT_TOKEN: "\${SERVICE_TOKEN}"
    CLIENT_CERT_PATH: "/etc/ssl/certs/client.pem"
    
    # Rate limiting
    REQUESTS_PER_MINUTE: "60"
    BURST_LIMIT: "10"
    
    # Monitoring
    TRACE_ENDPOINT: "https://tracing.company.com/v1/traces"
    LOG_LEVEL: "info"
    
  # Output for CI/CD integration
  output:
    verbose: true
    showHeaders: false    # Reduce log noise
    showBody: false      # Don't log sensitive data
    showMetrics: true    # Track performance
    format: json         # Machine-readable
    saveToFile: "artifacts/api-test-results.json"
    
  # Production defaults
  defaults:
    timeout: 30000       # 30 seconds for production APIs
    followRedirects: true
    maxRedirects: 3
    insecure: false      # Always verify certificates
    
    # Standard headers
    headers:
      User-Agent: "curl-runner-ci/1.0.0"
      Accept: "application/json"
      X-Environment: "production"
      X-Service: "api-tests"
      
    # Retry configuration for reliability
    retry:
      count: 3
      delay: 5000        # 5 second delays
      
    # Basic validation
    expect:
      headers:
        content-type: "application/json"
        x-rate-limit-remaining: "*"  # Ensure rate limits tracked
        
collection:
  name: "Production Health Checks"
  description: "Critical API endpoints monitoring"
  
  variables:
    HEALTH_CHECK_VERSION: "v1"
    ALERT_WEBHOOK: "\${SLACK_WEBHOOK_URL}"
    
  defaults:
    headers:
      Authorization: "Bearer \${SERVICE_ACCOUNT_TOKEN}"
      X-Health-Check: "automated"
      
  requests:
    - name: "Primary API Health"
      url: "\${API_PRIMARY}/\${HEALTH_CHECK_VERSION}/health"
      method: GET
      expect:
        status: 200
        body:
          status: "healthy"
          version: "*"
          timestamp: "*"
          
    - name: "Database Connection Check"
      url: "\${API_PRIMARY}/\${HEALTH_CHECK_VERSION}/health/database"
      method: GET
      timeout: 60000     # Database checks can be slow
      expect:
        status: 200
        body:
          database:
            status: "connected"
            response_time: "^[0-9]+ms$"
            
    - name: "Cache Health Check"  
      url: "\${API_PRIMARY}/\${HEALTH_CHECK_VERSION}/health/cache"
      method: GET
      expect:
        status: 200
        body:
          cache:
            status: "operational"
            hit_rate: "*"
            
    - name: "External Service Dependencies"
      url: "\${API_PRIMARY}/\${HEALTH_CHECK_VERSION}/health/dependencies"
      method: GET
      timeout: 45000     # External services may be slow
      expect:
        status: [200, 207]  # 207 for partial failures
        body:
          dependencies:
            - name: "*"
              status: ["healthy", "degraded"]`;

export default function GlobalSettingsPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Global Settings API Reference"
          text="Complete reference for global configuration options that control execution behavior, output formatting, and default request settings."
        />

        <div className="space-y-8">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground mb-6">
              Global settings control the overall behavior of{' '}
              <code className="font-mono">curl-runner</code> execution, including how requests are
              processed, output formatting, default values, and variable management. These settings
              apply to all requests unless overridden at the collection or request level.
            </p>

            <CodeBlockServer language="yaml" filename="global-config-complete.yaml">
              {globalConfigExample}
            </CodeBlockServer>
          </section>

          {/* Properties Table */}
          <section>
            <H2 id="properties">Global Configuration Properties</H2>
            <div className="border rounded-lg overflow-hidden">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Property</th>
                    <th className="text-left p-3 font-medium">Type</th>
                    <th className="text-left p-3 font-medium">Default</th>
                    <th className="text-left p-3 font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">execution</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">'sequential' | 'parallel'</td>
                    <td className="p-3 text-sm text-muted-foreground">'sequential'</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      How to execute multiple requests
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">continueOnError</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">boolean</td>
                    <td className="p-3 text-sm text-muted-foreground">false</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Whether to continue execution after errors
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">variables</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Record&lt;string, string&gt;
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">{}</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Global variables for all requests
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">output</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">OutputConfig</td>
                    <td className="p-3 text-sm text-muted-foreground">-</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Output formatting configuration
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">defaults</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Partial&lt;RequestConfig&gt;
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">{}</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Default settings for all requests
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Execution Modes */}
          <section>
            <H2 id="execution-modes">Execution Modes</H2>
            <p className="text-muted-foreground mb-6">
              Control how multiple requests are executed: sequentially (one after another) or in
              parallel (simultaneously).
            </p>

            <CodeBlockServer language="yaml" filename="execution-modes.yaml">
              {executionModesExample}
            </CodeBlockServer>

            <div className="mt-6 grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <ArrowRight className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      Sequential <Badge variant="default">Default</Badge>
                    </h4>
                    <div className="text-sm text-muted-foreground space-y-2">
                      <p>
                        <strong>Use for:</strong> Dependent requests, workflows, login sequences
                      </p>
                      <p>
                        <strong>Benefits:</strong> Predictable order, can use previous responses
                      </p>
                      <p>
                        <strong>Drawbacks:</strong> Slower total execution time
                      </p>
                    </div>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Layers className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      Parallel <Badge variant="secondary">Performance</Badge>
                    </h4>
                    <div className="text-sm text-muted-foreground space-y-2">
                      <p>
                        <strong>Use for:</strong> Load testing, independent health checks
                      </p>
                      <p>
                        <strong>Benefits:</strong> Faster execution, higher throughput
                      </p>
                      <p>
                        <strong>Drawbacks:</strong> No request dependencies, resource intensive
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Error Handling */}
          <section>
            <H2 id="error-handling">Error Handling Configuration</H2>
            <p className="text-muted-foreground mb-6">
              Configure how <code className="font-mono">curl-runner</code> responds to request
              failures and validation errors.
            </p>

            <CodeBlockServer language="yaml" filename="error-handling.yaml">
              {errorHandlingExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="error-strategies">Error Handling Strategies</H3>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-red-500/10 p-2">
                      <AlertTriangle className="h-4 w-4 text-red-600 dark:text-red-400" />
                    </div>
                    <div>
                      <div className="flex items-center gap-2 mb-2">
                        <h4 className="font-medium">Fail-Fast Mode</h4>
                        <Badge variant="destructive">continueOnError: false</Badge>
                      </div>
                      <p className="text-sm text-muted-foreground">
                        Stop execution on first error. Best for critical workflows.
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-green-500/10 p-2">
                      <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    <div>
                      <div className="flex items-center gap-2 mb-2">
                        <h4 className="font-medium">Resilient Mode</h4>
                        <Badge variant="secondary">continueOnError: true</Badge>
                      </div>
                      <p className="text-sm text-muted-foreground">
                        Continue despite errors. Best for comprehensive testing.
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Global Variables */}
          <section>
            <H2 id="global-variables">Global Variables</H2>
            <p className="text-muted-foreground mb-6">
              Define variables that are available to all requests in the configuration. Variables
              support environment variable interpolation and can be overridden at collection and
              request levels.
            </p>

            <CodeBlockServer language="yaml" filename="global-variables.yaml">
              {variablesExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-blue-500/5 dark:bg-blue-500/10 border-blue-500/20 p-4">
              <p className="text-sm">
                <strong className="text-blue-600 dark:text-blue-400">Variable Precedence:</strong>{' '}
                Request-level variables override collection variables, which override global
                variables. Use{' '}
                <code>
                  {'$'}
                  {'{VAR_NAME}'}
                </code>{' '}
                syntax to reference environment variables.
              </p>
            </div>
          </section>

          {/* Output Configuration */}
          <section>
            <H2 id="output-configuration">Output Configuration</H2>
            <p className="text-muted-foreground mb-6">
              Control how results are displayed and saved, including verbosity, format, and file
              output options.
            </p>

            <CodeBlockServer language="yaml" filename="output-config.yaml">
              {outputConfigurationExample}
            </CodeBlockServer>

            <div className="mt-6 border rounded-lg overflow-hidden">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Output Option</th>
                    <th className="text-left p-3 font-medium">Type</th>
                    <th className="text-left p-3 font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">verbose</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">boolean</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Show detailed execution information
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">showHeaders</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">boolean</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Include response headers in output
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">showBody</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">boolean</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Include response body content
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">showMetrics</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">boolean</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Show performance timing metrics
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">format</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">'json' | 'pretty' | 'raw'</td>
                    <td className="p-3 text-sm text-muted-foreground">Output format style</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">prettyLevel</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      'minimal' | 'standard' | 'detailed'
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Pretty format verbosity level
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">saveToFile</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">string</td>
                    <td className="p-3 text-sm text-muted-foreground">File path to save results</td>
                  </tr>
                </tbody>
              </table>
            </div>

            <div className="mt-6 space-y-3">
              <H3 id="output-formats">Output Format Details</H3>
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <FileText className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">JSON Format</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Structured, machine-readable output
                      </p>
                      <Badge variant="outline">CI/CD Integration</Badge>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-green-500/10 p-2">
                      <Monitor className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Pretty Format</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Human-readable, colorized output with configurable verbosity levels
                      </p>
                      <Badge variant="outline">Development</Badge>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-orange-500/10 p-2">
                      <Database className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2">Raw Format</h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Raw response content only
                      </p>
                      <Badge variant="outline">Data Extraction</Badge>
                    </div>
                  </div>
                </div>
              </div>

              <H3 id="pretty-levels">Pretty Format Levels</H3>
              <p className="text-muted-foreground mb-4">
                The pretty format supports three verbosity levels to control how much information is
                displayed.
              </p>
              <div className="grid gap-4 md:grid-cols-3">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-gray-500/10 p-2">
                      <Monitor className="h-4 w-4 text-gray-600 dark:text-gray-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2 flex items-center gap-2">
                        Minimal <Badge variant="secondary">Quiet</Badge>
                      </h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Shows only request name, method, URL, and status. No body, headers, or
                        metrics.
                      </p>
                      <div className="text-xs text-muted-foreground">
                        <strong>Use for:</strong> Quick status checks, CI/CD summary
                      </div>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-blue-500/10 p-2">
                      <Monitor className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2 flex items-center gap-2">
                        Standard <Badge variant="default">Default</Badge>
                      </h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Shows body and metrics when explicitly enabled via showBody/showMetrics
                        flags.
                      </p>
                      <div className="text-xs text-muted-foreground">
                        <strong>Use for:</strong> Development, testing, debugging
                      </div>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-green-500/10 p-2">
                      <Monitor className="h-4 w-4 text-green-600 dark:text-green-400" />
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium mb-2 flex items-center gap-2">
                        Detailed <Badge variant="outline">Verbose</Badge>
                      </h4>
                      <p className="text-sm text-muted-foreground mb-3">
                        Shows all available information: headers, body, detailed metrics, request
                        details, and commands.
                      </p>
                      <div className="text-xs text-muted-foreground">
                        <strong>Use for:</strong> Troubleshooting, detailed analysis
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Default Settings */}
          <section>
            <H2 id="default-settings">Default Request Settings</H2>
            <p className="text-muted-foreground mb-6">
              Configure default values that apply to all requests, reducing repetition and ensuring
              consistency across your API configurations.
            </p>

            <CodeBlockServer language="yaml" filename="defaults-config.yaml">
              {defaultsExample}
            </CodeBlockServer>

            <div className="mt-6 space-y-3">
              <H3 id="default-precedence">Setting Precedence Order</H3>
              <div className="space-y-3">
                <div className="rounded-lg border p-4">
                  <h4 className="font-medium mb-2">1. Request-Level Settings</h4>
                  <p className="text-sm text-muted-foreground">
                    Values defined directly on individual requests have the highest priority and
                    override all defaults.
                  </p>
                </div>

                <div className="rounded-lg border p-4">
                  <h4 className="font-medium mb-2">2. Collection-Level Defaults</h4>
                  <p className="text-sm text-muted-foreground">
                    Collection defaults override global defaults but are overridden by request-level
                    settings.
                  </p>
                </div>

                <div className="rounded-lg border p-4">
                  <h4 className="font-medium mb-2">3. Global Defaults</h4>
                  <p className="text-sm text-muted-foreground">
                    Global defaults have the lowest priority and provide fallback values for
                    unspecified settings.
                  </p>
                </div>
              </div>
            </div>
          </section>

          {/* Complex Production Example */}
          <section>
            <H2 id="production-example">Production Configuration Example</H2>
            <p className="text-muted-foreground mb-6">
              A comprehensive example showing how to configure{' '}
              <code className="font-mono">curl-runner</code> for production monitoring and CI/CD
              integration.
            </p>

            <CodeBlockServer language="yaml" filename="production-config.yaml">
              {complexConfigExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-green-500/5 dark:bg-green-500/10 border-green-500/20 p-4">
              <p className="text-sm">
                <strong className="text-green-600 dark:text-green-400">Production Ready:</strong>{' '}
                This configuration demonstrates enterprise-level settings including proper
                authentication, monitoring, error handling, and CI/CD integration patterns.
              </p>
            </div>
          </section>

          {/* Environment Integration */}
          <section>
            <H2 id="environment-integration">Environment Integration</H2>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Settings className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Environment Variables</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Reference environment variables using{' '}
                      <code>
                        {'$'}
                        {'{VAR_NAME}'}
                      </code>{' '}
                      syntax. <code className="font-mono">curl-runner</code> automatically loads
                      from <code>.env</code> files.
                    </p>
                    <CodeBlockServer language="bash">
                      {`# .env file
API_TOKEN=your_secret_token
BASE_URL=https://api.staging.com
DEBUG_MODE=true`}
                    </CodeBlockServer>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <FileText className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Configuration Files</h4>
                    <p className="text-sm text-muted-foreground mb-3">
                      Use different configuration files for different environments by organizing
                      global settings appropriately.
                    </p>
                    <CodeBlockServer language="bash">
                      {`# Run with specific environment
curl-runner config/production.yaml
curl-runner config/staging.yaml
curl-runner config/development.yaml`}
                    </CodeBlockServer>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Best Practices */}
          <section>
            <H2 id="best-practices">Global Configuration Best Practices</H2>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Globe className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Environment Separation</h4>
                    <p className="text-sm text-muted-foreground">
                      Use different global configurations for development, staging, and production
                      environments to avoid accidental cross-environment requests.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-red-500/10 p-2">
                    <Lock className="h-4 w-4 text-red-600 dark:text-red-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Secure Credentials</h4>
                    <p className="text-sm text-muted-foreground">
                      Never hard-code credentials in configuration files. Always use environment
                      variables or secure credential management systems.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <Clock className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Appropriate Timeouts</h4>
                    <p className="text-sm text-muted-foreground">
                      Set reasonable default timeouts based on your API characteristics. Too short
                      causes unnecessary failures, too long delays error detection.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Globe className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Consistent Headers</h4>
                    <p className="text-sm text-muted-foreground">
                      Use global defaults for common headers like User-Agent, Accept, and API
                      versioning to ensure consistency across all requests.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4 md:col-span-2">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Monitor className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Output Configuration</h4>
                    <p className="text-sm text-muted-foreground">
                      Configure output formats appropriate for your use case: pretty for
                      development, JSON for CI/CD, raw for data processing.
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
