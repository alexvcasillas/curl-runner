/**
 * Wizard types and interfaces.
 */

export interface WizardAnswers {
  // Request basics
  name?: string;
  url: string;
  method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'HEAD' | 'OPTIONS';

  // Headers
  headers?: Record<string, string>;

  // Body
  bodyType?: 'none' | 'json' | 'form' | 'raw';
  body?: unknown;
  formData?: Record<string, string | { file: string }>;

  // Auth
  authType?: 'none' | 'basic' | 'bearer';
  username?: string;
  password?: string;
  token?: string;

  // Query params
  params?: Record<string, string>;

  // Advanced
  timeout?: number;
  followRedirects?: boolean;
  maxRedirects?: number;
  insecure?: boolean;
  http2?: boolean;

  // Retry
  retryEnabled?: boolean;
  retryCount?: number;
  retryDelay?: number;
  retryBackoff?: number;

  // Validation
  expectStatus?: number | number[];
  expectResponseTime?: string;

  // Output
  outputFile?: string;
  verbose?: boolean;
  showHeaders?: boolean;
  showBody?: boolean;
}

export interface WizardOptions {
  /** File to edit (edit mode) */
  editFile?: string;
  /** Template to start from */
  template?: string;
  /** Run request after creating */
  runAfter?: boolean;
  /** Output file path */
  outputPath?: string;
}

export type WizardTemplate = 'basic-get' | 'basic-post' | 'api-test' | 'file-upload' | 'auth-flow';

export interface TemplateConfig {
  name: string;
  description: string;
  defaults: Partial<WizardAnswers>;
}
