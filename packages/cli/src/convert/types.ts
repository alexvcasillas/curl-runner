/**
 * Types for the curl ⇄ YAML conversion engine.
 */

/** Parsed curl command AST */
export interface CurlAST {
  url: string;
  method?: string;
  headers: Array<{ key: string; value: string }>;
  data?: string[];
  dataUrlencode?: string[];
  dataRaw?: string[];
  dataBinary?: string[];
  form?: string[];
  formString?: string[];
  user?: string;
  get?: boolean;
  head?: boolean;
  compressed?: boolean;
  insecure?: boolean;
  location?: boolean;
  maxRedirs?: number;
  maxTime?: number;
  proxy?: string;
  output?: string;
  http2?: boolean;
  cacert?: string;
  cert?: string;
  key?: string;
  cookie?: string;
  cookieJar?: string;
  userAgent?: string;
  referer?: string;
  silent?: boolean;
  verbose?: boolean;
  unsupportedFlags: Array<{ flag: string; value?: string }>;
}

/** Body type in IR */
export type BodyType = 'json' | 'form' | 'urlencoded' | 'raw' | 'binary';

/** Body IR */
export interface BodyIR {
  type: BodyType;
  content: unknown;
}

/** Auth IR */
export interface AuthIR {
  type: 'basic' | 'bearer';
  username?: string;
  password?: string;
  token?: string;
}

/** File attachment in form data IR */
export interface FormFileIR {
  file: string;
  filename?: string;
  contentType?: string;
}

/** Form field IR */
export type FormFieldIR = string | FormFileIR;

/** Intermediate representation shared by both converters */
export interface CurlRunnerIR {
  name?: string;
  method: string;
  url: string;
  headers: Record<string, string>;
  params?: Record<string, string>;
  body?: BodyIR;
  formData?: Record<string, FormFieldIR>;
  auth?: AuthIR;
  insecure?: boolean;
  followRedirects?: boolean;
  maxRedirects?: number;
  timeout?: number;
  proxy?: string;
  output?: string;
  http2?: boolean;
  ssl?: {
    ca?: string;
    cert?: string;
    key?: string;
  };
  metadata: {
    source: 'curl' | 'yaml';
    warnings: string[];
    unsupportedFlags?: Array<{ flag: string; value?: string }>;
  };
}

/** Convert command options */
export interface ConvertOptions {
  output?: string;
  pretty?: boolean;
  normalize?: boolean;
  lossReport?: boolean;
  batch?: boolean;
  envDetect?: boolean;
  debug?: boolean;
}

/** Batch conversion result */
export interface BatchResult {
  requests: CurlRunnerIR[];
  warnings: string[];
}
