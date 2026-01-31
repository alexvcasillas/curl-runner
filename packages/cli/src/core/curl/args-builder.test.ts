import { describe, expect, test } from 'bun:test';
import {
  buildCurlArgs,
  createBatchMarker,
  formatArgsForDisplay,
  METRICS_MARKER_END,
  METRICS_MARKER_START,
} from './args-builder';

describe('buildCurlArgs', () => {
  test('builds basic GET request', () => {
    const { args, url } = buildCurlArgs({ url: 'https://api.example.com/users' });
    expect(args).toContain('-X');
    expect(args).toContain('GET');
    expect(url).toBe('https://api.example.com/users');
  });

  test('builds POST request', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com/users',
      method: 'POST',
    });
    expect(args).toContain('-X');
    expect(args).toContain('POST');
  });

  test('includes write-out marker for metrics', () => {
    const { args } = buildCurlArgs({ url: 'https://api.example.com' });
    expect(args).toContain('-w');
    const wIdx = args.indexOf('-w');
    expect(args[wIdx + 1]).toContain(METRICS_MARKER_START);
    expect(args[wIdx + 1]).toContain(METRICS_MARKER_END);
  });

  test('includes custom write-out marker', () => {
    const customMarker = '__CUSTOM__%{json}__END__';
    const { args } = buildCurlArgs(
      { url: 'https://api.example.com' },
      { writeOutMarker: customMarker },
    );
    const wIdx = args.indexOf('-w');
    expect(args[wIdx + 1]).toBe(customMarker);
  });

  test('adds headers', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
      },
    });
    expect(args).toContain('-H');
    expect(args).toContain('Content-Type: application/json');
    expect(args).toContain('Accept: application/json');
  });

  test('adds basic auth', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com',
      auth: { type: 'basic', username: 'user', password: 'pass' },
    });
    expect(args).toContain('-u');
    expect(args).toContain('user:pass');
  });

  test('adds bearer token auth', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com',
      auth: { type: 'bearer', token: 'abc123' },
    });
    expect(args).toContain('-H');
    expect(args).toContain('Authorization: Bearer abc123');
  });

  test('adds JSON body', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com',
      method: 'POST',
      body: { name: 'test' },
    });
    expect(args).toContain('-d');
    expect(args).toContain('{"name":"test"}');
    expect(args).toContain('Content-Type: application/json');
  });

  test('adds string body', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com',
      method: 'POST',
      body: 'raw data',
    });
    expect(args).toContain('-d');
    expect(args).toContain('raw data');
  });

  test('does not add Content-Type if already present', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com',
      method: 'POST',
      headers: { 'Content-Type': 'text/plain' },
      body: 'test',
    });
    const contentTypeCount = args.filter((a) => a.includes('Content-Type')).length;
    expect(contentTypeCount).toBe(1);
    expect(args).toContain('Content-Type: text/plain');
  });

  test('adds form data fields', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com/upload',
      method: 'POST',
      formData: {
        name: 'test',
        value: 123,
      },
    });
    expect(args).toContain('--form-string');
    expect(args).toContain('name=test');
    expect(args).toContain('value=123');
  });

  test('adds file attachment', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com/upload',
      method: 'POST',
      formData: {
        file: { file: '/path/to/file.txt' },
      },
    });
    expect(args).toContain('-F');
    expect(args).toContain('file=@/path/to/file.txt');
  });

  test('adds file attachment with filename and content type', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com/upload',
      method: 'POST',
      formData: {
        file: {
          file: '/path/to/file.txt',
          filename: 'custom.txt',
          contentType: 'text/plain',
        },
      },
    });
    expect(args).toContain('-F');
    expect(args).toContain('file=@/path/to/file.txt;filename=custom.txt;type=text/plain');
  });

  test('adds timeout', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com',
      timeout: 30,
    });
    expect(args).toContain('--max-time');
    expect(args).toContain('30');
  });

  test('adds follow redirects by default', () => {
    const { args } = buildCurlArgs({ url: 'https://api.example.com' });
    expect(args).toContain('-L');
  });

  test('does not add follow redirects when disabled', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com',
      followRedirects: false,
    });
    expect(args).not.toContain('-L');
  });

  test('adds max redirects', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com',
      maxRedirects: 5,
    });
    expect(args).toContain('--max-redirs');
    expect(args).toContain('5');
  });

  test('adds proxy', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com',
      proxy: 'http://proxy:8080',
    });
    expect(args).toContain('-x');
    expect(args).toContain('http://proxy:8080');
  });

  test('adds insecure flag', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com',
      insecure: true,
    });
    expect(args).toContain('-k');
  });

  test('adds SSL certificate options', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com',
      ssl: {
        ca: '/path/to/ca.pem',
        cert: '/path/to/cert.pem',
        key: '/path/to/key.pem',
      },
    });
    expect(args).toContain('--cacert');
    expect(args).toContain('/path/to/ca.pem');
    expect(args).toContain('--cert');
    expect(args).toContain('/path/to/cert.pem');
    expect(args).toContain('--key');
    expect(args).toContain('/path/to/key.pem');
  });

  test('adds insecure from ssl.verify=false', () => {
    const { args } = buildCurlArgs({
      url: 'https://api.example.com',
      ssl: { verify: false },
    });
    expect(args).toContain('-k');
  });

  test('adds output file when includeOutputFlag is true', () => {
    const { args } = buildCurlArgs(
      { url: 'https://api.example.com', output: '/path/to/output' },
      { includeOutputFlag: true },
    );
    expect(args).toContain('-o');
    expect(args).toContain('/path/to/output');
  });

  test('does not add output file when includeOutputFlag is false', () => {
    const { args } = buildCurlArgs(
      { url: 'https://api.example.com', output: '/path/to/output' },
      { includeOutputFlag: false },
    );
    expect(args).not.toContain('-o');
  });

  test('adds HTTP/2 flag when enabled', () => {
    const { args } = buildCurlArgs(
      { url: 'https://api.example.com', http2: true },
      { includeHttp2Flag: true },
    );
    expect(args).toContain('--http2');
  });

  test('does not add HTTP/2 flag when includeHttp2Flag is false', () => {
    const { args } = buildCurlArgs(
      { url: 'https://api.example.com', http2: true },
      { includeHttp2Flag: false },
    );
    expect(args).not.toContain('--http2');
  });

  test('adds silent flags by default', () => {
    const { args } = buildCurlArgs({ url: 'https://api.example.com' });
    expect(args).toContain('-s');
    expect(args).toContain('-S');
  });

  test('does not add silent flags when disabled', () => {
    const { args } = buildCurlArgs(
      { url: 'https://api.example.com' },
      { includeSilentFlags: false },
    );
    expect(args).not.toContain('-s');
    expect(args).not.toContain('-S');
  });

  test('appends query params to URL', () => {
    const { url } = buildCurlArgs({
      url: 'https://api.example.com/search',
      params: { q: 'test', limit: '10' },
    });
    expect(url).toBe('https://api.example.com/search?q=test&limit=10');
  });

  test('appends query params to URL with existing params', () => {
    const { url } = buildCurlArgs({
      url: 'https://api.example.com/search?existing=true',
      params: { q: 'test' },
    });
    expect(url).toBe('https://api.example.com/search?existing=true&q=test');
  });
});

describe('formatArgsForDisplay', () => {
  test('formats simple args', () => {
    const result = formatArgsForDisplay(['-X', 'GET', 'https://api.example.com']);
    expect(result).toBe('curl -X GET https://api.example.com');
  });

  test('quotes args with spaces', () => {
    const result = formatArgsForDisplay(['-H', 'Content-Type: application/json']);
    expect(result).toBe("curl -H 'Content-Type: application/json'");
  });

  test('escapes single quotes in args', () => {
    const result = formatArgsForDisplay(['-d', "it's data"]);
    expect(result).toBe("curl -d 'it'\\''s data'");
  });

  test('quotes args with double quotes', () => {
    const result = formatArgsForDisplay(['-d', '{"key":"value"}']);
    expect(result).toBe('curl -d \'{"key":"value"}\'');
  });
});

describe('createBatchMarker', () => {
  test('creates marker for index 0', () => {
    const marker = createBatchMarker(0);
    expect(marker).toContain('__CURL_BATCH_0_START__');
    expect(marker).toContain('__CURL_BATCH_0_END__');
    expect(marker).toContain('%{json}');
  });

  test('creates marker for index 5', () => {
    const marker = createBatchMarker(5);
    expect(marker).toContain('__CURL_BATCH_5_START__');
    expect(marker).toContain('__CURL_BATCH_5_END__');
  });

  test('creates unique markers for different indices', () => {
    const marker0 = createBatchMarker(0);
    const marker1 = createBatchMarker(1);
    expect(marker0).not.toBe(marker1);
    expect(marker0).toContain('_0_');
    expect(marker1).toContain('_1_');
  });
});

describe('marker constants', () => {
  test('METRICS_MARKER_START is defined', () => {
    expect(METRICS_MARKER_START).toBe('__CURL_METRICS_START__');
  });

  test('METRICS_MARKER_END is defined', () => {
    expect(METRICS_MARKER_END).toBe('__CURL_METRICS_END__');
  });
});
