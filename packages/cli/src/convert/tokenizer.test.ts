import { describe, expect, test } from 'bun:test';
import { tokenize } from './tokenizer';

describe('tokenizer', () => {
  test('simple command', () => {
    expect(tokenize('curl https://example.com')).toEqual(['curl', 'https://example.com']);
  });

  test('single-quoted strings', () => {
    expect(tokenize("curl -d '{\"key\":\"value\"}'")).toEqual([
      'curl', '-d', '{"key":"value"}',
    ]);
  });

  test('double-quoted strings', () => {
    expect(tokenize('curl -H "Content-Type: application/json"')).toEqual([
      'curl', '-H', 'Content-Type: application/json',
    ]);
  });

  test('escaped characters in double quotes', () => {
    expect(tokenize('curl -d "hello \\"world\\""')).toEqual([
      'curl', '-d', 'hello "world"',
    ]);
  });

  test('backslash line continuation', () => {
    expect(tokenize('curl \\\n  -X POST \\\n  https://example.com')).toEqual([
      'curl', '-X', 'POST', 'https://example.com',
    ]);
  });

  test('mixed quoting', () => {
    expect(tokenize(`curl -H 'Accept: */*' -d "test"`)).toEqual([
      'curl', '-H', 'Accept: */*', '-d', 'test',
    ]);
  });

  test('empty input', () => {
    expect(tokenize('')).toEqual([]);
  });

  test('tabs and multiple spaces', () => {
    expect(tokenize('curl   -X   GET\thttps://example.com')).toEqual([
      'curl', '-X', 'GET', 'https://example.com',
    ]);
  });

  test('backslash escape outside quotes', () => {
    expect(tokenize('curl hello\\ world')).toEqual(['curl', 'hello world']);
  });

  test('windows line endings in continuation', () => {
    expect(tokenize('curl \\\r\n  -X POST')).toEqual(['curl', '-X', 'POST']);
  });

  test('adjacent quoted segments', () => {
    expect(tokenize(`curl 'hello'"world"`)).toEqual(['curl', 'helloworld']);
  });
});
