export type JsonValue = string | number | boolean | null | JsonObject | JsonArray;
export interface JsonObject {
  [key: string]: JsonValue;
}
export interface JsonArray extends Array<JsonValue> {}

/**
 * Configuration for a file attachment in a form data request.
 *
 * Examples:
 * - `{ file: "./image.png" }` - Simple file attachment
 * - `{ file: "./doc.pdf", filename: "document.pdf" }` - With custom filename
 * - `{ file: "./data.json", contentType: "application/json" }` - With explicit content type
 */
export interface FileAttachment {
  /** Path to the file (relative to YAML file or absolute) */
  file: string;
  /** Custom filename to send (defaults to actual filename) */
  filename?: string;
  /** Explicit content type (curl will auto-detect if not specified) */
  contentType?: string;
}

/**
 * A form field value can be a string, number, boolean, or a file attachment.
 */
export type FormFieldValue = string | number | boolean | FileAttachment;

/**
 * Configuration for form data (multipart/form-data) requests.
 * Each key is a form field name, and the value can be a simple value or a file attachment.
 *
 * Examples:
 * ```yaml
 * formData:
 *   name: "John Doe"
 *   age: 30
 *   avatar:
 *     file: "./avatar.png"
 *   document:
 *     file: "./report.pdf"
 *     filename: "quarterly-report.pdf"
 *     contentType: "application/pdf"
 * ```
 */
export type FormDataConfig = Record<string, FormFieldValue>;

/**
 * Configuration for storing response values as variables for subsequent requests.
 * Maps a variable name to a JSON path in the response.
 *
 * Examples:
 * - `{ "userId": "body.id" }` - Store response body's id field as ${store.userId}
 * - `{ "token": "body.data.token" }` - Store nested field
 * - `{ "statusCode": "status" }` - Store HTTP status code
 * - `{ "contentType": "headers.content-type" }` - Store response header
 */
export type StoreConfig = Record<string, string>;

/**
 * Operators for conditional expressions.
 */
export type ConditionOperator =
  | '=='
  | '!='
  | '>'
  | '<'
  | '>='
  | '<='
  | 'contains'
  | 'matches'
  | 'exists'
  | 'not-exists';

/**
 * A single condition expression comparing a store value against an expected value.
 *
 * Examples:
 * - `{ left: "store.status", operator: "==", right: 200 }`
 * - `{ left: "store.userId", operator: "exists" }`
 * - `{ left: "store.body.type", operator: "contains", right: "user" }`
 */
export interface ConditionExpression {
  /** Left operand - typically a store path like "store.status" or "store.body.id" */
  left: string;
  /** Comparison operator */
  operator: ConditionOperator;
  /** Right operand - the value to compare against. Optional for exists/not-exists. */
  right?: string | number | boolean;
  /** Case-sensitive comparison for string operators. Default: false (case-insensitive) */
  caseSensitive?: boolean;
}

/**
 * Configuration for conditional request execution.
 * Supports single conditions, AND (all), and OR (any) compound conditions.
 *
 * Examples:
 * ```yaml
 * # Single condition
 * when:
 *   left: store.status
 *   operator: "=="
 *   right: 200
 *
 * # AND condition (all must be true)
 * when:
 *   all:
 *     - left: store.status
 *       operator: "=="
 *       right: 200
 *     - left: store.userId
 *       operator: exists
 *
 * # OR condition (any must be true)
 * when:
 *   any:
 *     - left: store.type
 *       operator: "=="
 *       right: "admin"
 *     - left: store.type
 *       operator: "=="
 *       right: "superuser"
 * ```
 */
export interface WhenCondition {
  /** All conditions must be true (AND logic) */
  all?: ConditionExpression[];
  /** Any condition must be true (OR logic) */
  any?: ConditionExpression[];
  /** Single condition - left operand */
  left?: string;
  /** Single condition - operator */
  operator?: ConditionOperator;
  /** Single condition - right operand */
  right?: string | number | boolean;
  /** Case-sensitive comparison for string operators. Default: false */
  caseSensitive?: boolean;
}

/**
 * SSL/TLS certificate configuration options.
 *
 * Examples:
 * - `{ verify: false }` - Disable SSL verification (equivalent to insecure: true)
 * - `{ ca: "./certs/ca.pem" }` - Use custom CA certificate
 * - `{ cert: "./certs/client.pem", key: "./certs/client-key.pem" }` - Mutual TLS (mTLS)
 */
export interface SSLConfig {
  /** Whether to verify SSL certificates. Defaults to true. */
  verify?: boolean;
  /** Path to CA certificate file for custom certificate authorities. */
  ca?: string;
  /** Path to client certificate file for mutual TLS authentication. */
  cert?: string;
  /** Path to client private key file for mutual TLS authentication. */
  key?: string;
}

export interface RequestConfig {
  name?: string;
  url: string;
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'HEAD' | 'OPTIONS';
  headers?: Record<string, string>;
  params?: Record<string, string>;
  sourceFile?: string; // Source YAML file for better output organization
  body?: JsonValue;
  /**
   * Form data for multipart/form-data requests.
   * Use this for file uploads or when you need to send form fields.
   * Cannot be used together with 'body'.
   *
   * @example
   * formData:
   *   username: "john"
   *   avatar:
   *     file: "./avatar.png"
   */
  formData?: FormDataConfig;
  timeout?: number;
  followRedirects?: boolean;
  maxRedirects?: number;
  auth?: {
    type: 'basic' | 'bearer';
    username?: string;
    password?: string;
    token?: string;
  };
  proxy?: string;
  insecure?: boolean;
  /**
   * SSL/TLS certificate configuration.
   * Use this for custom CA certificates or mutual TLS (mTLS) authentication.
   *
   * @example
   * ssl:
   *   verify: true
   *   ca: "./certs/ca.pem"
   *   cert: "./certs/client.pem"
   *   key: "./certs/client-key.pem"
   */
  ssl?: SSLConfig;
  output?: string;
  /**
   * Use HTTP/2 protocol for this request.
   */
  http2?: boolean;
  retry?: {
    count: number;
    delay?: number;
    /** Exponential backoff multiplier. Default is 1 (no backoff).
     * Example: with delay=1000 and backoff=2, delays are: 1000ms, 2000ms, 4000ms, 8000ms... */
    backoff?: number;
  };
  variables?: Record<string, string>;
  /**
   * Store response values as variables for subsequent requests.
   * Use JSON path syntax to extract values from the response.
   *
   * @example
   * store:
   *   userId: body.id
   *   token: body.data.accessToken
   *   statusCode: status
   *   contentType: headers.content-type
   */
  store?: StoreConfig;
  /**
   * Conditional execution - skip/run request based on previous results.
   * Only works in sequential execution mode.
   *
   * @example
   * # Object syntax
   * when:
   *   left: store.status
   *   operator: "=="
   *   right: 200
   *
   * # String shorthand
   * when: "store.status == 200"
   *
   * # Compound conditions
   * when:
   *   all:
   *     - left: store.userId
   *       operator: exists
   *     - left: store.status
   *       operator: "<"
   *       right: 400
   */
  when?: WhenCondition | string;
  expect?: {
    failure?: boolean; // If true, expect the request to fail (for negative testing)
    status?: number | number[];
    headers?: Record<string, string>;
    body?: JsonValue;
    responseTime?: string; // Response time validation like "< 1000", "> 500, < 2000"
  };
  /**
   * Snapshot configuration for this request.
   * Use `true` to enable with defaults, or provide detailed config.
   */
  snapshot?: SnapshotConfig | boolean;
  /**
   * Response diffing configuration for this request.
   * Use `true` to enable with defaults, or provide detailed config.
   */
  diff?: DiffConfig | boolean;
  sourceOutputConfig?: {
    verbose?: boolean;
    showHeaders?: boolean;
    showBody?: boolean;
    showMetrics?: boolean;
    format?: 'json' | 'pretty' | 'raw';
    prettyLevel?: 'minimal' | 'standard' | 'detailed';
    saveToFile?: string;
  };
}

export interface CollectionConfig {
  name: string;
  description?: string;
  variables?: Record<string, string>;
  defaults?: Partial<RequestConfig>;
  requests: RequestConfig[];
}

/**
 * CI exit code configuration options.
 * These options control how curl-runner exits in CI/CD pipelines.
 */
export interface CIExitConfig {
  /**
   * When true, exit with code 1 if any validation failures occur,
   * regardless of the continueOnError setting.
   * This is useful for CI/CD pipelines that need strict validation.
   */
  strictExit?: boolean;
  /**
   * Maximum number of failures allowed before exiting with code 1.
   * If set to 0, any failure will cause a non-zero exit.
   * If undefined and strictExit is true, any failure causes non-zero exit.
   */
  failOn?: number;
  /**
   * Maximum percentage of failures allowed before exiting with code 1.
   * Value should be between 0 and 100.
   * If set to 10, up to 10% of requests can fail without causing a non-zero exit.
   */
  failOnPercentage?: number;
}

/**
 * Configuration for TCP connection pooling with HTTP/2 multiplexing.
 * Groups requests by host and executes them in batched curl processes
 * to reuse TCP/TLS connections and leverage HTTP/2 stream multiplexing.
 */
export interface ConnectionPoolConfig {
  /**
   * Enable connection pooling. When enabled, requests to the same host
   * are batched into a single curl process with HTTP/2 multiplexing.
   * Default: false
   */
  enabled?: boolean;
  /**
   * Maximum concurrent streams per host connection.
   * Maps to curl's --parallel-max flag.
   * Default: 10
   */
  maxStreamsPerHost?: number;
  /**
   * TCP keepalive time in seconds.
   * Maps to curl's --keepalive-time flag.
   * Default: 60
   */
  keepaliveTime?: number;
  /**
   * Connection timeout in seconds for establishing new connections.
   * Maps to curl's --connect-timeout flag.
   * Default: 30
   */
  connectTimeout?: number;
}

export interface GlobalConfig {
  execution?: 'sequential' | 'parallel';
  /**
   * Maximum number of concurrent requests when using parallel execution.
   * If not set, all requests will execute simultaneously.
   * Useful for avoiding rate limiting or overwhelming target servers.
   */
  maxConcurrency?: number;
  continueOnError?: boolean;
  /**
   * Dry run mode: show curl commands without executing them.
   * Useful for previewing what commands would be run.
   */
  dryRun?: boolean;
  /**
   * Use HTTP/2 protocol with multiplexing support.
   * Passes --http2 flag to curl.
   */
  http2?: boolean;
  /**
   * TCP connection pooling configuration.
   * Enables request batching and HTTP/2 multiplexing for same-host requests.
   */
  connectionPool?: ConnectionPoolConfig;
  /**
   * CI/CD exit code configuration.
   * Controls when curl-runner should exit with non-zero status codes.
   */
  ci?: CIExitConfig;
  /**
   * Global SSL/TLS certificate configuration.
   * Applied to all requests unless overridden at the request level.
   */
  ssl?: SSLConfig;
  /**
   * Watch mode configuration.
   * Automatically re-runs requests when YAML files change.
   */
  watch?: WatchConfig;
  /**
   * Performance profiling mode configuration.
   * Runs requests multiple times to collect p50/p95/p99 latency stats.
   */
  profile?: ProfileConfig;
  /**
   * Snapshot testing configuration.
   * Saves response snapshots and compares future runs against them.
   */
  snapshot?: GlobalSnapshotConfig;
  /**
   * Response diffing configuration.
   * Compare responses between environments or runs to detect API drift.
   */
  diff?: GlobalDiffConfig;
  variables?: Record<string, string>;
  output?: {
    verbose?: boolean;
    showHeaders?: boolean;
    showBody?: boolean;
    showMetrics?: boolean;
    format?: 'json' | 'pretty' | 'raw';
    prettyLevel?: 'minimal' | 'standard' | 'detailed';
    saveToFile?: string;
    /** Internal flag set when dry-run mode is enabled to always show commands */
    dryRun?: boolean;
  };
  defaults?: Partial<RequestConfig>;
}

export interface YamlFile {
  version?: string;
  global?: GlobalConfig;
  collection?: CollectionConfig;
  requests?: RequestConfig[];
  request?: RequestConfig;
}

export interface ExecutionResult {
  request: RequestConfig;
  success: boolean;
  status?: number;
  headers?: Record<string, string>;
  body?: JsonValue;
  error?: string;
  /** True if this result is from dry-run mode (no actual request made) */
  dryRun?: boolean;
  metrics?: {
    duration: number;
    size?: number;
    dnsLookup?: number;
    tcpConnection?: number;
    tlsHandshake?: number;
    firstByte?: number;
    download?: number;
  };
  /** Snapshot comparison result (if snapshot testing enabled). */
  snapshotResult?: SnapshotCompareResult;
  /** Diff comparison result (if response diffing enabled). */
  diffResult?: DiffCompareResult;
  /** Whether this request was skipped due to a `when` condition. */
  skipped?: boolean;
  /** Reason the request was skipped (condition that failed). */
  skipReason?: string;
}

export interface ExecutionSummary {
  total: number;
  successful: number;
  failed: number;
  skipped: number;
  duration: number;
  results: ExecutionResult[];
}

/**
 * Context for storing response values between sequential requests.
 * Values are stored as strings and can be referenced using ${store.variableName} syntax.
 */
export type ResponseStoreContext = Record<string, string>;

/**
 * Configuration for watch mode.
 * Watch mode automatically re-runs requests when YAML files change.
 */
export interface WatchConfig {
  /** Enable watch mode. Default: false */
  enabled?: boolean;
  /** Debounce delay in milliseconds. Default: 300 */
  debounce?: number;
  /** Clear screen between runs. Default: true */
  clear?: boolean;
}

/**
 * Configuration for performance profiling mode.
 * Runs requests multiple times to collect latency statistics.
 */
export interface ProfileConfig {
  /** Number of iterations to run. Default: 10 */
  iterations: number;
  /** Number of warmup iterations to exclude from stats. Default: 1 */
  warmup?: number;
  /** Number of concurrent iterations. Default: 1 (sequential) */
  concurrency?: number;
  /** Show ASCII histogram in output. Default: false */
  histogram?: boolean;
  /** Export raw timings to file (JSON or CSV based on extension) */
  exportFile?: string;
}

/**
 * Statistics computed from profile run timings.
 */
export interface ProfileStats {
  /** Total iterations run (excluding warmup) */
  iterations: number;
  /** Warmup iterations excluded */
  warmup: number;
  /** Minimum latency in ms */
  min: number;
  /** Maximum latency in ms */
  max: number;
  /** Mean latency in ms */
  mean: number;
  /** Median latency in ms (same as p50) */
  median: number;
  /** 50th percentile latency in ms */
  p50: number;
  /** 95th percentile latency in ms */
  p95: number;
  /** 99th percentile latency in ms */
  p99: number;
  /** Standard deviation in ms */
  stdDev: number;
  /** Number of failed iterations */
  failures: number;
  /** Failure rate as percentage */
  failureRate: number;
  /** Raw timing values (for export) */
  timings: number[];
}

/**
 * Result of a profiled request execution.
 */
export interface ProfileResult {
  request: RequestConfig;
  stats: ProfileStats;
  /** Individual results from each iteration */
  iterations: ExecutionResult[];
}

/**
 * Configuration for snapshot testing.
 * Snapshots save response data and compare future runs against them.
 */
export interface SnapshotConfig {
  /** Enable snapshot testing for this request. */
  enabled?: boolean;
  /** Custom snapshot name (defaults to request name). */
  name?: string;
  /** What to include in snapshot. Default: ['body'] */
  include?: ('body' | 'status' | 'headers')[];
  /** Paths to exclude from comparison (e.g., 'body.timestamp'). */
  exclude?: string[];
  /** Match rules for dynamic values (path -> '*' or 'regex:pattern'). */
  match?: Record<string, string>;
}

/**
 * Global snapshot configuration.
 */
export interface GlobalSnapshotConfig extends SnapshotConfig {
  /** Directory for snapshot files. Default: '__snapshots__' */
  dir?: string;
  /** Update mode: 'none' | 'all' | 'failing'. Default: 'none' */
  updateMode?: 'none' | 'all' | 'failing';
  /** CI mode: fail if snapshot is missing. Default: false */
  ci?: boolean;
}

/**
 * Stored snapshot data for a single request.
 */
export interface Snapshot {
  status?: number;
  headers?: Record<string, string>;
  body?: JsonValue;
  hash: string;
  updatedAt: string;
}

/**
 * Snapshot file format.
 */
export interface SnapshotFile {
  version: number;
  snapshots: Record<string, Snapshot>;
}

/**
 * Result of comparing a response against a snapshot.
 */
export interface SnapshotDiff {
  path: string;
  expected: unknown;
  received: unknown;
  type: 'added' | 'removed' | 'changed' | 'type_mismatch';
}

/**
 * Result of snapshot comparison.
 */
export interface SnapshotCompareResult {
  match: boolean;
  isNew: boolean;
  updated: boolean;
  differences: SnapshotDiff[];
}

/**
 * Configuration for response diffing at request level.
 */
export interface DiffConfig {
  /** Enable diffing for this request. */
  enabled?: boolean;
  /** Paths to exclude from comparison (e.g., 'body.timestamp'). */
  exclude?: string[];
  /** Match rules for dynamic values (path -> '*' or 'regex:pattern'). */
  match?: Record<string, string>;
  /** Include timing differences in comparison. Default: false */
  includeTimings?: boolean;
}

/**
 * Global configuration for response diffing.
 */
export interface GlobalDiffConfig extends DiffConfig {
  /** Directory for baseline files. Default: '__baselines__' */
  dir?: string;
  /** Label for current run (e.g., 'staging', 'production'). */
  label?: string;
  /** Label to compare against. */
  compareWith?: string;
  /** Save current run as baseline. */
  save?: boolean;
  /** Output format for diff results. Default: 'terminal' */
  outputFormat?: 'terminal' | 'json' | 'markdown';
}

/**
 * Stored baseline data for a single request.
 */
export interface Baseline {
  status?: number;
  headers?: Record<string, string>;
  body?: JsonValue;
  timing?: number;
  hash: string;
  capturedAt: string;
}

/**
 * Baseline file format.
 */
export interface BaselineFile {
  version: number;
  label: string;
  capturedAt: string;
  baselines: Record<string, Baseline>;
}

/**
 * Single difference in response comparison.
 */
export interface ResponseDiff {
  path: string;
  baseline: unknown;
  current: unknown;
  type: 'added' | 'removed' | 'changed' | 'type_mismatch';
}

/**
 * Result of comparing a response against a baseline.
 */
export interface DiffCompareResult {
  requestName: string;
  hasDifferences: boolean;
  isNewBaseline: boolean;
  baselineLabel: string;
  currentLabel: string;
  differences: ResponseDiff[];
  timingDiff?: {
    baseline: number;
    current: number;
    changePercent: number;
  };
}

/**
 * Summary of diff comparison across all requests.
 */
export interface DiffSummary {
  totalRequests: number;
  unchanged: number;
  changed: number;
  newBaselines: number;
  results: DiffCompareResult[];
}
