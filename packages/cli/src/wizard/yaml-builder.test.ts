import { describe, expect, it } from 'bun:test';
import type { WizardAnswers } from './types';
import { buildRequestConfig, buildYamlConfig, toYamlString } from './yaml-builder';

describe('yaml-builder', () => {
  describe('buildRequestConfig', () => {
    it('builds minimal config with just URL', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'GET',
      };

      const config = buildRequestConfig(answers);

      expect(config).toEqual({
        url: 'https://api.example.com',
      });
    });

    it('includes method when not GET', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'POST',
      };

      const config = buildRequestConfig(answers);

      expect(config.method).toBe('POST');
    });

    it('includes name when provided', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'GET',
        name: 'My Request',
      };

      const config = buildRequestConfig(answers);

      expect(config.name).toBe('My Request');
    });

    it('includes headers when provided', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
          Authorization: 'Bearer token',
        },
      };

      const config = buildRequestConfig(answers);

      expect(config.headers).toEqual({
        'Content-Type': 'application/json',
        Authorization: 'Bearer token',
      });
    });

    it('includes JSON body', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'POST',
        bodyType: 'json',
        body: { name: 'test', value: 123 },
      };

      const config = buildRequestConfig(answers);

      expect(config.body).toEqual({ name: 'test', value: 123 });
    });

    it('includes form data', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'POST',
        bodyType: 'form',
        formData: {
          field1: 'value1',
          file: { file: './upload.txt' },
        },
      };

      const config = buildRequestConfig(answers);

      expect(config.formData).toEqual({
        field1: 'value1',
        file: { file: './upload.txt' },
      });
    });

    it('includes raw body', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'POST',
        bodyType: 'raw',
        body: 'plain text content',
      };

      const config = buildRequestConfig(answers);

      expect(config.body).toBe('plain text content');
    });

    it('includes bearer auth', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'GET',
        authType: 'bearer',
        token: 'my-token',
      };

      const config = buildRequestConfig(answers);

      expect(config.auth).toEqual({
        type: 'bearer',
        token: 'my-token',
      });
    });

    it('includes basic auth', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'GET',
        authType: 'basic',
        username: 'user',
        password: 'pass',
      };

      const config = buildRequestConfig(answers);

      expect(config.auth).toEqual({
        type: 'basic',
        username: 'user',
        password: 'pass',
      });
    });

    it('includes timeout', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'GET',
        timeout: 5000,
      };

      const config = buildRequestConfig(answers);

      expect(config.timeout).toBe(5000);
    });

    it('includes followRedirects when false', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'GET',
        followRedirects: false,
      };

      const config = buildRequestConfig(answers);

      expect(config.followRedirects).toBe(false);
    });

    it('includes insecure flag', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'GET',
        insecure: true,
      };

      const config = buildRequestConfig(answers);

      expect(config.insecure).toBe(true);
    });

    it('includes http2 flag', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'GET',
        http2: true,
      };

      const config = buildRequestConfig(answers);

      expect(config.http2).toBe(true);
    });

    it('includes retry config', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'GET',
        retryEnabled: true,
        retryCount: 3,
        retryDelay: 1000,
        retryBackoff: 2,
      };

      const config = buildRequestConfig(answers);

      expect(config.retry).toEqual({
        count: 3,
        delay: 1000,
        backoff: 2,
      });
    });

    it('includes expect validation', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'GET',
        expectStatus: 200,
        expectResponseTime: '< 2000',
      };

      const config = buildRequestConfig(answers);

      expect(config.expect).toEqual({
        status: 200,
        responseTime: '< 2000',
      });
    });

    it('handles array of status codes', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'GET',
        expectStatus: [200, 201],
      };

      const config = buildRequestConfig(answers);

      expect(config.expect?.status).toEqual([200, 201]);
    });
  });

  describe('buildYamlConfig', () => {
    it('wraps config in request key', () => {
      const answers: WizardAnswers = {
        url: 'https://api.example.com',
        method: 'GET',
      };

      const config = buildYamlConfig(answers);

      expect(config.request).toBeDefined();
      expect(config.request.url).toBe('https://api.example.com');
    });
  });

  describe('toYamlString', () => {
    it('serializes simple config to YAML', () => {
      const config = buildYamlConfig({
        url: 'https://api.example.com',
        method: 'GET',
      });

      const yaml = toYamlString(config);

      expect(yaml).toContain('request:');
      // URL contains : so it gets quoted
      expect(yaml).toContain('url: "https://api.example.com"');
    });

    it('serializes POST with body', () => {
      const config = buildYamlConfig({
        url: 'https://api.example.com',
        method: 'POST',
        bodyType: 'json',
        body: { name: 'test' },
      });

      const yaml = toYamlString(config);

      expect(yaml).toContain('method: POST');
      expect(yaml).toContain('body:');
      expect(yaml).toContain('name: test');
    });

    it('quotes strings with special characters', () => {
      const config = buildYamlConfig({
        url: 'https://api.example.com',
        method: 'POST',
        bodyType: 'json',
        body: { message: 'hello: world' },
      });

      const yaml = toYamlString(config);

      expect(yaml).toContain('"hello: world"');
    });

    it('serializes arrays inline for simple values', () => {
      const config = buildYamlConfig({
        url: 'https://api.example.com',
        method: 'GET',
        expectStatus: [200, 201],
      });

      const yaml = toYamlString(config);

      expect(yaml).toContain('[200, 201]');
    });
  });
});
