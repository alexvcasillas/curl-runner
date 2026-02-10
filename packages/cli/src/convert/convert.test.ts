import { describe, expect, test } from 'bun:test';
import type { RequestConfig } from '../types/config';
import { extractCurlCommands } from './batch-parser';
import { convertCurlToYaml, parseConvertArgs } from './convert-command';
import { generateCurlFromIR } from './curl-generator';
import { parseCurl } from './curl-parser';
import { normalize } from './normalizer';
import { serializeBatchToYaml, serializeToYaml } from './yaml-serializer';
import { yamlToIR } from './yaml-to-ir';

describe('convert: curl → YAML → curl round-trip', () => {
  test('simple GET round-trip', () => {
    const original = 'curl https://api.example.com/users';
    const ir = normalize(parseCurl(original));
    const curl = generateCurlFromIR(ir);
    expect(curl).toContain('https://api.example.com/users');
    expect(curl).not.toContain('-X');
  });

  test('POST with JSON body round-trip', () => {
    const original = `curl -X POST -H "Content-Type: application/json" -d '{"name":"Alex"}' https://api.example.com/users`;
    const ir = normalize(parseCurl(original));
    const curl = generateCurlFromIR(ir);
    expect(curl).toContain('-X POST');
    expect(curl).toContain('{"name":"Alex"}');
    expect(curl).toContain('https://api.example.com/users');
  });

  test('bearer auth round-trip', () => {
    const original = 'curl -H "Authorization: Bearer TOKEN123" https://api.example.com';
    const ir = normalize(parseCurl(original));
    const curl = generateCurlFromIR(ir);
    expect(curl).toContain('Bearer TOKEN123');
  });

  test('basic auth round-trip', () => {
    const original = 'curl -u admin:secret https://api.example.com';
    const ir = normalize(parseCurl(original));
    const curl = generateCurlFromIR(ir);
    expect(curl).toContain('admin:secret');
  });
});

describe('convert: YAML → IR → curl', () => {
  test('simple request', () => {
    const config: RequestConfig = {
      url: 'https://api.example.com',
      method: 'GET',
    };
    const ir = yamlToIR(config);
    const curl = generateCurlFromIR(ir);
    expect(curl).toContain('https://api.example.com');
  });

  test('POST with JSON body', () => {
    const config: RequestConfig = {
      url: 'https://api.example.com',
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: { name: 'Alex', age: 30 },
    };
    const ir = yamlToIR(config);
    const curl = generateCurlFromIR(ir);
    expect(curl).toContain('-X POST');
    expect(curl).toContain('"name":"Alex"');
  });

  test('with auth', () => {
    const config: RequestConfig = {
      url: 'https://api.example.com',
      method: 'GET',
      auth: { type: 'bearer', token: 'mytoken' },
    };
    const ir = yamlToIR(config);
    const curl = generateCurlFromIR(ir);
    expect(curl).toContain('Bearer mytoken');
  });

  test('with params', () => {
    const config: RequestConfig = {
      url: 'https://api.example.com/search',
      params: { q: 'test', page: '1' },
    };
    const ir = yamlToIR(config);
    const curl = generateCurlFromIR(ir);
    expect(curl).toContain('q=test');
    expect(curl).toContain('page=1');
  });

  test('YAML-only features generate warnings', () => {
    const config: RequestConfig = {
      url: 'https://api.example.com',
      expect: { status: 200 },
      store: { userId: 'body.id' },
    };
    const ir = yamlToIR(config);
    expect(ir.metadata.warnings).toContain('expect block has no curl equivalent');
    expect(ir.metadata.warnings).toContain('store block has no curl equivalent');
  });
});

describe('convert: YAML serializer', () => {
  test('produces valid YAML structure', () => {
    const ir = normalize(parseCurl('curl -X POST -d \'{"name":"Alex"}\' https://api.example.com'));
    const yaml = serializeToYaml(ir);
    expect(yaml).toContain('request:');
    expect(yaml).toContain('method: POST');
    expect(yaml).toContain('url: https://api.example.com');
  });

  test('includes headers', () => {
    const ir = normalize(parseCurl('curl -H "Accept: text/plain" https://example.com'));
    const yaml = serializeToYaml(ir);
    expect(yaml).toContain('headers:');
    expect(yaml).toContain('Accept: text/plain');
  });

  test('includes auth block', () => {
    const ir = normalize(parseCurl('curl -u admin:pass https://example.com'));
    const yaml = serializeToYaml(ir);
    expect(yaml).toContain('auth:');
    expect(yaml).toContain('type: basic');
    expect(yaml).toContain('username: admin');
    expect(yaml).toContain('password: pass');
  });

  test('includes JSON body', () => {
    const ir = normalize(parseCurl(`curl -d '{"name":"Alex"}' https://example.com`));
    const yaml = serializeToYaml(ir);
    expect(yaml).toContain('body:');
    expect(yaml).toContain('json:');
    expect(yaml).toContain('name: Alex');
  });

  test('includes warnings as comments', () => {
    const ir = normalize(parseCurl('curl --compressed https://example.com'));
    const yaml = serializeToYaml(ir, { lossReport: true });
    expect(yaml).toContain('# Warning:');
  });

  test('batch serializer', () => {
    const ir1 = normalize(parseCurl('curl https://example.com/a'));
    const ir2 = normalize(parseCurl('curl https://example.com/b'));
    ir1.name = 'first';
    ir2.name = 'second';
    const yaml = serializeBatchToYaml([ir1, ir2]);
    expect(yaml).toContain('requests:');
    expect(yaml).toContain('name: first');
    expect(yaml).toContain('name: second');
  });
});

describe('convert: batch parser', () => {
  test('extracts curl from script', () => {
    const script = `#!/bin/bash
# API tests
curl https://api.example.com/health
curl -X POST https://api.example.com/users -d '{"name":"test"}'
`;
    const cmds = extractCurlCommands(script);
    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toContain('https://api.example.com/health');
    expect(cmds[1]).toContain('-X POST');
  });

  test('handles multiline curl commands', () => {
    const script = `curl -X POST \\
  -H "Content-Type: application/json" \\
  -d '{"test":true}' \\
  https://api.example.com`;
    const cmds = extractCurlCommands(script);
    expect(cmds).toHaveLength(1);
    expect(cmds[0]).toContain('-X POST');
    expect(cmds[0]).toContain('https://api.example.com');
  });

  test('skips comments and empty lines', () => {
    const script = `# This is a comment

# Another comment
curl https://example.com
`;
    const cmds = extractCurlCommands(script);
    expect(cmds).toHaveLength(1);
  });

  test('strips piped output', () => {
    const script = `curl https://api.example.com | jq .`;
    const cmds = extractCurlCommands(script);
    expect(cmds).toHaveLength(1);
    expect(cmds[0]).not.toContain('jq');
  });

  test('handles curl after && operator', () => {
    const script = `echo "test" && curl https://example.com`;
    const cmds = extractCurlCommands(script);
    expect(cmds).toHaveLength(1);
    expect(cmds[0]).toContain('https://example.com');
  });
});

describe('convert: parseConvertArgs', () => {
  test('parses curl subcommand', () => {
    const result = parseConvertArgs(['convert', 'curl', 'curl https://example.com']);
    expect(result).not.toBeNull();
    expect(result!.subcommand).toBe('curl');
    expect(result!.input).toBe('curl https://example.com');
  });

  test('parses file subcommand', () => {
    const result = parseConvertArgs(['convert', 'file', 'script.sh']);
    expect(result!.subcommand).toBe('file');
    expect(result!.input).toBe('script.sh');
  });

  test('parses yaml subcommand', () => {
    const result = parseConvertArgs(['convert', 'yaml', 'test.yaml']);
    expect(result!.subcommand).toBe('yaml');
    expect(result!.input).toBe('test.yaml');
  });

  test('parses options', () => {
    const result = parseConvertArgs([
      'convert',
      'curl',
      'input',
      '--output',
      'out.yaml',
      '--debug',
      '--pretty',
    ]);
    expect(result!.options.output).toBe('out.yaml');
    expect(result!.options.debug).toBe(true);
    expect(result!.options.pretty).toBe(true);
  });

  test('returns null for missing args', () => {
    expect(parseConvertArgs(['convert'])).toBeNull();
    expect(parseConvertArgs(['convert', 'curl'])).toBeNull();
  });

  test('returns null for unknown subcommand', () => {
    expect(parseConvertArgs(['convert', 'unknown', 'input'])).toBeNull();
  });
});

describe('convert: convertCurlToYaml integration', () => {
  test('full POST conversion', () => {
    const result = convertCurlToYaml(
      `curl -X POST https://api.example.com/users -H 'Authorization: Bearer TOKEN' -d '{"name":"Alex"}'`,
    );
    expect(result.output).toContain('method: POST');
    expect(result.output).toContain('url: https://api.example.com/users');
    expect(result.output).toContain('auth:');
    expect(result.output).toContain('token: TOKEN');
    expect(result.output).toContain('json:');
    expect(result.output).toContain('name: Alex');
  });

  test('GET with query params', () => {
    const result = convertCurlToYaml('curl "https://api.example.com/search?q=hello&page=1"');
    expect(result.output).toContain('url: https://api.example.com/search');
    expect(result.output).toContain('params:');
    expect(result.output).toContain('q: hello');
  });

  test('debug mode includes AST and IR', () => {
    const result = convertCurlToYaml('curl https://example.com', { debug: true });
    expect(result.debug).toBeDefined();
    expect(result.debug!.tokens).toBeDefined();
    expect(result.debug!.ast).toBeDefined();
    expect(result.debug!.ir).toBeDefined();
  });

  test('complex real-world curl', () => {
    const result = convertCurlToYaml(
      `curl -X POST https://api.stripe.com/v1/charges \
        -u sk_test_key: \
        -d amount=2000 \
        -d currency=usd \
        -d source=tok_visa`,
    );
    expect(result.output).toContain('method: POST');
    expect(result.output).toContain('auth:');
    expect(result.output).toContain('type: basic');
  });
});
