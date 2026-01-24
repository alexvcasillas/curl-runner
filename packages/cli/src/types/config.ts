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
  output?: string;
  retry?: {
    count: number;
    delay?: number;
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
  expect?: {
    failure?: boolean; // If true, expect the request to fail (for negative testing)
    status?: number | number[];
    headers?: Record<string, string>;
    body?: JsonValue;
    responseTime?: string; // Response time validation like "< 1000", "> 500, < 2000"
  };
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
   * CI/CD exit code configuration.
   * Controls when curl-runner should exit with non-zero status codes.
   */
  ci?: CIExitConfig;
  variables?: Record<string, string>;
  output?: {
    verbose?: boolean;
    showHeaders?: boolean;
    showBody?: boolean;
    showMetrics?: boolean;
    format?: 'json' | 'pretty' | 'raw';
    prettyLevel?: 'minimal' | 'standard' | 'detailed';
    saveToFile?: string;
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
  metrics?: {
    duration: number;
    size?: number;
    dnsLookup?: number;
    tcpConnection?: number;
    tlsHandshake?: number;
    firstByte?: number;
    download?: number;
  };
}

export interface ExecutionSummary {
  total: number;
  successful: number;
  failed: number;
  duration: number;
  results: ExecutionResult[];
}

/**
 * Context for storing response values between sequential requests.
 * Values are stored as strings and can be referenced using ${store.variableName} syntax.
 */
export type ResponseStoreContext = Record<string, string>;
