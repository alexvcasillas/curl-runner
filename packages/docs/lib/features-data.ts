export interface Benefit {
  title: string;
  description: string;
}

export interface FeatureData {
  slug: string;
  title: string;
  /** Page <title> (without the site suffix) when it should differ from the sidebar `title`. */
  seoTitle?: string;
  /** H1: the benefit, as a sentence. */
  headline: string;
  description: string;
  /** Must pass `curl-runner validate --strict` (tests/features/examples.test.ts). */
  example: { filename: string; yaml: string };
  benefits: [Benefit, Benefit, Benefit];
  goodFor: [string, string, string, string];
  keywords: string[];
  /** Optional "from curl" comparison section: a curl command next to its YAML equivalent. */
  fromCurl?: {
    heading: string;
    intro: string;
    curl: string;
    /** Must pass `curl-runner validate --strict` too. */
    yaml: string;
    outro: string;
    linkText: string;
    href: string;
  };
}

/** Trim the template's outer blank lines and end with one newline, like a file. */
function yaml(source: string): string {
  return `${source.replace(/^\n+|\s+$/g, '')}\n`;
}

/** Sidebar order (the `items` of the Feature mockup). */
const FEATURES: FeatureData[] = [
  {
    slug: 'parallel-execution',
    title: 'Parallel Execution',
    headline: 'Run the whole suite at once, not one request at a time.',
    description:
      'Switch a file to parallel execution and every request goes out together on one libcurl multi handle. Cap concurrency to stay inside rate limits, and keep going past failures so one broken endpoint never hides the rest.',
    example: {
      filename: 'parallel.yaml',
      yaml: yaml(`
global:
  execution: parallel
  maxConcurrency: 5
  continueOnError: true
  variables:
    BASE: https://api.example.com

requests:
  - name: List users
    url: \${BASE}/users
    expect:
      status: 200

  - name: List orders
    url: \${BASE}/orders
    expect:
      status: 200

  - name: List invoices
    url: \${BASE}/invoices
    expect:
      status: 200
`),
    },
    benefits: [
      {
        title: 'One switch',
        description:
          'Set execution: parallel in the file, or pass -p on the command line. The requests stay exactly as they are.',
      },
      {
        title: 'Bounded concurrency',
        description:
          'maxConcurrency caps how many requests are in flight. Leave it unset and everything starts together.',
      },
      {
        title: 'Complete results',
        description:
          'With continueOnError, a failure doesn’t stop the run. You get a result for every request in one report.',
      },
    ],
    goodFor: [
      'Smoke-testing many independent endpoints',
      'Large regression suites in CI',
      'Checking that every service in a stack answers',
      'Respecting a rate limit while still running concurrently',
    ],
    keywords: [
      'parallel execution',
      'concurrent requests',
      'maxConcurrency',
      'parallel API tests',
      'continueOnError',
    ],
  },
  {
    slug: 'response-validation',
    title: 'Response Validation',
    headline: 'Assert on what the API actually returned.',
    description:
      'Describe the response you expect — status, headers, body fields and response time — and curl-runner checks every one. Match exact values, regex patterns, numeric ranges, a list of allowed values, or just that a field exists.',
    example: {
      filename: 'user.yaml',
      yaml: yaml(`
request:
  name: Get user
  url: https://api.example.com/users/1
  expect:
    status: 200
    headers:
      content-type: application/json
    responseTime: "< 500"
    body:
      id: 1
      email: "^[^@]+@[^@]+$"
      role: [admin, editor]
      age: ">= 18"
      createdAt: "*"
`),
    },
    benefits: [
      {
        title: 'More than equality',
        description:
          'Regex patterns, ranges like ">= 18", one-of lists and "*" wildcards cover the values that change on every call.',
      },
      {
        title: 'Status, headers, body, time',
        description:
          'Check the whole response in one expect block, including nested objects, arrays and a response-time budget.',
      },
      {
        title: 'Failures that explain themselves',
        description:
          'Each failed check names the path, what was expected and what came back, so you fix the right thing first.',
      },
    ],
    goodFor: [
      'Contract checks against a running API',
      'Catching a field that silently changed type',
      'Holding endpoints to a response-time budget',
      'Post-deploy smoke tests',
    ],
    keywords: [
      'response validation',
      'API assertions',
      'expect status',
      'regex validation',
      'responseTime',
    ],
  },
  {
    slug: 'retry-mechanism',
    title: 'Retry Mechanism',
    seoTitle: 'Retry failed curl requests with backoff',
    headline: 'Flaky networks shouldn’t fail your suite.',
    description:
      'Retry failed requests with a count, a delay and exponential backoff, per request or per file. 429 and 5xx get another try; a 400 doesn’t.',
    example: {
      filename: 'retry.yaml',
      yaml: yaml(`
global:
  defaults:
    retry: { count: 3, delay: 1000, backoff: 2, retryableStatuses: [429, 500, 502, 503] }

requests:
  - name: Flaky upstream
    url: https://api.example.com/data
    retry:
      count: 5
      delay: 2000
`),
    },
    benefits: [
      {
        title: 'Exponential backoff',
        description:
          'Each attempt waits longer than the last, so a struggling server gets room to recover.',
      },
      {
        title: 'Per request or global',
        description:
          'Set a default for the file, override it on the one endpoint that needs more patience.',
      },
      {
        title: 'Only the right failures',
        description:
          'Retries 429, 500, 502, 503 and 504, never a 400 your request deserved. Dropped connections and timeouts retry too, and a 429’s Retry-After is honoured.',
      },
    ],
    goodFor: [
      'Intermittent network failures in CI',
      'Rate-limited APIs that answer 429',
      'Services that take a moment to warm up after a deploy',
      'Smoke tests against shared staging environments',
    ],
    keywords: [
      'curl retry',
      'curl --retry',
      'curl retry on failure',
      'curl retry delay',
      'exponential backoff',
      'flaky tests',
      'rate limiting',
      'Retry-After',
    ],
    fromCurl: {
      heading: 'From curl --retry',
      intro:
        'curl’s --retry N retries transient errors only, waiting 1 second and doubling the wait on each attempt. --retry-delay S replaces that with a fixed wait of S seconds. The YAML equivalent:',
      curl: 'curl --retry 3 --retry-delay 2 https://api.example.com/data',
      yaml: yaml(`
request:
  name: Flaky upstream
  url: https://api.example.com/data
  retry:
    count: 3
    delay: 2000
    retryableStatuses: [408, 429, 500, 502, 503, 504]
`),
      outro:
        'curl-runner convert maps these flags for you: --retry N becomes count N with a 1000 ms delay, a backoff of 2 and the transient status codes 408, 429, 500, 502, 503 and 504; --retry-delay S turns it into a fixed delay of S × 1000 ms. On top of that, a retry policy can be set per request or as a default for the file, and retryableStatuses lets you choose which statuses are worth another try.',
      linkText: 'curl’s documentation for --retry',
      href: 'https://curl.se/docs/manpage.html#--retry',
    },
  },
  {
    slug: 'watch-mode',
    title: 'Watch Mode',
    headline: 'Save the file, see the result.',
    description:
      'Watch mode runs your requests, then runs them again every time a watched YAML file changes. Edits are debounced, the screen is cleared between runs, and a failing run never stops the watcher.',
    example: {
      filename: 'health.yaml',
      yaml: yaml(`
# Same as: curl-runner health.yaml --watch
global:
  watch:
    enabled: true
    debounce: 300
    clear: true

requests:
  - name: Health check
    url: http://localhost:3000/health
    expect:
      status: 200
`),
    },
    benefits: [
      {
        title: 'A tight loop',
        description:
          'Change a header, a body or an assertion and the run starts again on its own. No switching windows.',
      },
      {
        title: 'Debounced',
        description:
          'A burst of saves becomes one run. A change during a run queues exactly one more.',
      },
      {
        title: 'Flag or file',
        description:
          'Start it with --watch for a session, or set global.watch in the file so the suite always watches.',
      },
    ],
    goodFor: [
      'Building a new endpoint against a local server',
      'Writing assertions for an unfamiliar API',
      'Debugging a request until it passes',
      'Keeping a health check on screen while you work',
    ],
    keywords: ['watch mode', 'file watcher', 'live reload', 'debounce', 'development loop'],
  },
  {
    slug: 'ci-integration',
    title: 'CI Integration',
    headline: 'Exit codes your pipeline can trust.',
    description:
      'curl-runner is one binary that exits non-zero when requests fail. Decide how strict to be — any failure, more than N, or more than a percentage — and save the results as JSON for the build to keep.',
    example: {
      filename: 'ci.yaml',
      yaml: yaml(`
# curl-runner tests/ --output results.json
global:
  continueOnError: true
  ci:
    failOnPercentage: 5

requests:
  - name: Checkout is up
    url: https://staging.example.com/api/checkout/health
    expect:
      status: 200
      responseTime: "< 1000"
`),
    },
    benefits: [
      {
        title: 'Your failure threshold',
        description:
          'strictExit fails on any failed request; failOn and failOnPercentage tolerate a known amount of noise.',
      },
      {
        title: 'Runs anywhere',
        description:
          'A single static binary with no runtime to install — GitHub Actions, GitLab CI, Jenkins or a cron job.',
      },
      {
        title: 'Results as an artifact',
        description:
          '--output writes the run to a file, with secrets redacted, ready to upload or parse.',
      },
    ],
    goodFor: [
      'Blocking a deploy when the API breaks',
      'Nightly checks against production',
      'Post-deploy verification in a pipeline',
      'Tolerating a known-flaky endpoint without ignoring it',
    ],
    keywords: ['CI/CD', 'GitHub Actions', 'exit codes', 'strict exit', 'API testing pipeline'],
  },
  {
    slug: 'file-uploads',
    title: 'File Uploads',
    seoTitle: 'Upload files with multipart form data',
    headline: 'Multipart uploads without the curl flags.',
    description:
      'Upload files with multipart form data: plain fields and file attachments in YAML, with a filename and content type when the server needs them.',
    example: {
      filename: 'upload.yaml',
      yaml: yaml(`
request:
  name: Upload invoice
  url: https://api.example.com/invoices
  method: POST
  formData:
    customerId: 42
    paid: false
    invoice:
      file: ./fixtures/invoice.pdf
      filename: invoice-2026-10.pdf
      contentType: application/pdf
  expect:
    status: 201
`),
    },
    benefits: [
      {
        title: 'Fields and files together',
        description:
          'Strings, numbers and booleans sit next to file attachments in one formData map.',
      },
      {
        title: 'Control the part',
        description: 'Override the filename the server sees and the part’s content type, per file.',
      },
      {
        title: 'Paths stay in the project',
        description:
          'File paths resolve against the working directory and can’t escape it unless you pass --allow-path.',
      },
    ],
    goodFor: [
      'Testing avatar and document upload endpoints',
      'Import endpoints that take a CSV',
      'Forms that mix metadata with an attachment',
      'Regression tests for file-size and type limits',
    ],
    keywords: ['file upload', 'multipart', 'form data', 'formData', 'multipart/form-data'],
  },
  {
    slug: 'output-formats',
    title: 'Output Formats',
    headline: 'Readable in a terminal, parseable in a pipeline.',
    description:
      'Pretty output for people, JSON for tools, raw response bodies for piping. Choose how much detail to show, and save the full results to a file in the same run.',
    example: {
      filename: 'report.yaml',
      yaml: yaml(`
# Override per run: curl-runner report.yaml --output-format json
global:
  output:
    format: pretty
    prettyLevel: detailed
    showHeaders: true
    showMetrics: true
    saveToFile: results.json

requests:
  - name: List users
    url: https://api.example.com/users
`),
    },
    benefits: [
      {
        title: 'Three formats',
        description:
          'pretty for reading, json for jq and dashboards, raw for response bodies only.',
      },
      {
        title: 'Three levels of detail',
        description:
          'minimal prints one line per request; standard and detailed add more, up to a full timing breakdown.',
      },
      {
        title: 'Saved and redacted',
        description:
          'saveToFile or --output writes the results to disk, with secrets redacted as in the terminal.',
      },
    ],
    goodFor: [
      'Feeding results into jq or a dashboard',
      'Quiet CI logs that only show failures',
      'Inspecting headers and timings while debugging',
      'Piping a response body into another tool',
    ],
    keywords: ['output formats', 'JSON output', 'pretty output', 'raw output', 'test reports'],
  },
  {
    slug: 'response-storage',
    title: 'Response Storage',
    headline: 'Use one response in the next request.',
    description: `Store values from a response — a body field, a header, the status — and reference them later as \${store.name}. Chain create-then-read, log in once and reuse the token, all in one file.`,
    example: {
      filename: 'signup.yaml',
      yaml: yaml(`
requests:
  - name: Create user
    url: https://api.example.com/users
    method: POST
    headers:
      Content-Type: application/json
    body: { name: Ada }
    store:
      userId: body.id
      requestId: headers.x-request-id

  - name: Fetch the new user
    url: https://api.example.com/users/\${store.userId}
    headers:
      X-Request-ID: \${store.requestId}
    expect:
      status: 200
`),
    },
    benefits: [
      {
        title: 'Real workflows',
        description:
          'Create a resource, then read, update and delete it using the ID the API just returned.',
      },
      {
        title: 'Any part of the response',
        description:
          'Paths like body.data.items[0].id, headers.x-request-id and status pick exactly the value you need.',
      },
      {
        title: 'Plain interpolation',
        description: `\${store.name} works anywhere a variable does — URLs, headers, bodies — in sequential runs, the default.`,
      },
    ],
    goodFor: [
      'Login flows that hand out a token',
      'CRUD tests that need a fresh ID',
      'Following a pagination cursor',
      'Correlating requests by ID',
    ],
    keywords: [
      'response storage',
      'request chaining',
      'store values',
      'auth token reuse',
      'API workflows',
    ],
  },
  {
    slug: 'conditional-execution',
    title: 'Conditional Execution',
    headline: 'Run a request only when it makes sense.',
    description:
      'Add when: to a request and it runs only if the condition holds against values stored earlier. Write a one-line expression, or combine checks with all and any. Skipped requests are reported with the reason.',
    example: {
      filename: 'account.yaml',
      yaml: yaml(`
requests:
  - name: Get account
    url: https://api.example.com/account
    store:
      plan: body.plan
      status: status

  - name: Load premium reports
    url: https://api.example.com/reports/premium
    when: store.plan == premium

  - name: Open an incident
    url: https://api.example.com/incidents
    method: POST
    headers:
      Content-Type: application/json
    body: { source: smoke-test }
    when:
      any:
        - { left: store.status, operator: ">=", right: 500 }
        - { left: store.plan, operator: not-exists }
`),
    },
    benefits: [
      {
        title: 'Readable conditions',
        description:
          'store.plan == premium reads like what it does. Comparisons, contains, matches, exists and not-exists are built in.',
      },
      {
        title: 'Combine checks',
        description:
          'all requires every condition, any needs just one — enough for real branching without a scripting language.',
      },
      {
        title: 'Visible skips',
        description:
          'A skipped request shows up in the output and the summary, so nothing silently disappears from the run.',
      },
    ],
    goodFor: [
      'Feature-flagged or plan-gated endpoints',
      'Cleanup steps that only run after a create',
      'Branching on an earlier status code',
      'Suites shared between environments',
    ],
    keywords: [
      'conditional execution',
      'when condition',
      'API workflows',
      'skip requests',
      'branching tests',
    ],
  },
  {
    slug: 'performance-profiling',
    title: 'Performance Profiling',
    headline: 'Latency percentiles from the YAML you already have.',
    description:
      'Profile mode sends each request many times and reports min, max, mean, median, p95, p99 and standard deviation. Exclude warmup runs, add concurrency, and export the raw timings.',
    example: {
      filename: 'search.yaml',
      yaml: yaml(`
# curl-runner search.yaml --profile-histogram --profile-export timings.csv
global:
  profile:
    iterations: 100
    warmup: 5
    concurrency: 4

requests:
  - name: Search
    url: https://api.example.com/search?q=shoes
`),
    },
    benefits: [
      {
        title: 'Percentiles, not one number',
        description:
          'p50, p95 and p99 show the tail a single timing hides, alongside the failure rate.',
      },
      {
        title: 'Honest numbers',
        description:
          'Warmup iterations are left out of the statistics, so a cold start doesn’t skew the result.',
      },
      {
        title: 'Take the data with you',
        description:
          'Print an ASCII histogram in the terminal, or export every timing as JSON or CSV.',
      },
    ],
    goodFor: [
      'Comparing an endpoint before and after a change',
      'Spotting tail latency on a slow route',
      'Quick checks under light concurrent load',
      'Collecting timings for a spreadsheet or chart',
    ],
    keywords: ['performance profiling', 'latency percentiles', 'p95', 'p99', 'API benchmarking'],
  },
  {
    slug: 'response-diffing',
    title: 'Response Diffing',
    headline: 'See what changed between two runs.',
    description:
      'Save a run as a labelled baseline, then compare later runs — or another environment — against it. curl-runner reports every added, removed and changed field, in the terminal, as JSON or as Markdown.',
    example: {
      filename: 'product.yaml',
      yaml: yaml(`
# curl-runner product.yaml --diff-save --diff-label staging
# curl-runner product.yaml --diff --diff-compare staging
global:
  diff:
    dir: __baselines__
    outputFormat: markdown

requests:
  - name: Get product
    url: https://api.example.com/products/7
    diff: true
`),
    },
    benefits: [
      {
        title: 'Labelled baselines',
        description:
          'Save runs as staging, production or v1.4 and compare any run against any label.',
      },
      {
        title: 'Offline comparison',
        description:
          'curl-runner diff staging production compares two stored baselines without sending a request.',
      },
      {
        title: 'Reports for humans and tools',
        description:
          'Terminal output while you work, JSON for scripts, Markdown to paste into a pull request.',
      },
    ],
    goodFor: [
      'Checking staging matches production',
      'Reviewing what a refactor changed in responses',
      'Catching unannounced upstream API changes',
      'Attaching a response diff to a pull request',
    ],
    keywords: [
      'response diffing',
      'API diff',
      'baseline comparison',
      'environment parity',
      'regression detection',
    ],
  },
  {
    slug: 'snapshots',
    title: 'Snapshots',
    headline: 'Catch every change to a response.',
    description:
      'The first run saves the response to a snapshot file next to your tests; every later run compares against it. Review intentional changes, update the snapshot and commit it with your code.',
    example: {
      filename: 'me.yaml',
      yaml: yaml(`
# curl-runner me.yaml --snapshot --ci-snapshot
global:
  snapshot:
    dir: __snapshots__
    updateMode: failing

requests:
  - name: Get profile
    url: https://api.example.com/me
    snapshot: true
`),
    },
    benefits: [
      {
        title: 'No expectations to write',
        description:
          'The whole response body becomes the assertion. Any field that changes shows up as a diff.',
      },
      {
        title: 'Deliberate updates',
        description:
          'Use -u to update every snapshot, or --update-failing to refresh only the ones that changed.',
      },
      {
        title: 'Strict in CI',
        description:
          '--ci-snapshot fails the run when a snapshot is missing instead of quietly writing a new one.',
      },
    ],
    goodFor: [
      'Large responses that are tedious to assert field by field',
      'Locking down a public API’s shape',
      'Refactors that must not change output',
      'Reviewing response changes in code review',
    ],
    keywords: [
      'snapshot testing',
      'API snapshots',
      'regression testing',
      'golden files',
      'response comparison',
    ],
  },
  {
    slug: 'dry-run',
    title: 'Dry Run',
    headline: 'See the exact curl command before it runs.',
    description:
      'Dry run resolves variables, builds every request and prints the equivalent curl command — without sending anything. Secrets in the output are redacted.',
    example: {
      filename: 'create-user.yaml',
      yaml: yaml(`
# Same as: curl-runner create-user.yaml --dry-run
global:
  dryRun: true

requests:
  - name: Create user
    url: https://api.example.com/users
    method: POST
    headers:
      Authorization: Bearer \${API_TOKEN}
      Content-Type: application/json
    body: { name: Ada }
`),
    },
    benefits: [
      {
        title: 'Nothing is sent',
        description: 'Safe to point at production: requests are built and printed, never executed.',
      },
      {
        title: 'Copy-paste curl',
        description:
          'Each request prints as a curl command you can run by hand or share in a bug report.',
      },
      {
        title: 'Variables resolved',
        description:
          'See the final URL, headers and body after interpolation — the fastest way to find a typo in a variable.',
      },
    ],
    goodFor: [
      'Checking a new file before it touches a real API',
      'Debugging variable interpolation',
      'Sharing a reproducible request with a teammate',
      'Reviewing destructive requests before running them',
    ],
    keywords: ['dry run', 'preview requests', 'curl command', 'debugging', 'safe testing'],
  },
  {
    slug: 'http2',
    title: 'HTTP/2',
    headline: 'Test over the protocol you ship.',
    description:
      'Ask libcurl for HTTP/2 on one request, on every request in a file, or for a whole run with --http2. Servers that don’t support it fall back to HTTP/1.1.',
    example: {
      filename: 'feed.yaml',
      yaml: yaml(`
# Or for a whole run: curl-runner feed.yaml --http2
global:
  http2: true

requests:
  - name: Feed over HTTP/2
    url: https://api.example.com/feed

  - name: Legacy endpoint
    url: https://legacy.example.com/feed
    http2: false
`),
    },
    benefits: [
      {
        title: 'Production parity',
        description:
          'Exercise the same protocol your clients use, including the server’s HTTP/2 configuration.',
      },
      {
        title: 'Granular control',
        description:
          'A file default, a per-request override, or a flag for one run — whichever fits.',
      },
      {
        title: 'Built on libcurl',
        description:
          'Negotiation and fallback come from libcurl’s mature HTTP/2 support, not a custom client.',
      },
    ],
    goodFor: [
      'APIs served behind HTTP/2 load balancers',
      'Verifying a server’s HTTP/2 setup',
      'Comparing HTTP/1.1 and HTTP/2 behaviour',
      'Testing gRPC-adjacent HTTP/2 gateways',
    ],
    keywords: ['HTTP/2', 'http2', 'multiplexing', 'libcurl', 'protocol testing'],
  },
  {
    slug: 'connection-pooling',
    title: 'Connection Pooling',
    headline: 'Reuse connections across parallel requests.',
    description:
      'With pooling on, parallel requests share connections to the same host instead of each opening its own, and the pool caps connections per host, forces HTTP/2 and applies keepalive and connect timeouts. Multiplexing is always on in parallel runs.',
    example: {
      filename: 'pool.yaml',
      yaml: yaml(`
global:
  execution: parallel
  connectionPool:
    enabled: true
    maxStreamsPerHost: 10
    keepaliveTime: 60
    connectTimeout: 30

requests:
  - name: Users
    url: https://api.example.com/users
  - name: Posts
    url: https://api.example.com/posts
  - name: Comments
    url: https://api.example.com/comments
`),
    },
    benefits: [
      {
        title: 'Fewer handshakes',
        description:
          'Requests to the same host reuse open connections, so TCP and TLS setup isn’t repeated for each one.',
      },
      {
        title: 'HTTP/2 multiplexing',
        description:
          'In parallel runs, many requests share one connection as separate streams when the server supports HTTP/2.',
      },
      {
        title: 'Tunable',
        description:
          'maxStreamsPerHost, keepaliveTime and connectTimeout in YAML, or --connection-pool and friends on the command line.',
      },
    ],
    goodFor: [
      'Large parallel suites against a single API host',
      'Hosts with expensive TLS setup',
      'Keeping the connection count low on shared environments',
      'Mirroring how a pooled production client behaves',
    ],
    keywords: [
      'connection pooling',
      'keepalive',
      'HTTP/2 multiplexing',
      'TCP reuse',
      'TLS handshake',
    ],
  },
  {
    slug: 'yaml-wizard',
    title: 'YAML Wizard',
    headline: 'Answer a few questions, get a working test.',
    description:
      'curl-runner init asks for a URL, a method and a name and writes the file. The full wizard walks through headers, body, auth, retries and validation, starting from a template if you like — then offers to run it.',
    example: {
      filename: 'request.yaml',
      yaml: yaml(`
# curl-runner init --wizard, "API Test" template
request:
  name: List users
  url: https://api.example.com/users
  method: GET
  expect:
    status: 200
    responseTime: "< 2000"
`),
    },
    benefits: [
      {
        title: 'Start from a template',
        description:
          'Basic GET, basic POST, API test, file upload and auth flow templates fill in sensible defaults.',
      },
      {
        title: 'Edit what you have',
        description:
          'curl-runner edit opens an existing file in the wizard and keeps the keys it doesn’t ask about.',
      },
      {
        title: 'Run it straight away',
        description:
          'The wizard saves the file and offers to run it, so you see a real response in the first minute.',
      },
    ],
    goodFor: [
      'Writing your first curl-runner file',
      'Teammates new to the YAML format',
      'Scaffolding a test for a new endpoint',
      'Adding auth or retries to an existing file',
    ],
    keywords: ['YAML wizard', 'curl-runner init', 'interactive CLI', 'scaffolding', 'templates'],
  },
  {
    slug: 'validate',
    title: 'YAML Validation',
    headline: 'Catch the typo before the run.',
    description:
      'curl-runner validate checks every file against the schema: unknown keys with suggestions, invalid values, risky settings. --fix repairs what it safely can, and --strict turns warnings into failures.',
    example: {
      filename: 'user.yaml',
      yaml: yaml(`
# curl-runner validate --strict tests/
#   ● request.timout: Unknown key "timout" (did you mean "timeout"?)
#     fix: Rename to "timeout"
request:
  name: Get user
  url: https://api.example.com/users/1
  timeout: 5000
  expect:
    status: 200
`),
    },
    benefits: [
      {
        title: 'Suggestions, not just errors',
        description: 'A misspelled key comes back with the key you meant, and the path to find it.',
      },
      {
        title: 'Auto-fix',
        description:
          '--fix renames misspelled keys and normalises values in place where the fix is unambiguous.',
      },
      {
        title: 'A gate for CI',
        description:
          'Run validate --strict before the suite so a broken file fails fast with a clear message.',
      },
    ],
    goodFor: [
      'Pre-commit hooks',
      'A first CI step before the real run',
      'Reviewing files written by hand or by tools',
      'Migrating a large suite to a new version',
    ],
    keywords: ['YAML validation', 'schema validation', 'lint', 'auto-fix', 'curl-runner validate'],
  },
  {
    slug: 'convert',
    title: 'curl ⇄ YAML',
    seoTitle: 'Convert curl to YAML (and back)',
    headline: 'Paste a curl command, get a test.',
    description:
      'Convert curl commands, or a whole shell script of them, to YAML and back. Flags curl-runner doesn’t recognise come back as warnings, not errors.',
    example: {
      filename: 'users.yaml',
      yaml: yaml(`
# curl-runner convert curl "curl https://api.example.com/users?page=2 \\
#   -H 'Accept: application/json' -H 'Authorization: Bearer \${API_TOKEN}' -m 10"
request:
  method: GET
  url: https://api.example.com/users
  params:
    page: "2"
  headers:
    Accept: application/json
  auth:
    type: bearer
    token: "\${API_TOKEN}"
  timeout: 10000
`),
    },
    benefits: [
      {
        title: 'From the docs to a test',
        description:
          'Copy the curl example from API docs or your browser’s network tab and turn it into a file in one command.',
      },
      {
        title: 'Understands curl',
        description:
          'Query strings become params, bearer headers become auth, -m becomes a timeout in milliseconds.',
      },
      {
        title: 'Both directions',
        description:
          'convert yaml prints the curl command for a file — handy for bug reports and teammates without curl-runner.',
      },
    ],
    goodFor: [
      'Turning API documentation examples into tests',
      'Migrating a folder of shell scripts',
      'Sharing a test as a one-line curl command',
      'Reproducing a request from browser dev tools',
    ],
    keywords: ['curl to YAML', 'convert curl', 'curl converter', 'shell script migration', 'curl'],
  },
  {
    slug: 'env-files',
    title: 'Environment Files',
    headline: 'Keep secrets in .env, out of your YAML.',
    description:
      'curl-runner loads .env and .env.local, plus .env.<name> and .env.<name>.local when you pass --env <name>. Values become variables, and anything named SECRET_* is redacted from output.',
    example: {
      filename: 'api.yaml',
      yaml: yaml(`
# .env.staging
#   API_URL=https://staging.example.com
#   SECRET_API_KEY=...
#
# curl-runner api.yaml --env staging
request:
  name: List users
  url: \${API_URL}/users
  headers:
    X-API-Key: \${SECRET_API_KEY}
  expect:
    status: 200
`),
    },
    benefits: [
      {
        title: 'One file per environment',
        description:
          '--env staging layers .env.staging over .env; .local files override both and stay out of git.',
      },
      {
        title: 'Redacted by default',
        description:
          'SECRET_* values, and common token formats, show as [REDACTED] in the terminal and in saved results.',
      },
      {
        title: 'The same YAML everywhere',
        description:
          'Point one suite at local, staging and production by switching the --env name.',
      },
    ],
    goodFor: [
      'API keys and tokens that must not be committed',
      'Switching between staging and production',
      'Per-developer overrides in .env.local',
      'CI logs that must not leak credentials',
    ],
    keywords: ['env files', 'dotenv', 'secret redaction', 'environment variables', 'API keys'],
  },
];

export function getAllFeatures(): FeatureData[] {
  return FEATURES;
}

export function getFeatureBySlug(slug: string): FeatureData | null {
  return FEATURES.find((f) => f.slug === slug) ?? null;
}

export function getAdjacentFeatures(slug: string): {
  prev: FeatureData | null;
  next: FeatureData | null;
} {
  const index = FEATURES.findIndex((f) => f.slug === slug);
  if (index === -1) {
    return { prev: null, next: null };
  }
  return { prev: FEATURES[index - 1] ?? null, next: FEATURES[index + 1] ?? null };
}
