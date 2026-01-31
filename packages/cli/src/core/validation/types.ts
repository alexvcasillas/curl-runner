/**
 * Types for response validation.
 */

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

export interface ValidationResult {
  success: boolean;
  error?: string;
}

export interface ValueValidationResult {
  isValid: boolean;
  error?: string;
}

export interface ExpectConfig {
  status?: number | number[];
  headers?: Record<string, string>;
  body?: JsonValue;
  responseTime?: string;
  failure?: boolean;
}

export interface ResponseData {
  status?: number;
  headers?: Record<string, string>;
  body?: JsonValue;
  metrics?: {
    duration: number;
  };
}
