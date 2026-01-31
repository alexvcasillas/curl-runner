/**
 * Interactive prompts for the wizard using @clack/prompts.
 */

import * as p from '@clack/prompts';
import { getTemplateChoices } from './templates';
import type { WizardAnswers, WizardTemplate } from './types';

const HTTP_METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'] as const;

/**
 * Prompts for template selection.
 */
export async function promptTemplate(): Promise<WizardTemplate | 'blank' | symbol> {
  const choices = [
    { value: 'blank' as const, label: 'Blank', hint: 'Start from scratch' },
    ...getTemplateChoices(),
  ];

  return p.select({
    message: 'Start from a template?',
    options: choices,
  });
}

/**
 * Prompts for request basics (URL, method, name).
 */
export async function promptBasics(
  defaults?: Partial<WizardAnswers>,
): Promise<Pick<WizardAnswers, 'name' | 'url' | 'method'> | symbol> {
  const result = await p.group(
    {
      url: () =>
        p.text({
          message: 'Request URL',
          placeholder: 'https://api.example.com/endpoint',
          initialValue: defaults?.url || '',
          validate: (value) => {
            if (!value) {
              return 'URL is required';
            }
            try {
              new URL(value);
            } catch {
              return 'Invalid URL format';
            }
          },
        }),
      method: () =>
        p.select({
          message: 'HTTP method',
          initialValue: defaults?.method || 'GET',
          options: HTTP_METHODS.map((m) => ({ value: m, label: m })),
        }),
      name: () =>
        p.text({
          message: 'Request name (optional)',
          placeholder: 'My API Request',
          initialValue: defaults?.name || '',
        }),
    },
    {
      onCancel: () => {
        p.cancel('Wizard cancelled');
        process.exit(0);
      },
    },
  );

  return result as Pick<WizardAnswers, 'name' | 'url' | 'method'>;
}

/**
 * Prompts for headers configuration.
 */
export async function promptHeaders(
  defaults?: Record<string, string>,
): Promise<Record<string, string> | undefined | symbol> {
  const addHeaders = await p.confirm({
    message: 'Add headers?',
    initialValue: defaults ? Object.keys(defaults).length > 0 : false,
  });

  if (p.isCancel(addHeaders) || !addHeaders) {
    return p.isCancel(addHeaders) ? addHeaders : defaults;
  }

  const headers: Record<string, string> = { ...defaults };
  let addMore = true;

  // Common header presets
  const presets = await p.multiselect({
    message: 'Add common headers (space to select)',
    options: [
      { value: 'content-type-json', label: 'Content-Type: application/json' },
      { value: 'accept-json', label: 'Accept: application/json' },
      { value: 'user-agent', label: 'User-Agent: curl-runner' },
    ],
    required: false,
  });

  if (p.isCancel(presets)) {
    return presets;
  }

  if (Array.isArray(presets)) {
    for (const preset of presets) {
      if (preset === 'content-type-json') {
        headers['Content-Type'] = 'application/json';
      }
      if (preset === 'accept-json') {
        headers.Accept = 'application/json';
      }
      if (preset === 'user-agent') {
        headers['User-Agent'] = 'curl-runner';
      }
    }
  }

  // Custom headers
  while (addMore) {
    const addCustom = await p.confirm({
      message: 'Add custom header?',
      initialValue: false,
    });

    if (p.isCancel(addCustom)) {
      return addCustom;
    }
    if (!addCustom) {
      break;
    }

    const header = await p.group({
      key: () =>
        p.text({
          message: 'Header name',
          placeholder: 'X-Custom-Header',
          validate: (v) => (!v ? 'Header name required' : undefined),
        }),
      value: () =>
        p.text({
          message: 'Header value',
          placeholder: 'value',
          validate: (v) => (!v ? 'Header value required' : undefined),
        }),
    });

    if (header.key && header.value) {
      headers[header.key] = header.value;
    }

    addMore = true;
  }

  return Object.keys(headers).length > 0 ? headers : undefined;
}

/**
 * Prompts for request body configuration.
 */
export async function promptBody(
  defaults?: Partial<WizardAnswers>,
): Promise<Pick<WizardAnswers, 'bodyType' | 'body' | 'formData'> | symbol> {
  const bodyType = await p.select({
    message: 'Request body type',
    initialValue: defaults?.bodyType || 'none',
    options: [
      { value: 'none', label: 'None', hint: 'No request body' },
      { value: 'json', label: 'JSON', hint: 'JSON object' },
      { value: 'form', label: 'Form Data', hint: 'Multipart form / file upload' },
      { value: 'raw', label: 'Raw', hint: 'Plain text or other' },
    ],
  });

  if (p.isCancel(bodyType)) {
    return bodyType;
  }

  if (bodyType === 'none') {
    return { bodyType: 'none' };
  }

  if (bodyType === 'json') {
    const bodyText = await p.text({
      message: 'JSON body (or press enter for interactive)',
      placeholder: '{"key": "value"}',
      initialValue: defaults?.body ? JSON.stringify(defaults.body) : '',
    });

    if (p.isCancel(bodyText)) {
      return bodyText;
    }

    if (bodyText) {
      try {
        const parsed = JSON.parse(bodyText);
        return { bodyType: 'json', body: parsed };
      } catch {
        p.log.warn('Invalid JSON, storing as raw string');
        return { bodyType: 'raw', body: bodyText };
      }
    }

    // Interactive JSON builder
    const jsonBody: Record<string, unknown> = {};
    let addMore = true;

    while (addMore) {
      const field = await p.group({
        key: () =>
          p.text({
            message: 'Field name',
            placeholder: 'key',
          }),
        value: () =>
          p.text({
            message: 'Field value',
            placeholder: 'value',
          }),
        type: () =>
          p.select({
            message: 'Value type',
            options: [
              { value: 'string', label: 'String' },
              { value: 'number', label: 'Number' },
              { value: 'boolean', label: 'Boolean' },
            ],
          }),
      });

      if (field.key) {
        if (field.type === 'number') {
          jsonBody[field.key] = Number(field.value);
        } else if (field.type === 'boolean') {
          jsonBody[field.key] = field.value === 'true';
        } else {
          jsonBody[field.key] = field.value;
        }
      }

      const more = await p.confirm({
        message: 'Add another field?',
        initialValue: false,
      });

      if (p.isCancel(more)) {
        return more;
      }
      addMore = more;
    }

    return { bodyType: 'json', body: jsonBody };
  }

  if (bodyType === 'form') {
    const formData: Record<string, string | { file: string }> = {};
    let addMore = true;

    while (addMore) {
      const fieldType = await p.select({
        message: 'Field type',
        options: [
          { value: 'text', label: 'Text field' },
          { value: 'file', label: 'File upload' },
        ],
      });

      if (p.isCancel(fieldType)) {
        return fieldType;
      }

      const fieldName = await p.text({
        message: 'Field name',
        placeholder: fieldType === 'file' ? 'file' : 'field',
        validate: (v) => (!v ? 'Field name required' : undefined),
      });

      if (p.isCancel(fieldName)) {
        return fieldName;
      }

      if (fieldType === 'file') {
        const filePath = await p.text({
          message: 'File path',
          placeholder: './upload.txt',
          validate: (v) => (!v ? 'File path required' : undefined),
        });

        if (p.isCancel(filePath)) {
          return filePath;
        }
        formData[fieldName] = { file: filePath };
      } else {
        const fieldValue = await p.text({
          message: 'Field value',
          placeholder: 'value',
        });

        if (p.isCancel(fieldValue)) {
          return fieldValue;
        }
        formData[fieldName] = fieldValue;
      }

      const more = await p.confirm({
        message: 'Add another field?',
        initialValue: false,
      });

      if (p.isCancel(more)) {
        return more;
      }
      addMore = more;
    }

    return { bodyType: 'form', formData };
  }

  // Raw body
  const rawBody = await p.text({
    message: 'Raw body content',
    placeholder: 'Your content here',
    initialValue: (defaults?.body as string) || '',
  });

  if (p.isCancel(rawBody)) {
    return rawBody;
  }
  return { bodyType: 'raw', body: rawBody };
}

/**
 * Prompts for authentication configuration.
 */
export async function promptAuth(
  defaults?: Partial<WizardAnswers>,
): Promise<Pick<WizardAnswers, 'authType' | 'username' | 'password' | 'token'> | symbol> {
  const authType = await p.select({
    message: 'Authentication',
    initialValue: defaults?.authType || 'none',
    options: [
      { value: 'none', label: 'None' },
      { value: 'bearer', label: 'Bearer Token', hint: 'Authorization: Bearer <token>' },
      { value: 'basic', label: 'Basic Auth', hint: 'Username/password' },
    ],
  });

  if (p.isCancel(authType)) {
    return authType;
  }

  if (authType === 'none') {
    return { authType: 'none' };
  }

  if (authType === 'bearer') {
    const token = await p.text({
      message: 'Bearer token',
      placeholder: 'your-token-here',
      initialValue: defaults?.token || '',
      validate: (v) => (!v ? 'Token required' : undefined),
    });

    if (p.isCancel(token)) {
      return token;
    }
    return { authType: 'bearer', token };
  }

  // Basic auth
  const creds = await p.group({
    username: () =>
      p.text({
        message: 'Username',
        initialValue: defaults?.username || '',
        validate: (v) => (!v ? 'Username required' : undefined),
      }),
    password: () =>
      p.password({
        message: 'Password',
        validate: (v) => (!v ? 'Password required' : undefined),
      }),
  });

  if (p.isCancel(creds.username) || p.isCancel(creds.password)) {
    return Symbol('cancel');
  }

  return { authType: 'basic', username: creds.username, password: creds.password };
}

/**
 * Prompts for advanced options.
 */
export async function promptAdvanced(
  defaults?: Partial<WizardAnswers>,
): Promise<Partial<WizardAnswers> | symbol> {
  const configure = await p.confirm({
    message: 'Configure advanced options?',
    initialValue: false,
  });

  if (p.isCancel(configure) || !configure) {
    return p.isCancel(configure) ? configure : {};
  }

  const options = await p.group({
    timeout: () =>
      p.text({
        message: 'Timeout (ms)',
        placeholder: '30000',
        initialValue: defaults?.timeout?.toString() || '',
        validate: (v) => {
          if (!v) {
            return undefined;
          }
          const num = Number(v);
          if (Number.isNaN(num) || num < 0) {
            return 'Must be a positive number';
          }
        },
      }),
    followRedirects: () =>
      p.confirm({
        message: 'Follow redirects?',
        initialValue: defaults?.followRedirects ?? true,
      }),
    insecure: () =>
      p.confirm({
        message: 'Skip SSL verification? (insecure)',
        initialValue: defaults?.insecure ?? false,
      }),
    http2: () =>
      p.confirm({
        message: 'Use HTTP/2?',
        initialValue: defaults?.http2 ?? false,
      }),
  });

  const result: Partial<WizardAnswers> = {};

  if (options.timeout) {
    result.timeout = Number(options.timeout);
  }
  if (options.followRedirects === false) {
    result.followRedirects = false;
  }
  if (options.insecure) {
    result.insecure = true;
  }
  if (options.http2) {
    result.http2 = true;
  }

  return result;
}

/**
 * Prompts for retry configuration.
 */
export async function promptRetry(
  defaults?: Partial<WizardAnswers>,
): Promise<Partial<WizardAnswers> | symbol> {
  const configure = await p.confirm({
    message: 'Configure retry strategy?',
    initialValue: defaults?.retryEnabled ?? false,
  });

  if (p.isCancel(configure) || !configure) {
    return p.isCancel(configure) ? configure : {};
  }

  const retry = await p.group({
    count: () =>
      p.text({
        message: 'Max retry attempts',
        placeholder: '3',
        initialValue: defaults?.retryCount?.toString() || '3',
        validate: (v) => {
          const num = Number(v);
          if (Number.isNaN(num) || num < 1) {
            return 'Must be at least 1';
          }
        },
      }),
    delay: () =>
      p.text({
        message: 'Retry delay (ms)',
        placeholder: '1000',
        initialValue: defaults?.retryDelay?.toString() || '1000',
        validate: (v) => {
          if (!v) {
            return undefined;
          }
          const num = Number(v);
          if (Number.isNaN(num) || num < 0) {
            return 'Must be a positive number';
          }
        },
      }),
    backoff: () =>
      p.text({
        message: 'Backoff multiplier',
        placeholder: '2 (exponential)',
        initialValue: defaults?.retryBackoff?.toString() || '',
        validate: (v) => {
          if (!v) {
            return undefined;
          }
          const num = Number(v);
          if (Number.isNaN(num) || num < 1) {
            return 'Must be at least 1';
          }
        },
      }),
  });

  return {
    retryEnabled: true,
    retryCount: Number(retry.count),
    retryDelay: retry.delay ? Number(retry.delay) : undefined,
    retryBackoff: retry.backoff ? Number(retry.backoff) : undefined,
  };
}

/**
 * Prompts for validation/expect configuration.
 */
export async function promptValidation(
  defaults?: Partial<WizardAnswers>,
): Promise<Partial<WizardAnswers> | symbol> {
  const configure = await p.confirm({
    message: 'Add response validation?',
    initialValue: !!(defaults?.expectStatus || defaults?.expectResponseTime),
  });

  if (p.isCancel(configure) || !configure) {
    return p.isCancel(configure) ? configure : {};
  }

  const validation = await p.group({
    status: () =>
      p.text({
        message: 'Expected status code(s)',
        placeholder: '200 or 200,201',
        initialValue: defaults?.expectStatus
          ? Array.isArray(defaults.expectStatus)
            ? defaults.expectStatus.join(',')
            : String(defaults.expectStatus)
          : '',
      }),
    responseTime: () =>
      p.text({
        message: 'Response time constraint',
        placeholder: '< 2000 (ms)',
        initialValue: defaults?.expectResponseTime || '',
      }),
  });

  const result: Partial<WizardAnswers> = {};

  if (validation.status) {
    const codes = validation.status.split(',').map((s) => Number(s.trim()));
    result.expectStatus = codes.length === 1 ? codes[0] : codes;
  }
  if (validation.responseTime) {
    result.expectResponseTime = validation.responseTime;
  }

  return result;
}

/**
 * Prompts for output configuration.
 */
export async function promptOutput(
  defaults?: Partial<WizardAnswers>,
): Promise<{ outputPath: string; runAfter: boolean } | symbol> {
  const result = await p.group({
    outputPath: () =>
      p.text({
        message: 'Save to file',
        placeholder: 'api-request.yaml',
        initialValue: defaults?.outputFile || 'request.yaml',
        validate: (v) => {
          if (!v) {
            return 'File path required';
          }
          if (!v.endsWith('.yaml') && !v.endsWith('.yml')) {
            return 'File must end with .yaml or .yml';
          }
        },
      }),
    runAfter: () =>
      p.confirm({
        message: 'Run request after creating?',
        initialValue: true,
      }),
  });

  return result as { outputPath: string; runAfter: boolean };
}
