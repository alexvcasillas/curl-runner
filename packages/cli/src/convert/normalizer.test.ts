import { describe, expect, test } from 'bun:test';
import { parseCurl } from './curl-parser';
import { normalize } from './normalizer';

describe('normalizer', () => {
  test('simple GET preserves URL', () => {
    const ir = normalize(parseCurl('curl https://example.com'));
    expect(ir.method).toBe('GET');
    expect(ir.url).toBe('https://example.com');
    expect(ir.metadata.source).toBe('curl');
  });

  test('infers POST from -d', () => {
    const ir = normalize(parseCurl('curl -d "data" https://example.com'));
    expect(ir.method).toBe('POST');
  });

  test('explicit method overrides inference', () => {
    const ir = normalize(parseCurl('curl -X PUT -d "data" https://example.com'));
    expect(ir.method).toBe('PUT');
  });

  test('--get forces GET with data', () => {
    const ir = normalize(parseCurl('curl -G -d "q=test" https://example.com'));
    expect(ir.method).toBe('GET');
  });

  test('-I infers HEAD', () => {
    const ir = normalize(parseCurl('curl -I https://example.com'));
    expect(ir.method).toBe('HEAD');
  });

  test('extracts query params from URL', () => {
    const ir = normalize(parseCurl('curl https://example.com/api?key=val&page=2'));
    expect(ir.url).toBe('https://example.com/api');
    expect(ir.params).toEqual({ key: 'val', page: '2' });
  });

  test('detects JSON body', () => {
    const ir = normalize(parseCurl(`curl -d '{"name":"Alex","age":30}' https://example.com`));
    expect(ir.body?.type).toBe('json');
    expect(ir.body?.content).toEqual({ name: 'Alex', age: 30 });
  });

  test('detects form-urlencoded body', () => {
    const ir = normalize(parseCurl('curl -d "name=Alex&age=30" https://example.com'));
    expect(ir.body?.type).toBe('urlencoded');
    expect(ir.body?.content).toBe('name=Alex&age=30');
  });

  test('detects raw body', () => {
    const ir = normalize(parseCurl('curl -d "hello world" https://example.com'));
    expect(ir.body?.type).toBe('raw');
  });

  test('detects basic auth from -u', () => {
    const ir = normalize(parseCurl('curl -u admin:secret https://example.com'));
    expect(ir.auth).toEqual({ type: 'basic', username: 'admin', password: 'secret' });
  });

  test('detects bearer auth from header', () => {
    const ir = normalize(parseCurl('curl -H "Authorization: Bearer tok123" https://example.com'));
    expect(ir.auth).toEqual({ type: 'bearer', token: 'tok123' });
  });

  test('normalizes header casing', () => {
    const ir = normalize(parseCurl('curl -H "content-type: text/plain" https://example.com'));
    expect(ir.headers['Content-Type']).toBe('text/plain');
  });

  test('form data with file', () => {
    const ir = normalize(parseCurl('curl -F "file=@test.png" -F "name=test" https://example.com'));
    expect(ir.formData).toBeDefined();
    expect(ir.formData!.file).toEqual({ file: 'test.png' });
    expect(ir.formData!.name).toBe('test');
  });

  test('form data with metadata', () => {
    const ir = normalize(
      parseCurl('curl -F "file=@test.png;filename=photo.png;type=image/png" https://example.com'),
    );
    expect(ir.formData!.file).toEqual({
      file: 'test.png',
      filename: 'photo.png',
      contentType: 'image/png',
    });
  });

  test('insecure flag', () => {
    const ir = normalize(parseCurl('curl -k https://example.com'));
    expect(ir.insecure).toBe(true);
  });

  test('follow redirects', () => {
    const ir = normalize(parseCurl('curl -L --max-redirs 5 https://example.com'));
    expect(ir.followRedirects).toBe(true);
    expect(ir.maxRedirects).toBe(5);
  });

  test('timeout conversion (seconds to ms)', () => {
    const ir = normalize(parseCurl('curl --max-time 30 https://example.com'));
    expect(ir.timeout).toBe(30000);
  });

  test('proxy', () => {
    const ir = normalize(parseCurl('curl -x http://proxy:8080 https://example.com'));
    expect(ir.proxy).toBe('http://proxy:8080');
  });

  test('http2', () => {
    const ir = normalize(parseCurl('curl --http2 https://example.com'));
    expect(ir.http2).toBe(true);
  });

  test('SSL config', () => {
    const ir = normalize(
      parseCurl('curl --cacert ca.pem --cert client.pem --key key.pem https://example.com'),
    );
    expect(ir.ssl).toEqual({ ca: 'ca.pem', cert: 'client.pem', key: 'key.pem' });
  });

  test('unsupported flags generate warnings', () => {
    const ir = normalize(parseCurl('curl --compressed https://example.com'));
    expect(ir.metadata.warnings.length).toBeGreaterThan(0);
    expect(ir.metadata.warnings.some((w) => w.includes('compressed'))).toBe(true);
  });

  test('user-agent via -A', () => {
    const ir = normalize(parseCurl('curl -A "MyApp/1.0" https://example.com'));
    expect(ir.headers['User-Agent']).toBe('MyApp/1.0');
  });

  test('referer via -e', () => {
    const ir = normalize(parseCurl('curl -e "https://ref.com" https://example.com'));
    expect(ir.headers.Referer).toBe('https://ref.com');
  });

  test('cookie → header + warning', () => {
    const ir = normalize(parseCurl('curl -b "session=abc" https://example.com'));
    expect(ir.headers.Cookie).toBe('session=abc');
    expect(ir.metadata.warnings.some((w) => w.includes('Cookie'))).toBe(true);
  });
});
