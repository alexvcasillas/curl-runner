/**
 * Normalization layer: CurlAST → CurlRunnerIR.
 * Infers methods, detects body types, canonicalizes headers, parses query strings.
 */

import type { AuthIR, BodyIR, CurlAST, CurlRunnerIR, FormFieldIR, FormFileIR } from './types';

/**
 * Normalize a CurlAST into the canonical CurlRunnerIR.
 */
export function normalize(ast: CurlAST): CurlRunnerIR {
  const warnings: string[] = [];

  // Parse URL and extract query params
  const { url, params } = parseUrl(ast.url);

  // Normalize headers (canonical casing)
  const headers = normalizeHeaders(ast);

  // Detect auth from headers or -u flag
  const auth = detectAuth(ast, headers);

  // Detect body
  const { body, formData } = detectBody(ast, warnings);

  // Infer method
  const method = inferMethod(ast, body, formData);

  // SSL
  const ssl = detectSsl(ast);

  // Unsupported flags → warnings
  for (const f of ast.unsupportedFlags) {
    warnings.push(`Unsupported curl flag: ${f.flag}${f.value ? ` ${f.value}` : ''}`);
  }
  if (ast.compressed) {
    warnings.push('Flag --compressed has no YAML equivalent; curl handles decompression natively');
  }
  if (ast.cookie) {
    warnings.push(`Cookie flag (-b ${ast.cookie}) stored as header; manual review recommended`);
    headers.Cookie = ast.cookie;
  }
  if (ast.cookieJar) {
    warnings.push(`Cookie jar flag (-c ${ast.cookieJar}) has no YAML equivalent`);
  }
  if (ast.userAgent && !headers['User-Agent']) {
    headers['User-Agent'] = ast.userAgent;
  }
  if (ast.referer && !headers.Referer) {
    headers.Referer = ast.referer;
  }

  const ir: CurlRunnerIR = {
    method,
    url,
    headers,
    metadata: {
      source: 'curl',
      warnings,
      unsupportedFlags: ast.unsupportedFlags.length > 0 ? ast.unsupportedFlags : undefined,
    },
  };

  if (Object.keys(params).length > 0) {
    ir.params = params;
  }
  if (body) {
    ir.body = body;
  }
  if (formData) {
    ir.formData = formData;
  }
  if (auth) {
    ir.auth = auth;
  }
  if (ast.insecure) {
    ir.insecure = true;
  }
  if (ast.location) {
    ir.followRedirects = true;
  }
  if (ast.maxRedirs !== undefined) {
    ir.maxRedirects = ast.maxRedirs;
  }
  if (ast.maxTime !== undefined) {
    ir.timeout = ast.maxTime * 1000; // seconds → ms
  }
  if (ast.proxy) {
    ir.proxy = ast.proxy;
  }
  if (ast.output) {
    ir.output = ast.output;
  }
  if (ast.http2) {
    ir.http2 = true;
  }
  if (ssl) {
    ir.ssl = ssl;
  }

  return ir;
}

function parseUrl(rawUrl: string): { url: string; params: Record<string, string> } {
  const params: Record<string, string> = {};
  if (!rawUrl) {
    return { url: '', params };
  }

  const qIdx = rawUrl.indexOf('?');
  if (qIdx === -1) {
    return { url: rawUrl, params };
  }

  const base = rawUrl.slice(0, qIdx);
  const query = rawUrl.slice(qIdx + 1);

  for (const pair of query.split('&')) {
    const eqIdx = pair.indexOf('=');
    if (eqIdx > 0) {
      const key = decodeURIComponent(pair.slice(0, eqIdx));
      const val = decodeURIComponent(pair.slice(eqIdx + 1));
      params[key] = val;
    } else if (pair.length > 0) {
      params[decodeURIComponent(pair)] = '';
    }
  }

  return { url: base, params };
}

function normalizeHeaders(ast: CurlAST): Record<string, string> {
  const headers: Record<string, string> = {};
  for (const h of ast.headers) {
    // Skip auth headers that will be detected separately
    const lower = h.key.toLowerCase();
    if (lower === 'authorization') {
      // Keep for auth detection but don't add to headers
      continue;
    }
    headers[canonicalHeaderCase(h.key)] = h.value;
  }
  return headers;
}

/** Canonical HTTP header casing: content-type → Content-Type */
function canonicalHeaderCase(header: string): string {
  return header
    .split('-')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
    .join('-');
}

function detectAuth(ast: CurlAST, headers: Record<string, string>): AuthIR | undefined {
  // Check -u flag
  if (ast.user) {
    const colonIdx = ast.user.indexOf(':');
    if (colonIdx > 0) {
      return {
        type: 'basic',
        username: ast.user.slice(0, colonIdx),
        password: ast.user.slice(colonIdx + 1),
      };
    }
    return { type: 'basic', username: ast.user, password: '' };
  }

  // Check Authorization header
  for (const h of ast.headers) {
    if (h.key.toLowerCase() === 'authorization') {
      const value = h.value.trim();
      if (value.toLowerCase().startsWith('bearer ')) {
        return { type: 'bearer', token: value.slice(7).trim() };
      }
      if (value.toLowerCase().startsWith('basic ')) {
        // Decode basic auth
        try {
          const decoded = atob(value.slice(6).trim());
          const colonIdx = decoded.indexOf(':');
          if (colonIdx > 0) {
            return {
              type: 'basic',
              username: decoded.slice(0, colonIdx),
              password: decoded.slice(colonIdx + 1),
            };
          }
        } catch {
          // Can't decode, store as-is in header
          headers.Authorization = value;
        }
      } else {
        // Unknown auth scheme, keep as header
        headers.Authorization = value;
      }
    }
  }

  return undefined;
}

function detectBody(
  ast: CurlAST,
  _warnings: string[],
): { body?: BodyIR; formData?: Record<string, FormFieldIR> } {
  // Form data (-F)
  if (ast.form && ast.form.length > 0) {
    const formData: Record<string, FormFieldIR> = {};
    for (const entry of ast.form) {
      parseFormEntry(entry, formData);
    }
    if (ast.formString) {
      for (const entry of ast.formString) {
        const eqIdx = entry.indexOf('=');
        if (eqIdx > 0) {
          formData[entry.slice(0, eqIdx)] = entry.slice(eqIdx + 1);
        }
      }
    }
    return { formData };
  }

  // URL-encoded data (--data-urlencode)
  if (ast.dataUrlencode && ast.dataUrlencode.length > 0) {
    const parts = ast.dataUrlencode.map((d) => encodeURIComponent(d));
    return { body: { type: 'urlencoded', content: parts.join('&') } };
  }

  // Binary data (--data-binary)
  if (ast.dataBinary && ast.dataBinary.length > 0) {
    if (ast.dataBinary[0].startsWith('@')) {
      return { body: { type: 'binary', content: ast.dataBinary[0] } };
    }
    return { body: { type: 'raw', content: ast.dataBinary.join('') } };
  }

  // Regular data (-d / --data / --data-raw)
  const allData = [...(ast.data || []), ...(ast.dataRaw || [])];
  if (allData.length > 0) {
    const combined = allData.join('&');

    // Try JSON parse
    try {
      const parsed = JSON.parse(combined);
      return { body: { type: 'json', content: parsed } };
    } catch {
      // Not JSON — check if form-urlencoded
      if (looksLikeFormUrlencoded(combined)) {
        return { body: { type: 'urlencoded', content: combined } };
      }
      return { body: { type: 'raw', content: combined } };
    }
  }

  return {};
}

function parseFormEntry(entry: string, formData: Record<string, FormFieldIR>): void {
  const eqIdx = entry.indexOf('=');
  if (eqIdx <= 0) {
    return;
  }

  const key = entry.slice(0, eqIdx);
  const value = entry.slice(eqIdx + 1);

  if (value.startsWith('@')) {
    // File attachment: field=@path;filename=x;type=y
    const parts = value.slice(1).split(';');
    const file: FormFileIR = { file: parts[0] };
    for (let i = 1; i < parts.length; i++) {
      const [k, v] = parts[i].split('=');
      if (k === 'filename') {
        file.filename = v;
      } else if (k === 'type') {
        file.contentType = v;
      }
    }
    formData[key] = file;
  } else {
    formData[key] = value;
  }
}

function looksLikeFormUrlencoded(s: string): boolean {
  // Simple heuristic: contains key=value pairs separated by &
  return /^[^=&]+=[^&]*(&[^=&]+=[^&]*)*$/.test(s);
}

function inferMethod(ast: CurlAST, body?: BodyIR, formData?: Record<string, FormFieldIR>): string {
  // Explicit -X overrides everything
  if (ast.method) {
    return ast.method;
  }
  // --head / -I
  if (ast.head) {
    return 'HEAD';
  }
  // --get / -G forces GET even with data
  if (ast.get) {
    return 'GET';
  }
  // Data implies POST
  if (body || formData) {
    return 'POST';
  }
  // Default
  return 'GET';
}

function detectSsl(ast: CurlAST): { ca?: string; cert?: string; key?: string } | undefined {
  if (!ast.cacert && !ast.cert && !ast.key) {
    return undefined;
  }
  const ssl: { ca?: string; cert?: string; key?: string } = {};
  if (ast.cacert) {
    ssl.ca = ast.cacert;
  }
  if (ast.cert) {
    ssl.cert = ast.cert;
  }
  if (ast.key) {
    ssl.key = ast.key;
  }
  return ssl;
}
