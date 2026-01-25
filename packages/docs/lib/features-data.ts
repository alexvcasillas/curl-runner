import type { LucideIcon } from 'lucide-react';
import {
  Camera,
  CheckCircle,
  Database,
  Diff,
  Eye,
  FileJson,
  Gauge,
  GitBranch,
  RotateCcw,
  TrendingUp,
  Upload,
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
};

export const getAllFeatures = (): FeatureData[] => {
  return Object.values(featuresData);
};

export const getFeatureBySlug = (slug: string): FeatureData | null => {
  return featuresData[slug] || null;
};
