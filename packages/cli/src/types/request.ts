/**
 * Request configuration types.
 */

import type { DiffConfig } from './diff';
import type { JsonValue } from './json';
import type { SnapshotConfig } from './snapshot';

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
