/**
 * Builds YAML configuration from wizard answers.
 */

import type { JsonValue, RequestConfig } from '../types/config';
import type { WizardAnswers } from './types';

export interface YamlOutput {
  request: RequestConfig;
}

/**
 * Converts wizard answers to a RequestConfig object.
 */
export function buildRequestConfig(answers: WizardAnswers): RequestConfig {
  const config: RequestConfig = {
    url: answers.url,
  };

  // Name
  if (answers.name) {
    config.name = answers.name;
  }

  // Method (only include if not GET)
  if (answers.method && answers.method !== 'GET') {
    config.method = answers.method;
  }

  // Headers
  if (answers.headers && Object.keys(answers.headers).length > 0) {
    config.headers = answers.headers;
  }

  // Query params
  if (answers.params && Object.keys(answers.params).length > 0) {
    config.params = answers.params;
  }

  // Body
  if (answers.bodyType === 'json' && answers.body) {
    config.body = answers.body as JsonValue;
  } else if (answers.bodyType === 'form' && answers.formData) {
    config.formData = answers.formData;
  } else if (answers.bodyType === 'raw' && answers.body) {
    config.body = answers.body as string;
  }

  // Auth
  if (answers.authType === 'basic' && answers.username) {
    config.auth = {
      type: 'basic',
      username: answers.username,
      password: answers.password || '',
    };
  } else if (answers.authType === 'bearer' && answers.token) {
    config.auth = {
      type: 'bearer',
      token: answers.token,
    };
  }

  // Timeout
  if (answers.timeout) {
    config.timeout = answers.timeout;
  }

  // Redirects
  if (answers.followRedirects === false) {
    config.followRedirects = false;
  }
  if (answers.maxRedirects) {
    config.maxRedirects = answers.maxRedirects;
  }

  // SSL
  if (answers.insecure) {
    config.insecure = true;
  }

  // HTTP/2
  if (answers.http2) {
    config.http2 = true;
  }

  // Retry
  if (answers.retryEnabled && answers.retryCount) {
    config.retry = {
      count: answers.retryCount,
    };
    if (answers.retryDelay) {
      config.retry.delay = answers.retryDelay;
    }
    if (answers.retryBackoff) {
      config.retry.backoff = answers.retryBackoff;
    }
  }

  // Validation (expect)
  if (answers.expectStatus || answers.expectResponseTime) {
    config.expect = {};
    if (answers.expectStatus) {
      config.expect.status = answers.expectStatus;
    }
    if (answers.expectResponseTime) {
      config.expect.responseTime = answers.expectResponseTime;
    }
  }

  // Output file
  if (answers.outputFile) {
    config.output = answers.outputFile;
  }

  return config;
}

/**
 * Converts wizard answers to full YAML structure.
 */
export function buildYamlConfig(answers: WizardAnswers): YamlOutput {
  return {
    request: buildRequestConfig(answers),
  };
}

/**
 * Serializes config to YAML string.
 */
export function toYamlString(config: YamlOutput): string {
  return serializeToYaml(config);
}

/**
 * Custom YAML serializer for clean output.
 * Uses Bun's YAML stringify but with formatting.
 */
function serializeToYaml(obj: unknown, indent = 0): string {
  const spaces = '  '.repeat(indent);

  if (obj === null || obj === undefined) {
    return 'null';
  }

  if (typeof obj === 'string') {
    // Check if string needs quoting
    if (
      obj.includes(':') ||
      obj.includes('#') ||
      obj.includes('\n') ||
      obj.startsWith(' ') ||
      obj.endsWith(' ') ||
      /^[0-9]/.test(obj) ||
      ['true', 'false', 'null', 'yes', 'no'].includes(obj.toLowerCase())
    ) {
      return JSON.stringify(obj);
    }
    return obj;
  }

  if (typeof obj === 'number' || typeof obj === 'boolean') {
    return String(obj);
  }

  if (Array.isArray(obj)) {
    if (obj.length === 0) {
      return '[]';
    }

    // Check if simple array (all primitives)
    if (obj.every((item) => typeof item !== 'object' || item === null)) {
      return `[${obj.map((item) => serializeToYaml(item, 0)).join(', ')}]`;
    }

    return obj
      .map((item) => `${spaces}- ${serializeToYaml(item, indent + 1).trimStart()}`)
      .join('\n');
  }

  if (typeof obj === 'object') {
    const entries = Object.entries(obj as Record<string, unknown>);
    if (entries.length === 0) {
      return '{}';
    }

    const lines: string[] = [];
    for (const [key, value] of entries) {
      if (value === undefined) {
        continue;
      }

      if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
        lines.push(`${spaces}${key}:`);
        const nested = serializeToYaml(value, indent + 1);
        lines.push(nested);
      } else {
        const serialized = serializeToYaml(value, indent + 1);
        if (serialized.includes('\n')) {
          lines.push(`${spaces}${key}:`);
          lines.push(serialized);
        } else {
          lines.push(`${spaces}${key}: ${serialized}`);
        }
      }
    }

    return lines.join('\n');
  }

  return String(obj);
}
