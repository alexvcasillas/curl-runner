import type { LucideIcon } from 'lucide-react';
import {
  ArrowLeftRight,
  Camera,
  CheckCircle,
  Database,
  Diff,
  Eye,
  FileJson,
  Gauge,
  GitBranch,
  Layers,
  Lock,
  Network,
  RotateCcw,
  ShieldCheck,
  Terminal,
  TrendingUp,
  Upload,
  Wand2,
  Zap,
} from 'lucide-react';

export interface FeatureData {
  slug: string;
  title: string;
  shortDescription: string;
  description: string;
  icon: LucideIcon;
  color: string;
  gradient: string;
  /** Number of trailing words in shortDescription to apply the gradient to. Default: 2 */
  gradientWords?: number;
  benefits: Array<{
    title: string;
    description: string;
  }>;
  codeExample: {
    title: string;
    code: string;
  };
  useCases: string[];
  keywords: string[];
}

export const featuresData: Record<string, FeatureData> = {
  'parallel-execution': {
    slug: 'parallel-execution',
    title: 'Parallel Execution',
    shortDescription: 'Execute multiple HTTP requests simultaneously',
    description:
      'Run multiple HTTP requests in parallel for dramatically faster test execution. Perfect for load testing, concurrent API validation, and performance optimization.',
    icon: Gauge,
    color: 'cyan',
    gradient: 'from-cyan-400 to-cyan-600',
    benefits: [
      {
        title: 'Blazing Fast Performance',
        description:
          'Execute hundreds of requests simultaneously, reducing total execution time by up to 10x compared to sequential execution.',
      },
      {
        title: 'Concurrency Control',
        description:
          'Fine-tune parallel execution with maxConcurrency limits to respect API rate limits and prevent overwhelming servers.',
      },
      {
        title: 'Smart Error Handling',
        description:
          'Continue execution even when individual requests fail, ensuring you get complete test results every time.',
      },
    ],
    codeExample: {
      title: 'Run requests in parallel with concurrency control',
      code: `global:
  execution: parallel
  maxConcurrency: 5
  continueOnError: true
  variables:
    API_URL: https://api.example.com

requests:
  - name: Get Users
    url: \${API_URL}/users
  - name: Get Posts
    url: \${API_URL}/posts
  - name: Get Products
    url: \${API_URL}/products
  - name: Get Orders
    url: \${API_URL}/orders`,
    },
    useCases: [
      'Load testing and performance validation',
      'Running independent API tests simultaneously',
      'Fetching data from multiple endpoints concurrently',
      'Stress testing API rate limits',
    ],
    keywords: [
      'parallel execution',
      'concurrent requests',
      'performance testing',
      'load testing',
      'async HTTP',
      'concurrent API calls',
      'parallel testing',
      'fast request execution',
      'simultaneous requests',
      'maxConcurrency',
    ],
  },
  'response-validation': {
    slug: 'response-validation',
    title: 'Response Validation',
    shortDescription: 'Validate status, headers, and body content',
    description:
      'Automatically validate HTTP response status codes, headers, and body content. Ensure your APIs return exactly what you expect with powerful assertion capabilities.',
    icon: CheckCircle,
    color: 'green',
    gradient: 'from-green-400 to-green-600',
    benefits: [
      {
        title: 'Comprehensive Validation',
        description:
          'Validate status codes, headers, and JSON body content with exact, partial, or pattern matching.',
      },
      {
        title: 'Regex Pattern Support',
        description:
          'Use powerful regex patterns to validate dynamic values like IDs, timestamps, and tokens.',
      },
      {
        title: 'Clear Failure Reports',
        description:
          'Get detailed reports showing exactly what failed and what was expected vs actual.',
      },
    ],
    codeExample: {
      title: 'Validate API responses with multiple criteria',
      code: `requests:
  - name: Get User Profile
    url: https://api.example.com/users/1
    method: GET
    expect:
      status: 200
      headers:
        content-type: application/json
      body:
        id: 1
        username: "johndoe"
        email: "^[a-z]+@[a-z]+\\\\.[a-z]+$"
        active: true`,
    },
    useCases: [
      'API contract testing and validation',
      'Ensuring response format compliance',
      'Regression testing for API changes',
      'Validating error responses',
    ],
    keywords: [
      'response validation',
      'API testing',
      'status code validation',
      'header validation',
      'body validation',
      'response assertions',
      'validation rules',
      'automated testing',
      'contract testing',
    ],
  },
  'watch-mode': {
    slug: 'watch-mode',
    title: 'Watch Mode',
    shortDescription: 'Auto-rerun on file changes',
    description:
      'Automatically re-execute HTTP requests when YAML files change. Perfect for rapid development and testing workflows with instant feedback.',
    icon: Eye,
    color: 'purple',
    gradient: 'from-purple-400 to-purple-600',
    benefits: [
      {
        title: 'Instant Feedback Loop',
        description:
          'See results immediately after saving your YAML file. No need to manually re-run commands.',
      },
      {
        title: 'Smart Debouncing',
        description:
          'Configurable debounce delay prevents duplicate runs when files are saved rapidly.',
      },
      {
        title: 'Clean Terminal Output',
        description: 'Optional screen clearing keeps your terminal organized between runs.',
      },
    ],
    codeExample: {
      title: 'Enable watch mode for rapid development',
      code: `# Terminal command
curl-runner api.yaml --watch

# With options
curl-runner tests/ -w --watch-debounce 500

# YAML configuration
global:
  watch:
    enabled: true
    debounce: 300
    clear: true`,
    },
    useCases: [
      'API development with instant feedback',
      'Iterative test writing and debugging',
      'Rapid prototyping and exploration',
      'Live API monitoring during development',
    ],
    keywords: [
      'watch mode',
      'file watching',
      'auto re-run',
      'live reload',
      'development workflow',
      'instant feedback',
      'rapid development',
      'hot reload',
    ],
  },
  'retry-mechanism': {
    slug: 'retry-mechanism',
    title: 'Retry Mechanism',
    shortDescription: 'Automatic retry with backoff',
    description:
      'Automatically retry failed requests with configurable retry count, delay, and exponential backoff. Handle transient failures gracefully.',
    icon: RotateCcw,
    color: 'orange',
    gradient: 'from-orange-400 to-orange-600',
    benefits: [
      {
        title: 'Exponential Backoff',
        description:
          'Intelligent retry strategy with exponential backoff prevents overwhelming servers during failures.',
      },
      {
        title: 'Configurable Policies',
        description:
          'Fine-tune retry count, delay, and backoff multiplier per request or globally.',
      },
      {
        title: 'Selective Retry',
        description: 'Choose which HTTP status codes or error types should trigger retries.',
      },
    ],
    codeExample: {
      title: 'Configure retry behavior with exponential backoff',
      code: `global:
  retry:
    count: 3
    delay: 1000
    backoff: exponential
    backoffMultiplier: 2

requests:
  - name: Flaky API
    url: https://api.example.com/data
    retry:
      count: 5
      delay: 2000
      on: [500, 502, 503]`,
    },
    useCases: [
      'Handling intermittent network failures',
      'Working with flaky or unreliable APIs',
      'Rate limit recovery',
      'Production API monitoring',
    ],
    keywords: [
      'retry mechanism',
      'exponential backoff',
      'automatic retry',
      'failure recovery',
      'resilient requests',
      'retry policy',
      'error handling',
      'transient failures',
    ],
  },
  'ci-integration': {
    slug: 'ci-integration',
    title: 'CI/CD Integration',
    shortDescription: 'Seamless pipeline integration',
    description:
      'Integrate curl-runner with any CI/CD platform using CI-friendly exit codes, configurable failure thresholds, and JSON output for easy reporting.',
    icon: GitBranch,
    color: 'blue',
    gradient: 'from-blue-400 to-blue-600',
    benefits: [
      {
        title: 'Smart Exit Codes',
        description:
          'Configurable exit codes with strict mode, failure count, and percentage thresholds.',
      },
      {
        title: 'Platform Agnostic',
        description:
          'Works seamlessly with GitHub Actions, GitLab CI, Jenkins, CircleCI, and more.',
      },
      {
        title: 'Artifact Generation',
        description:
          'Export test results as JSON for integration with reporting and analytics tools.',
      },
    ],
    codeExample: {
      title: 'CI-friendly configuration with strict exit codes',
      code: `# GitHub Actions
- name: Run API Tests
  env:
    CURL_RUNNER_STRICT_EXIT: true
  run: |
    curl-runner tests/ \\
      --continue-on-error \\
      --strict-exit \\
      --output-format json \\
      --output results.json`,
    },
    useCases: [
      'Automated API testing in CI/CD pipelines',
      'Pre-deployment validation',
      'Continuous integration testing',
      'Regression test automation',
    ],
    keywords: [
      'CI/CD integration',
      'GitHub Actions',
      'GitLab CI',
      'Jenkins',
      'CircleCI',
      'exit codes',
      'pipeline testing',
      'automated testing',
      'continuous integration',
    ],
  },
  'file-uploads': {
    slug: 'file-uploads',
    title: 'File Uploads',
    shortDescription: 'Multipart form data support',
    description:
      'Upload files and multipart form data with ease. Support for binary files, multiple file uploads, and custom content types.',
    icon: Upload,
    color: 'pink',
    gradient: 'from-pink-400 to-pink-600',
    benefits: [
      {
        title: 'Multipart Form Support',
        description: 'Full support for multipart/form-data with mixed text and binary content.',
      },
      {
        title: 'Multiple Files',
        description: 'Upload multiple files in a single request with custom field names.',
      },
      {
        title: 'Auto Content-Type',
        description: 'Automatic content-type detection and header management for file uploads.',
      },
    ],
    codeExample: {
      title: 'Upload files with multipart form data',
      code: `requests:
  - name: Upload Avatar
    url: https://api.example.com/upload
    method: POST
    files:
      avatar: ./images/profile.jpg
      document: ./docs/resume.pdf
    body:
      userId: 123
      description: "User profile update"`,
    },
    useCases: [
      'Testing file upload endpoints',
      'Uploading images and documents',
      'Multipart form data validation',
      'Binary content uploads',
    ],
    keywords: [
      'file upload',
      'multipart form',
      'form data',
      'binary upload',
      'image upload',
      'file attachment',
      'POST files',
      'upload testing',
    ],
  },
  'output-formats': {
    slug: 'output-formats',
    title: 'Output Formats',
    shortDescription: 'Multiple export formats',
    description:
      'Export test results in multiple formats including JSON, CSV, HTML, and custom templates. Perfect for reporting and integration with other tools.',
    icon: FileJson,
    color: 'yellow',
    gradient: 'from-yellow-400 to-yellow-600',
    benefits: [
      {
        title: 'Multiple Formats',
        description:
          'Export results as JSON, CSV, HTML, or create custom templates for your needs.',
      },
      {
        title: 'Rich Reporting',
        description:
          'Generate comprehensive reports with request/response details, timing, and validation results.',
      },
      {
        title: 'CI/CD Ready',
        description:
          'Machine-readable formats for easy integration with CI/CD and analytics tools.',
      },
    ],
    codeExample: {
      title: 'Export results in various formats',
      code: `# JSON output
curl-runner tests/ --output-format json --output results.json

# CSV output
curl-runner tests/ --output-format csv --output report.csv

# Pretty formatted output
curl-runner tests/ --output-format pretty --pretty-level detailed`,
    },
    useCases: [
      'Generating test reports',
      'Exporting data for analysis',
      'CI/CD artifact creation',
      'Custom reporting workflows',
    ],
    keywords: [
      'output formats',
      'JSON export',
      'CSV export',
      'HTML report',
      'test reporting',
      'export results',
      'format conversion',
      'data export',
    ],
  },
  'response-storage': {
    slug: 'response-storage',
    title: 'Response Storage',
    shortDescription: 'Save responses to disk',
    description:
      'Automatically save HTTP responses to disk for later analysis, debugging, or archival. Support for binary content and custom file naming.',
    icon: Database,
    color: 'indigo',
    gradient: 'from-indigo-400 to-indigo-600',
    benefits: [
      {
        title: 'Automatic Persistence',
        description: 'Save all responses or selectively store specific ones based on criteria.',
      },
      {
        title: 'Binary Support',
        description: 'Store images, PDFs, and other binary content with proper file extensions.',
      },
      {
        title: 'Custom Naming',
        description:
          'Configure file naming patterns with variables and timestamps for organization.',
      },
    ],
    codeExample: {
      title: 'Save responses to disk automatically',
      code: `global:
  output:
    saveResponses: true
    responseDir: ./responses

requests:
  - name: Download Report
    url: https://api.example.com/report.pdf
    saveResponse: report-\${timestamp}.pdf

  - name: Get Data
    url: https://api.example.com/data
    saveResponse: data.json`,
    },
    useCases: [
      'Archiving API responses',
      'Debugging response content',
      'Downloading files and reports',
      'Response comparison over time',
    ],
    keywords: [
      'response storage',
      'save responses',
      'download files',
      'archive responses',
      'store output',
      'persist responses',
      'file download',
    ],
  },
  'conditional-execution': {
    slug: 'conditional-execution',
    title: 'Conditional Execution',
    shortDescription: 'Run requests based on conditions',
    description:
      'Execute requests conditionally based on previous responses, environment variables, or custom logic. Build complex test flows with ease.',
    icon: Zap,
    color: 'violet',
    gradient: 'from-violet-400 to-violet-600',
    benefits: [
      {
        title: 'Dynamic Workflows',
        description: 'Create complex test scenarios that adapt based on API responses and state.',
      },
      {
        title: 'Conditional Logic',
        description: 'Use if/else conditions, comparisons, and boolean logic to control execution.',
      },
      {
        title: 'Variable Dependencies',
        description:
          'Make requests conditional on environment variables or previous response data.',
      },
    ],
    codeExample: {
      title: 'Execute requests conditionally',
      code: `requests:
  - name: Login
    url: https://api.example.com/login
    method: POST
    body:
      username: testuser
      password: testpass

  - name: Get Protected Resource
    url: https://api.example.com/protected
    condition: \${login.status} == 200
    headers:
      Authorization: Bearer \${login.body.token}`,
    },
    useCases: [
      'Multi-step authentication flows',
      'Environment-specific testing',
      'Dependent request chains',
      'Complex integration scenarios',
    ],
    keywords: [
      'conditional execution',
      'if conditions',
      'dynamic workflows',
      'request chaining',
      'conditional logic',
      'flow control',
      'dependent requests',
    ],
  },
  'performance-profiling': {
    slug: 'performance-profiling',
    title: 'Performance Profiling',
    shortDescription: 'Detailed timing metrics',
    description:
      'Get comprehensive performance metrics including DNS lookup, TCP connection, TLS handshake, and response times. Identify bottlenecks and optimize API performance.',
    icon: TrendingUp,
    color: 'emerald',
    gradient: 'from-emerald-400 to-emerald-600',
    benefits: [
      {
        title: 'Detailed Metrics',
        description: 'Track DNS, TCP, TLS, TTFB, and total response times for every request.',
      },
      {
        title: 'Performance Insights',
        description:
          'Identify slow endpoints and bottlenecks with comprehensive timing breakdowns.',
      },
      {
        title: 'Trend Analysis',
        description: 'Export metrics for historical analysis and performance trend tracking.',
      },
    ],
    codeExample: {
      title: 'Enable performance profiling',
      code: `global:
  output:
    showMetrics: true
    showTimings: true
    saveMetrics: performance-\${timestamp}.json

requests:
  - name: API Endpoint
    url: https://api.example.com/data
    profile: true`,
    },
    useCases: [
      'API performance testing',
      'Identifying latency issues',
      'Network performance analysis',
      'Load testing metrics',
    ],
    keywords: [
      'performance profiling',
      'timing metrics',
      'API performance',
      'latency analysis',
      'TTFB',
      'response time',
      'performance testing',
      'metrics tracking',
    ],
  },
  'response-diffing': {
    slug: 'response-diffing',
    title: 'Response Diffing',
    shortDescription: 'Compare responses across environments',
    description:
      'Compare API responses between different environments (staging vs production) to detect inconsistencies and ensure parity.',
    icon: Diff,
    color: 'teal',
    gradient: 'from-teal-400 to-teal-600',
    benefits: [
      {
        title: 'Environment Parity',
        description: 'Ensure staging and production APIs return consistent responses.',
      },
      {
        title: 'Visual Diffs',
        description: 'See color-coded differences highlighting additions, deletions, and changes.',
      },
      {
        title: 'Automated Comparison',
        description: 'Automatically compare responses and flag discrepancies in CI/CD.',
      },
    ],
    codeExample: {
      title: 'Compare responses across environments',
      code: `global:
  diff:
    enabled: true
    environments:
      - staging
      - production

requests:
  - name: Get User
    url:
      staging: https://staging.api.com/user/1
      production: https://api.com/user/1`,
    },
    useCases: [
      'Staging vs production validation',
      'API version comparison',
      'Environment parity testing',
      'Migration validation',
    ],
    keywords: [
      'response diffing',
      'environment comparison',
      'diff tool',
      'API comparison',
      'response comparison',
      'staging vs production',
      'parity testing',
    ],
  },
  'dry-run': {
    slug: 'dry-run',
    title: 'Dry Run Mode',
    shortDescription: 'Preview commands without executing',
    description:
      'Preview the exact curl commands that would be executed without making actual API calls. Perfect for debugging configurations and validating request setups.',
    icon: Terminal,
    color: 'cyan',
    gradient: 'from-cyan-400 to-cyan-600',
    benefits: [
      {
        title: 'Preview Commands',
        description:
          'See the exact curl command that would be executed, including all headers, body, and options.',
      },
      {
        title: 'Safe Debugging',
        description:
          'Debug and validate configurations without making actual API calls or side effects.',
      },
      {
        title: 'Variable Validation',
        description:
          'Verify that variable interpolation and environment settings are working correctly.',
      },
    ],
    codeExample: {
      title: 'Preview curl commands without execution',
      code: `# Basic dry run
curl-runner api.yaml --dry-run

# Short form
curl-runner api.yaml -n

# With verbose output
curl-runner api.yaml -n -v

# Output example:
#   Command:
#     curl -X POST -H 'Authorization: Bearer token' \\
#       -d '{"name":"test"}' https://api.example.com/users
#   ✓ Create User [api]
#      ├─ POST: https://api.example.com/users
#      ├─ ✓ Status: DRY-RUN
#      └─ Duration: 0ms | 0 B`,
    },
    useCases: [
      'Debugging request configurations',
      'Validating authentication headers',
      'Verifying variable interpolation',
      'Reviewing commands before production runs',
    ],
    keywords: [
      'dry run',
      'preview commands',
      'debug requests',
      'validate configuration',
      'command preview',
      'curl preview',
      'safe testing',
      'no execution',
    ],
  },
  snapshots: {
    slug: 'snapshots',
    title: 'Snapshots',
    shortDescription: 'Snapshot testing for APIs',
    description:
      'Create and compare response snapshots to detect unintended API changes. Perfect for regression testing and change detection.',
    icon: Camera,
    color: 'rose',
    gradient: 'from-rose-400 to-rose-600',
    benefits: [
      {
        title: 'Regression Detection',
        description: 'Automatically detect unintended changes in API responses over time.',
      },
      {
        title: 'Easy Updates',
        description: 'Review and update snapshots when intentional changes are made.',
      },
      {
        title: 'Version Control',
        description: 'Store snapshots in version control for team collaboration and history.',
      },
    ],
    codeExample: {
      title: 'Create and compare response snapshots',
      code: `global:
  snapshots:
    enabled: true
    dir: ./__snapshots__
    updateSnapshots: false

requests:
  - name: Get User Profile
    url: https://api.example.com/users/1
    snapshot: true`,
    },
    useCases: [
      'Regression testing',
      'API contract validation',
      'Change detection',
      'Documentation of expected responses',
    ],
    keywords: [
      'snapshots',
      'snapshot testing',
      'regression testing',
      'change detection',
      'API snapshots',
      'response snapshots',
      'contract testing',
    ],
  },
  http2: {
    slug: 'http2',
    title: 'HTTP/2 Support',
    shortDescription: 'Enable HTTP/2 protocol with multiplexing',
    description:
      'Use HTTP/2 protocol for faster, more efficient requests with multiplexing support. Reduce latency and improve throughput for modern APIs.',
    icon: Network,
    color: 'sky',
    gradient: 'from-sky-400 to-sky-600',
    benefits: [
      {
        title: 'Multiplexing',
        description:
          'Send multiple requests over a single connection simultaneously, reducing overhead and latency.',
      },
      {
        title: 'Header Compression',
        description:
          'HTTP/2 compresses headers automatically, reducing bandwidth usage for repeated requests.',
      },
      {
        title: 'Better Performance',
        description:
          'Take advantage of modern server infrastructure with improved connection efficiency.',
      },
    ],
    codeExample: {
      title: 'Enable HTTP/2 for all requests',
      code: `# CLI flag
curl-runner api.yaml --http2

# Environment variable
CURL_RUNNER_HTTP2=true curl-runner api.yaml

# YAML configuration (global)
global:
  defaults:
    http2: true

requests:
  - name: Fast API Call
    url: https://api.example.com/data

# YAML configuration (per-request)
requests:
  - name: HTTP/2 Request
    url: https://api.example.com/data
    http2: true`,
    },
    useCases: [
      'High-performance API testing',
      'Testing HTTP/2 enabled servers',
      'Reducing latency in parallel requests',
      'Modern microservices communication',
    ],
    keywords: [
      'HTTP/2',
      'http2',
      'multiplexing',
      'protocol',
      'performance',
      'latency',
      'connection pooling',
      'header compression',
      'modern APIs',
    ],
  },
  'connection-pooling': {
    slug: 'connection-pooling',
    title: 'Connection Pooling',
    shortDescription: 'Reuse TCP connections with HTTP/2 multiplexing',
    description:
      'Keep TCP connections alive and reuse them for requests to the same host. Combined with HTTP/2 multiplexing, dramatically reduce connection overhead and improve performance for parallel requests.',
    icon: Layers,
    color: 'amber',
    gradient: 'from-amber-400 to-amber-600',
    benefits: [
      {
        title: 'Single TCP Handshake',
        description:
          'Establish one TCP connection per host instead of N connections, eliminating redundant handshakes.',
      },
      {
        title: 'Single TLS Negotiation',
        description:
          'Perform TLS handshake once per host, saving significant time on HTTPS requests.',
      },
      {
        title: 'HTTP/2 Stream Multiplexing',
        description:
          'Multiplex multiple requests as streams over a single connection for maximum throughput.',
      },
    ],
    codeExample: {
      title: 'Enable connection pooling with HTTP/2 multiplexing',
      code: `global:
  execution: parallel
  connectionPool:
    enabled: true
    maxStreamsPerHost: 10
    keepaliveTime: 60
    connectTimeout: 30

requests:
  - name: Get Users
    url: https://api.example.com/users
  - name: Get Posts
    url: https://api.example.com/posts
  - name: Get Comments
    url: https://api.example.com/comments
  # All 3 requests share ONE TCP + TLS connection!`,
    },
    useCases: [
      'High-volume API testing with reduced latency',
      'Parallel requests to the same host',
      'Performance-critical test suites',
      'Load testing with connection reuse',
    ],
    keywords: [
      'connection pooling',
      'TCP reuse',
      'keep-alive',
      'HTTP/2 multiplexing',
      'stream multiplexing',
      'connection reuse',
      'performance optimization',
      'parallel requests',
      'TLS handshake',
      'batch requests',
    ],
  },
  'yaml-wizard': {
    slug: 'yaml-wizard',
    title: 'YAML Wizard',
    shortDescription: 'Interactive CLI for creating YAML files',
    description:
      'Create curl-runner YAML configuration files interactively with a guided wizard. No need to memorize syntax - just answer prompts and get a working configuration file.',
    icon: Wand2,
    color: 'fuchsia',
    gradient: 'from-fuchsia-400 to-fuchsia-600',
    benefits: [
      {
        title: 'Zero Learning Curve',
        description:
          'Get started immediately without reading documentation. The wizard guides you through all options step by step.',
      },
      {
        title: 'Template Support',
        description:
          'Start from pre-built templates for common use cases like API tests, file uploads, and authentication flows.',
      },
      {
        title: 'Edit Existing Files',
        description:
          'Load and modify existing YAML files through the wizard. Perfect for updating configurations without manual editing.',
      },
    ],
    codeExample: {
      title: 'Create YAML files interactively',
      code: `# Quick create with prompts
curl-runner init

# Full interactive wizard
curl-runner init --wizard

# Quick create with URL
curl-runner init https://api.example.com/users

# Edit existing file
curl-runner edit api-test.yaml

# Save to custom file
curl-runner init -w -o my-api.yaml`,
    },
    useCases: [
      'Getting started with curl-runner quickly',
      'Creating complex configurations without syntax errors',
      'Exploring available options and features',
      'Updating existing YAML files safely',
    ],
    keywords: [
      'yaml wizard',
      'interactive cli',
      'configuration generator',
      'create yaml',
      'init command',
      'edit command',
      'wizard mode',
      'guided setup',
      'templates',
      'quick start',
    ],
  },
  validate: {
    slug: 'validate',
    title: 'YAML Validation',
    shortDescription: 'Validate YAML files and auto-fix issues',
    description:
      'Validate curl-runner YAML configuration files against the schema and curl options. Discover issues, get fix suggestions, and auto-fix common problems with a single command.',
    icon: ShieldCheck,
    color: 'lime',
    gradient: 'from-lime-400 to-lime-600',
    benefits: [
      {
        title: 'Catch Errors Early',
        description:
          'Validate YAML files before running them. Catch typos, invalid options, and schema violations immediately.',
      },
      {
        title: 'Auto-Fix Common Issues',
        description:
          'Automatically fix common problems like lowercase HTTP methods and missing URL schemes with --fix.',
      },
      {
        title: 'Detailed Error Reports',
        description:
          'Get clear, actionable error messages with the exact location and suggested fixes for each issue.',
      },
    ],
    codeExample: {
      title: 'Validate YAML files and auto-fix issues',
      code: `# Validate all YAML files in current directory
curl-runner validate

# Validate specific file
curl-runner validate api.yaml

# Validate with glob pattern
curl-runner validate "tests/**/*.yaml"

# Validate and auto-fix issues
curl-runner validate --fix

# Quiet mode - only show errors
curl-runner validate -q

# Combine flags
curl-runner validate -fq tests/`,
    },
    useCases: [
      'Pre-commit validation in CI/CD pipelines',
      'Catching configuration errors before execution',
      'Migrating or upgrading YAML configurations',
      'Onboarding new team members with instant feedback',
    ],
    keywords: [
      'yaml validation',
      'validate command',
      'schema validation',
      'auto-fix',
      'configuration validation',
      'syntax checking',
      'lint yaml',
      'yaml linter',
      'error detection',
      'fix command',
    ],
  },
  convert: {
    slug: 'convert',
    title: 'Curl ⇄ YAML Conversion',
    shortDescription: 'Convert curl commands to YAML and back',
    gradientWords: 4,
    description:
      'Bidirectional conversion between raw curl commands and curl-runner YAML specs. Paste a curl command from docs or DevTools, get clean YAML. Convert YAML back to canonical curl commands.',
    icon: ArrowLeftRight,
    color: 'cyan',
    gradient: 'from-cyan-400 to-teal-600',
    benefits: [
      {
        title: 'Zero Friction Onboarding',
        description:
          'Paste any curl command and instantly get a structured, editable YAML spec. No manual translation needed.',
      },
      {
        title: 'Batch Script Migration',
        description:
          'Convert entire shell scripts full of curl commands into a single YAML collection with one command.',
      },
      {
        title: 'Loss-Aware Translation',
        description:
          'Unsupported curl flags are reported as warnings so you know exactly what was and wasn\'t converted.',
      },
    ],
    codeExample: {
      title: 'Paste curl, get clean YAML',
      code: `# Convert a curl command to YAML
curl-runner convert curl "curl -X POST \\
  https://api.example.com/users \\
  -H 'Authorization: Bearer TOKEN' \\
  -d '{\\"name\\":\\"Alex\\"}'"

# Output:
# request:
#   method: POST
#   url: https://api.example.com/users
#   auth:
#     type: bearer
#     token: TOKEN
#   body:
#     json:
#       name: Alex`,
    },
    useCases: [
      'Importing curl commands from API documentation',
      'Migrating shell scripts to structured YAML workflows',
      'Sharing YAML specs as curl commands with teammates',
      'Converting browser DevTools network requests to test specs',
    ],
    keywords: [
      'curl to yaml',
      'yaml to curl',
      'curl converter',
      'curl import',
      'curl parser',
      'shell script conversion',
      'batch conversion',
      'API migration',
      'postman import',
      'convert command',
    ],
  },
  'env-files': {
    slug: 'env-files',
    title: 'Environment Files',
    shortDescription: 'Load .env files with secret redaction',
    description:
      'Automatically load variables from .env files with environment-specific overrides. Secrets are automatically redacted in output to prevent accidental exposure.',
    icon: Lock,
    color: 'slate',
    gradient: 'from-slate-400 to-slate-600',
    benefits: [
      {
        title: 'Automatic .env Loading',
        description:
          'Load variables from .env, .env.local, .env.{environment}, and .env.{environment}.local files automatically.',
      },
      {
        title: 'Secret Redaction',
        description:
          'Variables prefixed with SECRET_ are automatically redacted in output. Common API key patterns are also detected.',
      },
      {
        title: 'Environment Overrides',
        description:
          'Use --env flag to load environment-specific files like .env.staging or .env.production.',
      },
    ],
    codeExample: {
      title: 'Use .env files with automatic secret protection',
      code: `# .env file
API_URL=https://api.example.com
SECRET_API_KEY=sk_live_abc123def456

# api.yaml
request:
  url: \${API_URL}/users
  headers:
    Authorization: Bearer \${SECRET_API_KEY}

# Run with environment
curl-runner api.yaml --env production

# Output shows redacted secrets
#   curl -H 'Authorization: Bearer [REDACTED]' ...`,
    },
    useCases: [
      'Managing secrets across environments',
      'Preventing credential leaks in logs',
      'Environment-specific configuration',
      'Team collaboration with shared .env.example',
    ],
    keywords: [
      'env files',
      'dotenv',
      'secret redaction',
      'environment variables',
      'api key protection',
      'credential masking',
      'environment overrides',
      '.env support',
      'secret masking',
      'sensitive data',
    ],
  },
};

export const getAllFeatures = (): FeatureData[] => {
  return Object.values(featuresData);
};

export const getFeatureBySlug = (slug: string): FeatureData | null => {
  return featuresData[slug] || null;
};
