/**
 * YAML serializer: CurlRunnerIR → curl-runner YAML string.
 * Produces human-readable YAML with stable key ordering and loss comments.
 */

import type { CurlRunnerIR } from './types';

/**
 * Serialize a single IR to YAML string.
 */
export function serializeToYaml(
  ir: CurlRunnerIR,
  options?: { pretty?: boolean; lossReport?: boolean },
): string {
  const lines: string[] = [];
  const pretty = options?.pretty ?? true;
  const lossReport = options?.lossReport ?? true;

  // Loss report as comments
  if (lossReport && ir.metadata.warnings.length > 0) {
    for (const w of ir.metadata.warnings) {
      lines.push(`# Warning: ${w}`);
    }
    lines.push('');
  }

  lines.push('request:');
  if (ir.name) {
    lines.push(`  name: ${yamlStr(ir.name)}`);
  }
  lines.push(`  method: ${ir.method}`);
  lines.push(`  url: ${yamlStr(ir.url)}`);

  // Params
  if (ir.params && Object.keys(ir.params).length > 0) {
    lines.push('  params:');
    for (const [k, v] of sortedEntries(ir.params)) {
      lines.push(`    ${yamlKey(k)}: ${yamlStr(v)}`);
    }
  }

  // Headers
  if (Object.keys(ir.headers).length > 0) {
    lines.push('  headers:');
    for (const [k, v] of sortedEntries(ir.headers)) {
      lines.push(`    ${yamlKey(k)}: ${yamlStr(v)}`);
    }
  }

  // Auth
  if (ir.auth) {
    lines.push('  auth:');
    lines.push(`    type: ${ir.auth.type}`);
    if (ir.auth.type === 'basic') {
      lines.push(`    username: ${yamlStr(ir.auth.username || '')}`);
      lines.push(`    password: ${yamlStr(ir.auth.password || '')}`);
    } else if (ir.auth.type === 'bearer') {
      lines.push(`    token: ${yamlStr(ir.auth.token || '')}`);
    }
  }

  // Body
  if (ir.body) {
    lines.push('  body:');
    if (ir.body.type === 'json') {
      if (pretty) {
        const jsonLines = serializeJsonToYaml(ir.body.content, 4);
        lines.push(...jsonLines);
      } else {
        lines.push(`    json: ${JSON.stringify(ir.body.content)}`);
      }
    } else if (ir.body.type === 'urlencoded') {
      lines.push(`    form: ${yamlStr(String(ir.body.content))}`);
    } else if (ir.body.type === 'binary') {
      lines.push(`    binary: ${yamlStr(String(ir.body.content))}`);
    } else {
      lines.push(`    raw: ${yamlStr(String(ir.body.content))}`);
    }
  }

  // Form data
  if (ir.formData && Object.keys(ir.formData).length > 0) {
    lines.push('  formData:');
    for (const [k, v] of sortedEntries(ir.formData)) {
      if (typeof v === 'string') {
        lines.push(`    ${yamlKey(k)}: ${yamlStr(v)}`);
      } else {
        lines.push(`    ${yamlKey(k)}:`);
        lines.push(`      file: ${yamlStr(v.file)}`);
        if (v.filename) {
          lines.push(`      filename: ${yamlStr(v.filename)}`);
        }
        if (v.contentType) {
          lines.push(`      contentType: ${yamlStr(v.contentType)}`);
        }
      }
    }
  }

  // Optional fields
  if (ir.timeout !== undefined) {
    lines.push(`  timeout: ${ir.timeout}`);
  }
  if (ir.followRedirects) {
    lines.push('  followRedirects: true');
  }
  if (ir.maxRedirects !== undefined) {
    lines.push(`  maxRedirects: ${ir.maxRedirects}`);
  }
  if (ir.proxy) {
    lines.push(`  proxy: ${yamlStr(ir.proxy)}`);
  }
  if (ir.insecure) {
    lines.push('  insecure: true');
  }
  if (ir.http2) {
    lines.push('  http2: true');
  }
  if (ir.output) {
    lines.push(`  output: ${yamlStr(ir.output)}`);
  }

  // SSL
  if (ir.ssl) {
    lines.push('  ssl:');
    if (ir.ssl.ca) {
      lines.push(`    ca: ${yamlStr(ir.ssl.ca)}`);
    }
    if (ir.ssl.cert) {
      lines.push(`    cert: ${yamlStr(ir.ssl.cert)}`);
    }
    if (ir.ssl.key) {
      lines.push(`    key: ${yamlStr(ir.ssl.key)}`);
    }
  }

  return `${lines.join('\n')}\n`;
}

/**
 * Serialize multiple IRs to a collection YAML string.
 */
export function serializeBatchToYaml(
  irs: CurlRunnerIR[],
  options?: { pretty?: boolean; lossReport?: boolean },
): string {
  const lines: string[] = [];
  const pretty = options?.pretty ?? true;
  const lossReport = options?.lossReport ?? true;

  // Collect all warnings
  if (lossReport) {
    const allWarnings = irs.flatMap((ir) => ir.metadata.warnings);
    if (allWarnings.length > 0) {
      for (const w of allWarnings) {
        lines.push(`# Warning: ${w}`);
      }
      lines.push('');
    }
  }

  lines.push('requests:');

  for (let idx = 0; idx < irs.length; idx++) {
    const ir = irs[idx];
    if (idx > 0) {
      lines.push('');
    }

    if (ir.name) {
      lines.push(`  - name: ${yamlStr(ir.name)}`);
    } else {
      lines.push(`  - name: request_${idx + 1}`);
    }

    lines.push(`    method: ${ir.method}`);
    lines.push(`    url: ${yamlStr(ir.url)}`);

    if (ir.params && Object.keys(ir.params).length > 0) {
      lines.push('    params:');
      for (const [k, v] of sortedEntries(ir.params)) {
        lines.push(`      ${yamlKey(k)}: ${yamlStr(v)}`);
      }
    }

    if (Object.keys(ir.headers).length > 0) {
      lines.push('    headers:');
      for (const [k, v] of sortedEntries(ir.headers)) {
        lines.push(`      ${yamlKey(k)}: ${yamlStr(v)}`);
      }
    }

    if (ir.auth) {
      lines.push('    auth:');
      lines.push(`      type: ${ir.auth.type}`);
      if (ir.auth.type === 'basic') {
        lines.push(`      username: ${yamlStr(ir.auth.username || '')}`);
        lines.push(`      password: ${yamlStr(ir.auth.password || '')}`);
      } else if (ir.auth.type === 'bearer') {
        lines.push(`      token: ${yamlStr(ir.auth.token || '')}`);
      }
    }

    if (ir.body) {
      lines.push('    body:');
      if (ir.body.type === 'json') {
        if (pretty) {
          const jsonLines = serializeJsonToYaml(ir.body.content, 6);
          lines.push(...jsonLines);
        } else {
          lines.push(`      json: ${JSON.stringify(ir.body.content)}`);
        }
      } else if (ir.body.type === 'urlencoded') {
        lines.push(`      form: ${yamlStr(String(ir.body.content))}`);
      } else if (ir.body.type === 'binary') {
        lines.push(`      binary: ${yamlStr(String(ir.body.content))}`);
      } else {
        lines.push(`      raw: ${yamlStr(String(ir.body.content))}`);
      }
    }

    if (ir.formData && Object.keys(ir.formData).length > 0) {
      lines.push('    formData:');
      for (const [k, v] of sortedEntries(ir.formData)) {
        if (typeof v === 'string') {
          lines.push(`      ${yamlKey(k)}: ${yamlStr(v)}`);
        } else {
          lines.push(`      ${yamlKey(k)}:`);
          lines.push(`        file: ${yamlStr(v.file)}`);
          if (v.filename) {
            lines.push(`        filename: ${yamlStr(v.filename)}`);
          }
          if (v.contentType) {
            lines.push(`        contentType: ${yamlStr(v.contentType)}`);
          }
        }
      }
    }

    if (ir.timeout !== undefined) {
      lines.push(`    timeout: ${ir.timeout}`);
    }
    if (ir.followRedirects) {
      lines.push('    followRedirects: true');
    }
    if (ir.maxRedirects !== undefined) {
      lines.push(`    maxRedirects: ${ir.maxRedirects}`);
    }
    if (ir.proxy) {
      lines.push(`    proxy: ${yamlStr(ir.proxy)}`);
    }
    if (ir.insecure) {
      lines.push('    insecure: true');
    }
    if (ir.http2) {
      lines.push('    http2: true');
    }
    if (ir.output) {
      lines.push(`    output: ${yamlStr(ir.output)}`);
    }

    if (ir.ssl) {
      lines.push('    ssl:');
      if (ir.ssl.ca) {
        lines.push(`      ca: ${yamlStr(ir.ssl.ca)}`);
      }
      if (ir.ssl.cert) {
        lines.push(`      cert: ${yamlStr(ir.ssl.cert)}`);
      }
      if (ir.ssl.key) {
        lines.push(`      key: ${yamlStr(ir.ssl.key)}`);
      }
    }
  }

  return `${lines.join('\n')}\n`;
}

/** Serialize JSON value to YAML inline, with proper indentation */
function serializeJsonToYaml(value: unknown, indent: number): string[] {
  const pad = ' '.repeat(indent);

  if (value === null || value === undefined) {
    return [`${pad}json: null`];
  }

  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
    return [`${pad}json: ${JSON.stringify(value)}`];
  }

  if (Array.isArray(value)) {
    if (value.length === 0) {
      return [`${pad}json: []`];
    }
    // For simple arrays, use inline
    if (value.every((v) => typeof v !== 'object' || v === null)) {
      return [`${pad}json: [${value.map((v) => JSON.stringify(v)).join(', ')}]`];
    }
    // Complex arrays
    return [`${pad}json: ${JSON.stringify(value)}`];
  }

  if (typeof value === 'object') {
    const obj = value as Record<string, unknown>;
    const keys = Object.keys(obj);
    if (keys.length === 0) {
      return [`${pad}json: {}`];
    }

    // Use block style for readability
    const lines: string[] = [];
    lines.push(`${pad}json:`);
    for (const key of keys) {
      const v = obj[key];
      if (v === null) {
        lines.push(`${pad}  ${yamlKey(key)}: null`);
      } else if (typeof v === 'object' && !Array.isArray(v)) {
        // Nested object → inline JSON
        lines.push(`${pad}  ${yamlKey(key)}: ${JSON.stringify(v)}`);
      } else if (Array.isArray(v)) {
        lines.push(`${pad}  ${yamlKey(key)}: ${JSON.stringify(v)}`);
      } else {
        lines.push(`${pad}  ${yamlKey(key)}: ${yamlValue(v)}`);
      }
    }
    return lines;
  }

  return [`${pad}json: ${JSON.stringify(value)}`];
}

/** Quote a YAML string value if necessary */
function yamlStr(s: string): string {
  if (s === '') {
    return '""';
  }
  // Must quote if contains special chars or looks like a YAML keyword
  if (needsQuoting(s)) {
    return `"${s.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
  }
  return s;
}

/** Quote a YAML key if necessary */
function yamlKey(s: string): string {
  if (/^[a-zA-Z_][a-zA-Z0-9_-]*$/.test(s)) {
    return s;
  }
  return `"${s.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
}

/** Format a YAML primitive value */
function yamlValue(v: unknown): string {
  if (typeof v === 'string') {
    return yamlStr(v);
  }
  if (typeof v === 'number' || typeof v === 'boolean') {
    return String(v);
  }
  return JSON.stringify(v);
}

function needsQuoting(s: string): boolean {
  // YAML special characters that need quoting (colon only when followed by space)
  if (/[{}[\],&*?|>!%#@`]/.test(s)) {
    return true;
  }
  // Colon followed by space or at end of string is a YAML indicator
  if (/: /.test(s) || s.endsWith(':')) {
    return true;
  }
  // Starts with quote char
  if (s.startsWith("'") || s.startsWith('"')) {
    return true;
  }
  // YAML keywords
  const lower = s.toLowerCase();
  if (['true', 'false', 'null', 'yes', 'no', 'on', 'off'].includes(lower)) {
    return true;
  }
  // Pure number
  if (/^-?\d+(\.\d+)?$/.test(s)) {
    return true;
  }
  // Starts with special
  if (s.startsWith(' ') || s.endsWith(' ')) {
    return true;
  }
  return false;
}

function sortedEntries<T>(obj: Record<string, T>): [string, T][] {
  return Object.entries(obj).sort(([a], [b]) => a.localeCompare(b));
}
