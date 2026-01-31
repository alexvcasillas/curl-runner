import { describe, expect, test } from 'bun:test';
import { ValidateCommand } from './validate';

// Helper to create temp YAML file
async function createTempYaml(content: string): Promise<string> {
  const path = `/tmp/curl-runner-test-${Date.now()}-${Math.random().toString(36).slice(2)}.yaml`;
  await Bun.write(path, content);
  return path;
}

// Helper to capture console output during test
async function captureOutput(fn: () => Promise<void>): Promise<string[]> {
  const output: string[] = [];
  const originalLog = console.log;
  console.log = (...args: unknown[]) => {
    output.push(args.map(String).join(' '));
  };
  try {
    await fn();
  } catch {
    // Ignore errors (e.g. process.exit)
  } finally {
    console.log = originalLog;
  }
  return output;
}

describe('ValidateCommand', () => {
  // ===========================
  // VALID YAML FILES
  // ===========================
  describe('valid YAML files', () => {
    test('validates simple GET request', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com/users
  method: GET
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('validates POST request with body and Content-Type', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com/users
  method: POST
  headers:
    Content-Type: application/json
  body:
    name: "John Doe"
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('validates basic auth with env variables', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com/users
  auth:
    type: basic
    username: \${env.USER}
    password: \${env.PASS}
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('validates bearer auth with env token', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com/users
  auth:
    type: bearer
    token: \${env.TOKEN}
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('validates retry config', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com/users
  method: GET
  retry:
    count: 3
    delay: 1000
    backoff: 2
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('validates expect config with status array', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com/users
  expect:
    status: [200, 201, 204]
    headers:
      Content-Type: application/json
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('validates response time with variable', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com/users
  expect:
    responseTime: "< \${TIMEOUT}"
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('validates collection with defaults', async () => {
      const file = await createTempYaml(`
collection:
  name: User API
  defaults:
    headers:
      Authorization: Bearer token
  requests:
    - url: https://api.example.com/users
    - url: https://api.example.com/posts
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('validates global config with all options', async () => {
      const file = await createTempYaml(`
global:
  execution: parallel
  maxConcurrency: 5
  continueOnError: true
  http2: true
  output:
    format: pretty
    prettyLevel: detailed
    verbose: true

requests:
  - url: https://api.example.com/users
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('validates all HTTP methods', async () => {
      const methods = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'];
      for (const method of methods) {
        const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: ${method}
`);
        const output = await captureOutput(async () => {
          await new ValidateCommand().run([file]);
        });
        expect(output.some((line) => line.includes('All files valid'))).toBe(true);
      }
    });

    test('validates store config', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com/users
  store:
    userId: body.id
    statusCode: status
    contentType: headers.content-type
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('validates when condition with store', async () => {
      const file = await createTempYaml(`
requests:
  - url: https://api.example.com/login
    store:
      token: body.token
  - url: https://api.example.com/profile
    when:
      left: store.token
      operator: exists
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('validates SSL config', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  ssl:
    verify: true
    ca: ./certs/ca.pem
    cert: ./certs/client.pem
    key: ./certs/client-key.pem
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });
  });

  // ===========================
  // URL VALIDATIONS
  // ===========================
  describe('URL validations', () => {
    test('reports missing URL', async () => {
      const file = await createTempYaml(`
request:
  method: GET
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('URL is required'))).toBe(true);
    });

    test('reports missing https:// as warning', async () => {
      const file = await createTempYaml(`
request:
  url: api.example.com/users
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('http://'))).toBe(true);
    });

    test('detects htpp typo', async () => {
      const file = await createTempYaml(`
request:
  url: htpp://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('typo') && line.includes('htpp'))).toBe(true);
    });

    test('detects htps typo', async () => {
      const file = await createTempYaml(`
request:
  url: htps://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('typo') && line.includes('htps'))).toBe(true);
    });

    test('detects localhost typo', async () => {
      const file = await createTempYaml(`
request:
  url: http://localhsot:3000
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('typo') && line.includes('localhsot'))).toBe(true);
    });

    test('detects double slashes in URL path', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com//users
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('double slashes'))).toBe(true);
    });

    test('detects spaces in URL', async () => {
      const file = await createTempYaml(`
request:
  url: "https://api.example.com/users/john doe"
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('spaces'))).toBe(true);
    });

    test('detects invalid port number', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com:99999
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Invalid port'))).toBe(true);
    });

    test('shows HTTP security info', async () => {
      const file = await createTempYaml(`
request:
  url: http://api.example.com/users
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('HTTP') && line.includes('HTTPS'))).toBe(true);
    });

    test('skips URL validation for variables', async () => {
      const file = await createTempYaml(`
request:
  url: \${BASE_URL}/users
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });
  });

  // ===========================
  // METHOD VALIDATIONS
  // ===========================
  describe('method validations', () => {
    test('reports invalid HTTP method', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: INVALID
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Invalid method'))).toBe(true);
    });

    test('reports lowercase method as warning', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: get
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('uppercase'))).toBe(true);
    });

    test('suggests closest method match', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: DELET
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('DELETE'))).toBe(true);
    });
  });

  // ===========================
  // HEADER VALIDATIONS
  // ===========================
  describe('header validations', () => {
    test('suggests proper header casing', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: POST
  headers:
    content-type: application/json
  body: {}
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Content-Type'))).toBe(true);
    });

    test('detects duplicate headers', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  headers:
    Content-Type: application/json
    content-type: text/plain
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Duplicate'))).toBe(true);
    });

    test('warns about body without Content-Type', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: POST
  body:
    test: value
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Content-Type'))).toBe(true);
    });

    test('warns about auth block and Authorization header conflict', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  headers:
    Authorization: Bearer token123
  auth:
    type: bearer
    token: different-token
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(
        output.some((line) => line.includes('auth block') && line.includes('Authorization')),
      ).toBe(true);
    });
  });

  // ===========================
  // BODY VALIDATIONS
  // ===========================
  describe('body validations', () => {
    test('warns about body with GET method', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: GET
  body:
    test: value
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('GET') && line.includes('ignored'))).toBe(true);
    });

    test('warns about invalid JSON string body', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: POST
  headers:
    Content-Type: application/json
  body: "{invalid json"
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('JSON') && line.includes('invalid'))).toBe(true);
    });

    test('reports body and formData conflict', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: POST
  body:
    test: value
  formData:
    field: value
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Cannot use both'))).toBe(true);
    });
  });

  // ===========================
  // AUTH VALIDATIONS
  // ===========================
  describe('auth validations', () => {
    test('reports invalid auth type', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  auth:
    type: oauth
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Invalid auth type'))).toBe(true);
    });

    test('reports missing username for basic auth', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  auth:
    type: basic
    password: secret
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Username required'))).toBe(true);
    });

    test('reports missing token for bearer auth', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  auth:
    type: bearer
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Token required'))).toBe(true);
    });

    test('warns about hardcoded password', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  auth:
    type: basic
    username: admin
    password: supersecret123
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Hardcoded password'))).toBe(true);
    });

    test('warns about hardcoded token', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  auth:
    type: bearer
    token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Hardcoded token'))).toBe(true);
    });

    test('suggests closest auth type match', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  auth:
    type: basci
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('basic'))).toBe(true);
    });
  });

  // ===========================
  // SECURITY VALIDATIONS
  // ===========================
  describe('security validations', () => {
    test('warns about insecure mode', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  insecure: true
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Insecure') && line.includes('SSL'))).toBe(true);
    });

    test('warns about sensitive variable values', async () => {
      const file = await createTempYaml(`
global:
  variables:
    api_key: sk_live_abcdef123456
requests:
  - url: https://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('sensitive'))).toBe(true);
    });

    test('warns about sensitive header values', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  headers:
    X-API-Key: sk_live_supersecret
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('sensitive'))).toBe(true);
    });
  });

  // ===========================
  // UNKNOWN KEY VALIDATIONS
  // ===========================
  describe('unknown key validations', () => {
    test('detects unknown top-level key', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
unknown_key: value
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Unknown') && line.includes('unknown_key'))).toBe(
        true,
      );
    });

    test('detects unknown request key with suggestion', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  metod: GET
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('metod') && line.includes('method'))).toBe(true);
    });

    test('detects unknown global key', async () => {
      const file = await createTempYaml(`
global:
  excecution: parallel
requests:
  - url: https://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('excecution'))).toBe(true);
    });
  });

  // ===========================
  // TIMEOUT/RETRY VALIDATIONS
  // ===========================
  describe('timeout and retry validations', () => {
    test('reports negative timeout', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  timeout: -1000
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('positive'))).toBe(true);
    });

    test('warns about very short timeout', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  timeout: 50
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('short timeout'))).toBe(true);
    });

    test('warns about very long timeout', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  timeout: 600000
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('long timeout'))).toBe(true);
    });

    test('reports missing retry count', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  retry:
    delay: 1000
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('count') && line.includes('required'))).toBe(true);
    });

    test('warns about high retry count', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  retry:
    count: 15
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('High retry count'))).toBe(true);
    });

    test('warns about high backoff', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  retry:
    count: 3
    backoff: 10
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('backoff') && line.includes('delay'))).toBe(true);
    });

    test('warns about POST retry', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: POST
  retry:
    count: 3
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('POST') && line.includes('duplicate'))).toBe(true);
    });

    test('reports string timeout with auto-fix suggestion', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  timeout: "5000"
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('number') && line.includes('string'))).toBe(true);
    });
  });

  // ===========================
  // EXPECT VALIDATIONS
  // ===========================
  describe('expect validations', () => {
    test('reports invalid status code', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  expect:
    status: 1000
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('100-599'))).toBe(true);
    });

    test('reports status below 100', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  expect:
    status: 50
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('100-599'))).toBe(true);
    });

    test('reports string status with fix suggestion', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  expect:
    status: "200"
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('number') && line.includes('string'))).toBe(true);
    });

    test('warns about empty status array', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  expect:
    status: []
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Empty'))).toBe(true);
    });

    test('warns about invalid response time format', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  expect:
    responseTime: "fast"
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('responseTime'))).toBe(true);
    });
  });

  // ===========================
  // WHEN CONDITION VALIDATIONS
  // ===========================
  describe('when condition validations', () => {
    test('reports missing left operand', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  when:
    operator: "=="
    right: 200
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Left operand'))).toBe(true);
    });

    test('reports missing operator', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  when:
    left: store.status
    right: 200
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Operator') && line.includes('required'))).toBe(
        true,
      );
    });

    test('reports invalid operator', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  when:
    left: store.status
    operator: "==="
    right: 200
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Invalid operator'))).toBe(true);
    });

    test('reports missing right operand for comparison', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  when:
    left: store.status
    operator: "=="
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Right operand'))).toBe(true);
    });

    test('warns about when string without store reference', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  when: "someVar == 200"
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('store.'))).toBe(true);
    });

    test('warns about empty all array', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  when:
    all: []
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Empty') && line.includes('all'))).toBe(true);
    });
  });

  // ===========================
  // STORE VALIDATIONS
  // ===========================
  describe('store validations', () => {
    test('warns about invalid store path', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  store:
    value: invalid.path
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('body') || line.includes('status'))).toBe(true);
    });

    test('warns about invalid variable name', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  store:
    "invalid-name": body.id
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('alphanumeric'))).toBe(true);
    });
  });

  // ===========================
  // GLOBAL CONFIG VALIDATIONS
  // ===========================
  describe('global config validations', () => {
    test('reports invalid execution mode', async () => {
      const file = await createTempYaml(`
global:
  execution: invalid
requests:
  - url: https://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Invalid execution mode'))).toBe(true);
    });

    test('warns about maxConcurrency with sequential', async () => {
      const file = await createTempYaml(`
global:
  execution: sequential
  maxConcurrency: 5
requests:
  - url: https://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(
        output.some((line) => line.includes('maxConcurrency') && line.includes('sequential')),
      ).toBe(true);
    });

    test('reports missing request definition', async () => {
      const file = await createTempYaml(`
global:
  execution: parallel
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Missing request'))).toBe(true);
    });

    test('reports invalid output format', async () => {
      const file = await createTempYaml(`
global:
  output:
    format: xml
requests:
  - url: https://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Invalid format'))).toBe(true);
    });

    test('reports boolean string value', async () => {
      const file = await createTempYaml(`
global:
  continueOnError: "true"
requests:
  - url: https://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('boolean') && line.includes('string'))).toBe(true);
    });
  });

  // ===========================
  // SSL CONFIG VALIDATIONS
  // ===========================
  describe('SSL config validations', () => {
    test('warns about cert without key', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  ssl:
    cert: ./certs/client.pem
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('cert') && line.includes('key'))).toBe(true);
    });

    test('warns about key without cert', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  ssl:
    key: ./certs/client-key.pem
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('key') && line.includes('cert'))).toBe(true);
    });
  });

  // ===========================
  // DUPLICATE REQUEST NAMES
  // ===========================
  describe('duplicate request names', () => {
    test('warns about duplicate request names', async () => {
      const file = await createTempYaml(`
requests:
  - name: get-users
    url: https://api.example.com/users
  - name: get-users
    url: https://api.example.com/users/2
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Duplicate') && line.includes('get-users'))).toBe(
        true,
      );
    });
  });

  // ===========================
  // AUTO-FIX FUNCTIONALITY
  // ===========================
  describe('auto-fix functionality', () => {
    test('fixes lowercase method to uppercase', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: get
`);
      await captureOutput(async () => {
        await new ValidateCommand().run([file, '--fix']);
      });
      const content = await Bun.file(file).text();
      expect(content).toContain('GET');
    });

    test('fixes missing https:// prefix', async () => {
      const file = await createTempYaml(`
request:
  url: api.example.com
  method: GET
`);
      await captureOutput(async () => {
        await new ValidateCommand().run([file, '--fix']);
      });
      const content = await Bun.file(file).text();
      expect(content).toContain('https://api.example.com');
    });

    test('fixes URL typo htpp', async () => {
      const file = await createTempYaml(`
request:
  url: htpp://api.example.com
  method: GET
`);
      await captureOutput(async () => {
        await new ValidateCommand().run([file, '--fix']);
      });
      const content = await Bun.file(file).text();
      expect(content).toContain('http://api.example.com');
    });

    test('fixes spaces in URL', async () => {
      const file = await createTempYaml(`
request:
  url: "https://api.example.com/users/john doe"
  method: GET
`);
      await captureOutput(async () => {
        await new ValidateCommand().run([file, '--fix']);
      });
      const content = await Bun.file(file).text();
      expect(content).toContain('%20');
    });

    test('reports fixed count', async () => {
      const file = await createTempYaml(`
request:
  url: api.example.com
  method: get
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file, '--fix']);
      });
      expect(output.some((line) => line.includes('Fixed'))).toBe(true);
    });
  });

  // ===========================
  // CLI OPTIONS
  // ===========================
  describe('CLI options', () => {
    test('--quiet hides valid files', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: GET
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file, '--quiet']);
      });
      expect(output.some((line) => line.includes(file))).toBe(false);
    });

    test('-q short flag works', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: GET
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file, '-q']);
      });
      expect(output.some((line) => line.includes(file))).toBe(false);
    });

    test('-f short flag for fix works', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: get
`);
      await captureOutput(async () => {
        await new ValidateCommand().run([file, '-f']);
      });
      const content = await Bun.file(file).text();
      expect(content).toContain('GET');
    });

    test('--strict treats warnings as errors', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: get
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file, '--strict']);
      });
      expect(output.some((line) => line.includes('file(s) with'))).toBe(true);
    });

    test('-s short flag for strict works', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  method: get
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file, '-s']);
      });
      expect(output.some((line) => line.includes('file(s) with'))).toBe(true);
    });
  });

  // ===========================
  // FILE DISCOVERY
  // ===========================
  describe('file discovery', () => {
    test('handles no files found gracefully', async () => {
      const output = await captureOutput(async () => {
        await new ValidateCommand().run(['/tmp/nonexistent-dir-12345/*.yaml']);
      });
      expect(output.some((line) => line.includes('No YAML files found'))).toBe(true);
    });
  });

  // ===========================
  // PROXY VALIDATIONS
  // ===========================
  describe('proxy validations', () => {
    test('reports invalid proxy URL', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  proxy: invalid-proxy
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('Proxy'))).toBe(true);
    });

    test('accepts valid proxy URL', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  proxy: http://proxy.example.com:8080
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('accepts socks5 proxy', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com
  proxy: socks5://proxy.example.com:1080
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });
  });

  // ===========================
  // COLLECTION VALIDATIONS
  // ===========================
  describe('collection validations', () => {
    test('warns about missing collection name', async () => {
      const file = await createTempYaml(`
collection:
  requests:
    - url: https://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('name') && line.includes('recommended'))).toBe(
        true,
      );
    });

    test('reports missing collection requests', async () => {
      const file = await createTempYaml(`
collection:
  name: Test Collection
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('requests'))).toBe(true);
    });

    test('warns about empty collection requests', async () => {
      const file = await createTempYaml(`
collection:
  name: Test Collection
  requests: []
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('no requests'))).toBe(true);
    });
  });

  // ===========================
  // CI CONFIG VALIDATIONS
  // ===========================
  describe('CI config validations', () => {
    test('reports failOnPercentage out of range', async () => {
      const file = await createTempYaml(`
global:
  ci:
    failOnPercentage: 150
requests:
  - url: https://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('0 and 100'))).toBe(true);
    });

    test('reports negative failOn', async () => {
      const file = await createTempYaml(`
global:
  ci:
    failOn: -1
requests:
  - url: https://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('non-negative'))).toBe(true);
    });
  });

  // ===========================
  // PROFILE CONFIG VALIDATIONS
  // ===========================
  describe('profile config validations', () => {
    test('reports invalid iterations', async () => {
      const file = await createTempYaml(`
global:
  profile:
    iterations: 0
requests:
  - url: https://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('positive'))).toBe(true);
    });

    test('warns about very high iterations', async () => {
      const file = await createTempYaml(`
global:
  profile:
    iterations: 50000
requests:
  - url: https://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('high') && line.includes('iteration'))).toBe(true);
    });
  });

  // ===========================
  // CONNECTION POOL VALIDATIONS
  // ===========================
  describe('connection pool validations', () => {
    test('warns about very high maxStreamsPerHost', async () => {
      const file = await createTempYaml(`
global:
  connectionPool:
    enabled: true
    maxStreamsPerHost: 200
requests:
  - url: https://api.example.com
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('stream') && line.includes('issues'))).toBe(true);
    });
  });
});
