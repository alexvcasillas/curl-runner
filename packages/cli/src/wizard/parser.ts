/**
 * Parser for loading existing YAML files into wizard answers (edit mode).
 */

import { YamlParser } from '../parser/yaml';
import type { RequestConfig } from '../types/config';
import type { WizardAnswers } from './types';

/**
 * Loads a YAML file and converts it to WizardAnswers for editing.
 */
export async function loadYamlFile(filePath: string): Promise<WizardAnswers> {
  const content = await YamlParser.parseFile(filePath);

  // Find the request config
  let request: RequestConfig | undefined;

  if (content.request) {
    request = content.request;
  } else if (content.requests && content.requests.length > 0) {
    request = content.requests[0];
  } else if (content.collection?.requests && content.collection.requests.length > 0) {
    request = content.collection.requests[0];
  }

  if (!request) {
    throw new Error('No request found in YAML file');
  }

  return requestToAnswers(request);
}

/**
 * Converts a RequestConfig to WizardAnswers.
 */
export function requestToAnswers(request: RequestConfig): WizardAnswers {
  const answers: WizardAnswers = {
    url: request.url,
    method: request.method || 'GET',
  };

  // Name
  if (request.name) {
    answers.name = request.name;
  }

  // Headers
  if (request.headers && Object.keys(request.headers).length > 0) {
    answers.headers = request.headers;
  }

  // Query params
  if (request.params && Object.keys(request.params).length > 0) {
    answers.params = request.params;
  }

  // Body
  if (request.body !== undefined) {
    if (typeof request.body === 'object' && request.body !== null) {
      answers.bodyType = 'json';
      answers.body = request.body;
    } else {
      answers.bodyType = 'raw';
      answers.body = request.body;
    }
  } else if (request.formData) {
    answers.bodyType = 'form';
    answers.formData = request.formData as Record<string, string | { file: string }>;
  } else {
    answers.bodyType = 'none';
  }

  // Auth
  if (request.auth) {
    if (request.auth.type === 'bearer') {
      answers.authType = 'bearer';
      answers.token = request.auth.token;
    } else if (request.auth.type === 'basic') {
      answers.authType = 'basic';
      answers.username = request.auth.username;
      answers.password = request.auth.password;
    }
  } else {
    answers.authType = 'none';
  }

  // Advanced
  if (request.timeout) {
    answers.timeout = request.timeout;
  }
  if (request.followRedirects !== undefined) {
    answers.followRedirects = request.followRedirects;
  }
  if (request.maxRedirects) {
    answers.maxRedirects = request.maxRedirects;
  }
  if (request.insecure) {
    answers.insecure = request.insecure;
  }
  if (request.http2) {
    answers.http2 = request.http2;
  }

  // Retry
  if (request.retry) {
    answers.retryEnabled = true;
    answers.retryCount = request.retry.count;
    answers.retryDelay = request.retry.delay;
    answers.retryBackoff = request.retry.backoff;
  }

  // Validation
  if (request.expect) {
    if (request.expect.status) {
      answers.expectStatus = request.expect.status;
    }
    if (request.expect.responseTime) {
      answers.expectResponseTime = request.expect.responseTime;
    }
  }

  // Output
  if (request.output) {
    answers.outputFile = request.output;
  }

  return answers;
}

/**
 * Checks if a file exists.
 */
export async function fileExists(filePath: string): Promise<boolean> {
  try {
    const file = Bun.file(filePath);
    return await file.exists();
  } catch {
    return false;
  }
}
