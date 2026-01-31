import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { mkdir, rm } from 'node:fs/promises';
import { join } from 'node:path';
import type { RequestConfig } from '../types/config';
import { fileExists, loadYamlFile, requestToAnswers } from './parser';

describe('parser', () => {
  const testDir = join(import.meta.dir, '__test_fixtures__');

  beforeAll(async () => {
    await mkdir(testDir, { recursive: true });
  });

  afterAll(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('requestToAnswers', () => {
    it('converts minimal request', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
      };

      const answers = requestToAnswers(request);

      expect(answers.url).toBe('https://api.example.com');
      expect(answers.method).toBe('GET');
    });

    it('converts request with method', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
        method: 'POST',
      };

      const answers = requestToAnswers(request);

      expect(answers.method).toBe('POST');
    });

    it('converts request with name', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
        name: 'My Request',
      };

      const answers = requestToAnswers(request);

      expect(answers.name).toBe('My Request');
    });

    it('converts request with headers', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
        headers: {
          'Content-Type': 'application/json',
        },
      };

      const answers = requestToAnswers(request);

      expect(answers.headers).toEqual({
        'Content-Type': 'application/json',
      });
    });

    it('converts request with JSON body', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
        body: { key: 'value' },
      };

      const answers = requestToAnswers(request);

      expect(answers.bodyType).toBe('json');
      expect(answers.body).toEqual({ key: 'value' });
    });

    it('converts request with string body', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
        body: 'plain text',
      };

      const answers = requestToAnswers(request);

      expect(answers.bodyType).toBe('raw');
      expect(answers.body).toBe('plain text');
    });

    it('converts request with formData', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
        formData: {
          field: 'value',
        },
      };

      const answers = requestToAnswers(request);

      expect(answers.bodyType).toBe('form');
      expect(answers.formData).toEqual({ field: 'value' });
    });

    it('converts request with bearer auth', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
        auth: {
          type: 'bearer',
          token: 'my-token',
        },
      };

      const answers = requestToAnswers(request);

      expect(answers.authType).toBe('bearer');
      expect(answers.token).toBe('my-token');
    });

    it('converts request with basic auth', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
        auth: {
          type: 'basic',
          username: 'user',
          password: 'pass',
        },
      };

      const answers = requestToAnswers(request);

      expect(answers.authType).toBe('basic');
      expect(answers.username).toBe('user');
      expect(answers.password).toBe('pass');
    });

    it('converts request with timeout', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
        timeout: 5000,
      };

      const answers = requestToAnswers(request);

      expect(answers.timeout).toBe(5000);
    });

    it('converts request with retry config', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
        retry: {
          count: 3,
          delay: 1000,
          backoff: 2,
        },
      };

      const answers = requestToAnswers(request);

      expect(answers.retryEnabled).toBe(true);
      expect(answers.retryCount).toBe(3);
      expect(answers.retryDelay).toBe(1000);
      expect(answers.retryBackoff).toBe(2);
    });

    it('converts request with expect validation', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
        expect: {
          status: 200,
          responseTime: '< 2000',
        },
      };

      const answers = requestToAnswers(request);

      expect(answers.expectStatus).toBe(200);
      expect(answers.expectResponseTime).toBe('< 2000');
    });

    it('converts request with insecure flag', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
        insecure: true,
      };

      const answers = requestToAnswers(request);

      expect(answers.insecure).toBe(true);
    });

    it('converts request with http2 flag', () => {
      const request: RequestConfig = {
        url: 'https://api.example.com',
        http2: true,
      };

      const answers = requestToAnswers(request);

      expect(answers.http2).toBe(true);
    });
  });

  describe('loadYamlFile', () => {
    it('loads single request YAML', async () => {
      const filePath = join(testDir, 'single-request.yaml');
      await Bun.write(
        filePath,
        `
request:
  url: https://api.example.com
  method: POST
  headers:
    Content-Type: application/json
  body:
    name: test
`,
      );

      const answers = await loadYamlFile(filePath);

      expect(answers.url).toBe('https://api.example.com');
      expect(answers.method).toBe('POST');
      expect(answers.headers?.['Content-Type']).toBe('application/json');
    });

    it('loads first request from requests array', async () => {
      const filePath = join(testDir, 'multi-request.yaml');
      await Bun.write(
        filePath,
        `
requests:
  - url: https://api.example.com/first
    method: GET
  - url: https://api.example.com/second
    method: POST
`,
      );

      const answers = await loadYamlFile(filePath);

      expect(answers.url).toBe('https://api.example.com/first');
      expect(answers.method).toBe('GET');
    });

    it('loads first request from collection', async () => {
      const filePath = join(testDir, 'collection.yaml');
      await Bun.write(
        filePath,
        `
collection:
  name: Test Collection
  requests:
    - url: https://api.example.com/collection
      method: PUT
`,
      );

      const answers = await loadYamlFile(filePath);

      expect(answers.url).toBe('https://api.example.com/collection');
      expect(answers.method).toBe('PUT');
    });

    it('throws error for file with no requests', async () => {
      const filePath = join(testDir, 'empty.yaml');
      await Bun.write(
        filePath,
        `
global:
  variables:
    BASE_URL: https://api.example.com
`,
      );

      expect(loadYamlFile(filePath)).rejects.toThrow('No request found');
    });
  });

  describe('fileExists', () => {
    it('returns true for existing file', async () => {
      const filePath = join(testDir, 'exists.yaml');
      await Bun.write(filePath, 'test');

      const exists = await fileExists(filePath);

      expect(exists).toBe(true);
    });

    it('returns false for non-existing file', async () => {
      const exists = await fileExists(join(testDir, 'does-not-exist.yaml'));

      expect(exists).toBe(false);
    });
  });
});
