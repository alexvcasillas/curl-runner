import { describe, expect, test } from 'bun:test';
import { parseCurl } from './curl-parser';

describe('curl-parser', () => {
  test('simple GET', () => {
    const ast = parseCurl('curl https://example.com');
    expect(ast.url).toBe('https://example.com');
    expect(ast.method).toBeUndefined();
    expect(ast.headers).toEqual([]);
  });

  test('explicit method', () => {
    const ast = parseCurl('curl -X POST https://api.example.com');
    expect(ast.method).toBe('POST');
    expect(ast.url).toBe('https://api.example.com');
  });

  test('headers', () => {
    const ast = parseCurl(
      'curl -H "Content-Type: application/json" -H "Accept: text/plain" https://example.com',
    );
    expect(ast.headers).toEqual([
      { key: 'Content-Type', value: 'application/json' },
      { key: 'Accept', value: 'text/plain' },
    ]);
  });

  test('data flag', () => {
    const ast = parseCurl(`curl -d '{"name":"Alex"}' https://example.com`);
    expect(ast.data).toEqual(['{"name":"Alex"}']);
  });

  test('multiple data flags', () => {
    const ast = parseCurl('curl -d "a=1" -d "b=2" https://example.com');
    expect(ast.data).toEqual(['a=1', 'b=2']);
  });

  test('form data', () => {
    const ast = parseCurl('curl -F "file=@test.png" -F "name=test" https://example.com');
    expect(ast.form).toEqual(['file=@test.png', 'name=test']);
  });

  test('basic auth', () => {
    const ast = parseCurl('curl -u admin:secret https://example.com');
    expect(ast.user).toBe('admin:secret');
  });

  test('bearer token via header', () => {
    const ast = parseCurl('curl -H "Authorization: Bearer mytoken123" https://example.com');
    expect(ast.headers).toEqual([{ key: 'Authorization', value: 'Bearer mytoken123' }]);
  });

  test('insecure flag', () => {
    const ast = parseCurl('curl -k https://self-signed.example.com');
    expect(ast.insecure).toBe(true);
  });

  test('follow redirects', () => {
    const ast = parseCurl('curl -L https://example.com');
    expect(ast.location).toBe(true);
  });

  test('combined short flags -sSL', () => {
    const ast = parseCurl('curl -sSL https://example.com');
    expect(ast.silent).toBe(true);
    expect(ast.location).toBe(true);
  });

  test('combined value flag -XPOST', () => {
    const ast = parseCurl('curl -XPOST https://example.com');
    expect(ast.method).toBe('POST');
  });

  test('timeout', () => {
    const ast = parseCurl('curl --max-time 30 https://example.com');
    expect(ast.maxTime).toBe(30);
  });

  test('proxy', () => {
    const ast = parseCurl('curl -x http://proxy:8080 https://example.com');
    expect(ast.proxy).toBe('http://proxy:8080');
  });

  test('http2', () => {
    const ast = parseCurl('curl --http2 https://example.com');
    expect(ast.http2).toBe(true);
  });

  test('SSL certificates', () => {
    const ast = parseCurl(
      'curl --cacert ca.pem --cert client.pem --key client-key.pem https://example.com',
    );
    expect(ast.cacert).toBe('ca.pem');
    expect(ast.cert).toBe('client.pem');
    expect(ast.key).toBe('client-key.pem');
  });

  test('output file', () => {
    const ast = parseCurl('curl -o output.json https://example.com');
    expect(ast.output).toBe('output.json');
  });

  test('user agent', () => {
    const ast = parseCurl('curl -A "MyApp/1.0" https://example.com');
    expect(ast.userAgent).toBe('MyApp/1.0');
  });

  test('unknown flags stored', () => {
    const ast = parseCurl('curl --unknown-flag value https://example.com');
    expect(ast.unsupportedFlags.length).toBeGreaterThan(0);
  });

  test('multiline command', () => {
    const ast = parseCurl(
      `curl -X POST \\\n  -H "Content-Type: application/json" \\\n  -d '{"test":true}' \\\n  https://api.example.com`,
    );
    expect(ast.method).toBe('POST');
    expect(ast.url).toBe('https://api.example.com');
    expect(ast.data).toEqual(['{"test":true}']);
  });

  test('--head flag', () => {
    const ast = parseCurl('curl -I https://example.com');
    expect(ast.head).toBe(true);
  });

  test('--get flag', () => {
    const ast = parseCurl('curl -G -d "q=test" https://example.com');
    expect(ast.get).toBe(true);
    expect(ast.data).toEqual(['q=test']);
  });

  test('--data-urlencode', () => {
    const ast = parseCurl('curl --data-urlencode "name=John Doe" https://example.com');
    expect(ast.dataUrlencode).toEqual(['name=John Doe']);
  });

  test('--data-raw', () => {
    const ast = parseCurl('curl --data-raw "@not-a-file" https://example.com');
    expect(ast.dataRaw).toEqual(['@not-a-file']);
  });

  test('--compressed flag', () => {
    const ast = parseCurl('curl --compressed https://example.com');
    expect(ast.compressed).toBe(true);
  });

  test('cookie flags', () => {
    const ast = parseCurl('curl -b "session=abc123" -c cookies.txt https://example.com');
    expect(ast.cookie).toBe('session=abc123');
    expect(ast.cookieJar).toBe('cookies.txt');
  });

  test('max-redirs', () => {
    const ast = parseCurl('curl -L --max-redirs 5 https://example.com');
    expect(ast.location).toBe(true);
    expect(ast.maxRedirs).toBe(5);
  });
});
