export type JsonValue = string | number | boolean | null | JsonObject | JsonArray;
export interface JsonObject {
  [key: string]: JsonValue;
}
export interface JsonArray extends Array<JsonValue> {}

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
  expect?: {
    failure?: boolean;  // If true, expect the request to fail (for negative testing)
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
