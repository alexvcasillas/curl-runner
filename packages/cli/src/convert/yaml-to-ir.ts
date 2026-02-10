/**
 * YAML → IR converter: parses a curl-runner YAML file into CurlRunnerIR.
 */

import type { RequestConfig } from '../types/config';
import type { CurlRunnerIR, FormFieldIR } from './types';

/**
 * Convert a RequestConfig (parsed from YAML) to CurlRunnerIR.
 */
export function yamlToIR(config: RequestConfig): CurlRunnerIR {
  const warnings: string[] = [];
  const headers: Record<string, string> = {};

  if (config.headers) {
    for (const [k, v] of Object.entries(config.headers)) {
      headers[k] = v;
    }
  }

  const ir: CurlRunnerIR = {
    method: config.method || 'GET',
    url: config.url,
    headers,
    metadata: { source: 'yaml', warnings },
  };

  if (config.name) ir.name = config.name;
  if (config.params) ir.params = { ...config.params };

  // Auth
  if (config.auth) {
    ir.auth = {
      type: config.auth.type,
      username: config.auth.username,
      password: config.auth.password,
      token: config.auth.token,
    };
  }

  // Body
  if (config.body !== undefined) {
    if (typeof config.body === 'object' && config.body !== null && !Array.isArray(config.body)) {
      const bodyObj = config.body as Record<string, unknown>;
      if ('json' in bodyObj) {
        ir.body = { type: 'json', content: bodyObj.json };
      } else if ('form' in bodyObj) {
        ir.body = { type: 'urlencoded', content: String(bodyObj.form) };
      } else if ('raw' in bodyObj) {
        ir.body = { type: 'raw', content: String(bodyObj.raw) };
      } else if ('binary' in bodyObj) {
        ir.body = { type: 'binary', content: String(bodyObj.binary) };
      } else {
        // Treat entire object as JSON body
        ir.body = { type: 'json', content: config.body };
      }
    } else {
      // Primitive or array body → JSON
      ir.body = { type: 'json', content: config.body };
    }
  }

  // Form data
  if (config.formData) {
    const formData: Record<string, FormFieldIR> = {};
    for (const [k, v] of Object.entries(config.formData)) {
      if (typeof v === 'object' && v !== null && 'file' in v) {
        formData[k] = {
          file: v.file,
          filename: v.filename,
          contentType: v.contentType,
        };
      } else {
        formData[k] = String(v);
      }
    }
    ir.formData = formData;
  }

  if (config.timeout) ir.timeout = config.timeout;
  if (config.followRedirects !== undefined) ir.followRedirects = config.followRedirects;
  if (config.maxRedirects) ir.maxRedirects = config.maxRedirects;
  if (config.proxy) ir.proxy = config.proxy;
  if (config.insecure) ir.insecure = true;
  if (config.http2) ir.http2 = true;
  if (config.output) ir.output = config.output;

  if (config.ssl) {
    ir.ssl = {};
    if (config.ssl.ca) ir.ssl.ca = config.ssl.ca;
    if (config.ssl.cert) ir.ssl.cert = config.ssl.cert;
    if (config.ssl.key) ir.ssl.key = config.ssl.key;
  }

  // Warn about YAML-only features that don't map to curl
  if (config.expect) warnings.push('expect block has no curl equivalent');
  if (config.store) warnings.push('store block has no curl equivalent');
  if (config.when) warnings.push('when condition has no curl equivalent');
  if (config.retry) warnings.push('retry config simplified in curl output');
  if (config.snapshot) warnings.push('snapshot config has no curl equivalent');
  if (config.diff) warnings.push('diff config has no curl equivalent');

  return ir;
}
