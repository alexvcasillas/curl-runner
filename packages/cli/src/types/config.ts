export type JsonValue = string | number | boolean | null | JsonObject | JsonArray;
export interface JsonObject {
  [key: string]: JsonValue;
}
export interface JsonArray extends Array<JsonValue> {}

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

export interface GlobalConfig {
  execution?: 'sequential' | 'parallel';
  continueOnError?: boolean;
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
