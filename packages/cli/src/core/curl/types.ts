/**
 * Shared types for curl command building and response parsing.
 */

import type { FileAttachment, FormFieldValue } from '../../types/config';

/**
 * Raw metrics returned by curl's JSON write-out format.
 */
export interface CurlMetrics {
  response_code?: number;
  http_code?: number;
  time_total?: number;
  size_download?: number;
  time_namelookup?: number;
  time_connect?: number;
  time_appconnect?: number;
  time_starttransfer?: number;
}

/**
 * Processed metrics with human-friendly naming (in milliseconds).
 */
export interface ProcessedMetrics {
  duration: number;
  size?: number;
  dnsLookup?: number;
  tcpConnection?: number;
  tlsHandshake?: number;
  firstByte?: number;
  download?: number;
}

/**
 * Options for building curl arguments.
 */
export interface CurlArgsOptions {
  /** Custom write-out format marker (for batching) */
  writeOutMarker?: string;
  /** Whether to include -s -S flags */
  includeSilentFlags?: boolean;
  /** Whether to include HTTP/2 flag from config */
  includeHttp2Flag?: boolean;
  /** Whether to include output file flag */
  includeOutputFlag?: boolean;
}

/**
 * Result of building curl arguments.
 */
export interface CurlArgsResult {
  args: string[];
  url: string;
}

/**
 * Result of executing curl.
 */
export interface CurlExecutionResult {
  success: boolean;
  status?: number;
  headers?: Record<string, string>;
  body?: string;
  metrics?: ProcessedMetrics;
  error?: string;
}

/**
 * Type guard to check if a form field value is a file attachment.
 */
export function isFileAttachment(value: FormFieldValue): value is FileAttachment {
  return typeof value === 'object' && value !== null && 'file' in value;
}
