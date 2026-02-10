/**
 * Curl generator: YAML (RequestConfig) → canonical curl command string.
 * Produces deterministic, shell-safe curl commands.
 */

import type { RequestConfig } from '../types/config';
import type { CurlRunnerIR } from './types';

/**
 * Generate a curl command from a RequestConfig (YAML parsed).
 */
export function generateCurlFromYaml(config: RequestConfig): string {
  const segments: string[] = ['curl'];

  const method = config.method || 'GET';
  if (method !== 'GET' || config.body || config.formData) {
    segments.push(`-X ${method}`);
  }

  if (config.headers) {
    for (const [key, value] of Object.entries(config.headers)) {
      segments.push(`-H ${shellQuote(`${key}: ${value}`)}`);
    }
  }

  if (config.auth) {
    if (config.auth.type === 'basic' && config.auth.username) {
      segments.push(`-u ${shellQuote(`${config.auth.username}:${config.auth.password || ''}`)}`);
    } else if (config.auth.type === 'bearer' && config.auth.token) {
      segments.push(`-H ${shellQuote(`Authorization: Bearer ${config.auth.token}`)}`);
    }
  }

  if (config.formData) {
    for (const [fieldName, fieldValue] of Object.entries(config.formData)) {
      if (typeof fieldValue === 'object' && fieldValue !== null && 'file' in fieldValue) {
        let fileSpec = `@${fieldValue.file}`;
        if (fieldValue.filename) fileSpec += `;filename=${fieldValue.filename}`;
        if (fieldValue.contentType) fileSpec += `;type=${fieldValue.contentType}`;
        segments.push(`-F ${shellQuote(`${fieldName}=${fileSpec}`)}`);
      } else {
        segments.push(`--form-string ${shellQuote(`${fieldName}=${String(fieldValue)}`)}`);
      }
    }
  } else if (config.body !== undefined) {
    const bodyStr = typeof config.body === 'string' ? config.body : JSON.stringify(config.body);
    segments.push(`-d ${shellQuote(bodyStr)}`);
    if (!config.headers?.['Content-Type'] && typeof config.body !== 'string') {
      segments.push(`-H ${shellQuote('Content-Type: application/json')}`);
    }
  }

  if (config.timeout) {
    segments.push(`--max-time ${Math.ceil(config.timeout / 1000)}`);
  }

  if (config.followRedirects !== false) {
    segments.push('-L');
  }
  if (config.maxRedirects) {
    segments.push(`--max-redirs ${config.maxRedirects}`);
  }

  if (config.proxy) segments.push(`-x ${shellQuote(config.proxy)}`);
  if (config.insecure) segments.push('-k');
  if (config.ssl) {
    if (config.ssl.verify === false) segments.push('-k');
    if (config.ssl.ca) segments.push(`--cacert ${shellQuote(config.ssl.ca)}`);
    if (config.ssl.cert) segments.push(`--cert ${shellQuote(config.ssl.cert)}`);
    if (config.ssl.key) segments.push(`--key ${shellQuote(config.ssl.key)}`);
  }
  if (config.http2) segments.push('--http2');
  if (config.output) segments.push(`-o ${shellQuote(config.output)}`);

  let url = config.url;
  if (config.params && Object.keys(config.params).length > 0) {
    const qs = new URLSearchParams(config.params).toString();
    url += (url.includes('?') ? '&' : '?') + qs;
  }
  segments.push(shellQuote(url));

  return segments.join(' \\\n  ');
}

/**
 * Generate a curl command from a CurlRunnerIR.
 */
export function generateCurlFromIR(ir: CurlRunnerIR): string {
  const segments: string[] = ['curl'];

  if (ir.method !== 'GET' || ir.body || ir.formData) {
    segments.push(`-X ${ir.method}`);
  }

  for (const [key, value] of Object.entries(ir.headers)) {
    segments.push(`-H ${shellQuote(`${key}: ${value}`)}`);
  }

  if (ir.auth) {
    if (ir.auth.type === 'basic') {
      segments.push(`-u ${shellQuote(`${ir.auth.username || ''}:${ir.auth.password || ''}`)}`);
    } else if (ir.auth.type === 'bearer') {
      segments.push(`-H ${shellQuote(`Authorization: Bearer ${ir.auth.token || ''}`)}`);
    }
  }

  if (ir.formData) {
    for (const [fieldName, fieldValue] of Object.entries(ir.formData)) {
      if (typeof fieldValue === 'string') {
        segments.push(`--form-string ${shellQuote(`${fieldName}=${fieldValue}`)}`);
      } else {
        let fileSpec = `@${fieldValue.file}`;
        if (fieldValue.filename) fileSpec += `;filename=${fieldValue.filename}`;
        if (fieldValue.contentType) fileSpec += `;type=${fieldValue.contentType}`;
        segments.push(`-F ${shellQuote(`${fieldName}=${fileSpec}`)}`);
      }
    }
  } else if (ir.body) {
    if (ir.body.type === 'json') {
      segments.push(`-d ${shellQuote(JSON.stringify(ir.body.content))}`);
      if (!ir.headers['Content-Type']) {
        segments.push(`-H ${shellQuote('Content-Type: application/json')}`);
      }
    } else if (ir.body.type === 'urlencoded') {
      segments.push(`--data-urlencode ${shellQuote(String(ir.body.content))}`);
    } else if (ir.body.type === 'binary') {
      segments.push(`--data-binary ${shellQuote(String(ir.body.content))}`);
    } else {
      segments.push(`-d ${shellQuote(String(ir.body.content))}`);
    }
  }

  if (ir.timeout !== undefined) {
    segments.push(`--max-time ${Math.ceil(ir.timeout / 1000)}`);
  }

  if (ir.followRedirects) segments.push('-L');
  if (ir.maxRedirects !== undefined) segments.push(`--max-redirs ${ir.maxRedirects}`);
  if (ir.proxy) segments.push(`-x ${shellQuote(ir.proxy)}`);
  if (ir.insecure) segments.push('-k');
  if (ir.http2) segments.push('--http2');
  if (ir.output) segments.push(`-o ${shellQuote(ir.output)}`);

  if (ir.ssl) {
    if (ir.ssl.ca) segments.push(`--cacert ${shellQuote(ir.ssl.ca)}`);
    if (ir.ssl.cert) segments.push(`--cert ${shellQuote(ir.ssl.cert)}`);
    if (ir.ssl.key) segments.push(`--key ${shellQuote(ir.ssl.key)}`);
  }

  let url = ir.url;
  if (ir.params && Object.keys(ir.params).length > 0) {
    const qs = new URLSearchParams(ir.params).toString();
    url += (url.includes('?') ? '&' : '?') + qs;
  }
  segments.push(shellQuote(url));

  return segments.join(' \\\n  ');
}

/** Shell-safe quoting for a value */
function shellQuote(s: string): string {
  // If it contains single quotes, use double quoting with escapes
  if (s.includes("'")) {
    return `"${s.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\$/g, '\\$').replace(/`/g, '\\`')}"`;
  }
  // If it contains spaces or special chars that need quoting, use single quotes
  if (/[^a-zA-Z0-9_./:@=,+%-]/.test(s)) {
    return `'${s}'`;
  }
  return s;
}
