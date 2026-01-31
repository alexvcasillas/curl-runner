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

    test('validates POST request with body', async () => {
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

    test('validates basic auth', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com/users
  auth:
    type: basic
    username: admin
    password: secret
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('validates bearer auth', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com/users
  auth:
    type: bearer
    token: my-token
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

    test('validates expect config', async () => {
      const file = await createTempYaml(`
request:
  url: https://api.example.com/users
  expect:
    status: 200
    headers:
      content-type: application/json
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

    test('validates collection', async () => {
      const file = await createTempYaml(`
collection:
  name: User API
  requests:
    - url: https://api.example.com/users
    - url: https://api.example.com/posts
`);
      const output = await captureOutput(async () => {
        await new ValidateCommand().run([file]);
      });
      expect(output.some((line) => line.includes('All files valid'))).toBe(true);
    });

    test('validates global config', async () => {
      const file = await createTempYaml(`
global:
  execution: parallel
  maxConcurrency: 5
  output:
    format: pretty
    prettyLevel: detailed

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
  });

  describe('invalid YAML files', () => {
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
  });

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
      // Valid file should not show in quiet mode
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
  });

  describe('file discovery', () => {
    test('handles no files found gracefully', async () => {
      const output = await captureOutput(async () => {
        await new ValidateCommand().run(['/tmp/nonexistent-dir-12345/*.yaml']);
      });
      expect(output.some((line) => line.includes('No YAML files found'))).toBe(true);
    });
  });
});
