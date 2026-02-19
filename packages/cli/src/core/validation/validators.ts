/**
 * Response validation utilities.
 */

import type {
  ExpectConfig,
  JsonValue,
  ResponseData,
  ValidationResult,
  ValueValidationResult,
} from './types';

/**
 * Validates a response against expected values.
 */
export function validateResponse(result: ResponseData, expect: ExpectConfig): ValidationResult {
  if (!expect) {
    return { success: true };
  }

  const errors: string[] = [];

  // Validate status
  if (expect.status !== undefined) {
    const statusErrors = validateStatus(result.status, expect.status);
    errors.push(...statusErrors);
  }

  // Validate headers
  if (expect.headers) {
    const headerErrors = validateHeaders(result.headers, expect.headers);
    errors.push(...headerErrors);
  }

  // Validate body
  if (expect.body !== undefined) {
    const bodyErrors = validateBodyProperties(result.body, expect.body, '');
    errors.push(...bodyErrors);
  }

  // Validate response time
  if (expect.responseTime !== undefined && result.metrics) {
    const responseTimeMs = result.metrics.duration;
    if (!validateRangePattern(responseTimeMs, expect.responseTime)) {
      errors.push(
        `Expected response time to match ${expect.responseTime}ms, got ${responseTimeMs.toFixed(2)}ms`,
      );
    }
  }

  const hasValidationErrors = errors.length > 0;

  // Handle failure expectation (negative testing)
  if (expect.failure === true) {
    if (hasValidationErrors) {
      return { success: false, error: errors.join('; ') };
    }

    const status = result.status || 0;
    if (status >= 400) {
      return { success: true };
    }
    return {
      success: false,
      error: `Expected request to fail (4xx/5xx) but got status ${status}`,
    };
  }

  // Normal case: expect success
  if (hasValidationErrors) {
    return { success: false, error: errors.join('; ') };
  }
  return { success: true };
}

/**
 * Validates HTTP status code.
 */
export function validateStatus(actual: number | undefined, expected: number | number[]): string[] {
  const expectedStatuses = Array.isArray(expected) ? expected : [expected];
  if (!expectedStatuses.includes(actual || 0)) {
    return [`Expected status ${expectedStatuses.join(' or ')}, got ${actual}`];
  }
  return [];
}

/**
 * Validates HTTP headers.
 */
export function validateHeaders(
  actual: Record<string, string> | undefined,
  expected: Record<string, string>,
): string[] {
  const errors: string[] = [];
  for (const [key, value] of Object.entries(expected)) {
    const actualValue = actual?.[key] || actual?.[key.toLowerCase()];
    if (actualValue !== value) {
      errors.push(`Expected header ${key}="${value}", got "${actualValue}"`);
    }
  }
  return errors;
}

/**
 * Validates body properties recursively.
 */
export function validateBodyProperties(
  actualBody: JsonValue,
  expectedBody: JsonValue,
  path: string,
): string[] {
  const errors: string[] = [];

  if (typeof expectedBody !== 'object' || expectedBody === null) {
    const validationResult = validateValue(actualBody, expectedBody, path || 'body');
    if (!validationResult.isValid) {
      errors.push(validationResult.error!);
    }
    return errors;
  }

  // Array validation
  if (Array.isArray(expectedBody)) {
    if (Array.isArray(actualBody)) {
      // Element-wise: each expected item must find a matching actual item
      for (let i = 0; i < expectedBody.length; i++) {
        const expectedItem = expectedBody[i];
        const itemPath = `${path || 'body'}[${i}]`;

        if (typeof expectedItem === 'object' && expectedItem !== null) {
          const found = actualBody.some((actualItem) => {
            const itemErrors = validateBodyProperties(actualItem, expectedItem, itemPath);
            return itemErrors.length === 0;
          });
          if (!found) {
            errors.push(
              `Expected ${path || 'body'} to contain item matching ${JSON.stringify(expectedItem)}, but no match found`,
            );
          }
        } else {
          const found = actualBody.some((actualItem) => {
            const result = validateValue(actualItem, expectedItem, itemPath);
            return result.isValid;
          });
          if (!found) {
            errors.push(
              `Expected ${path || 'body'} to contain item matching ${JSON.stringify(expectedItem)}, but no match found`,
            );
          }
        }
      }
      return errors;
    }

    // Actual is scalar, expected is array → "one of" matching
    const validationResult = validateValue(actualBody, expectedBody, path || 'body');
    if (!validationResult.isValid) {
      errors.push(validationResult.error!);
    }
    return errors;
  }

  // Object property comparison with array selector support
  for (const [key, expectedValue] of Object.entries(expectedBody)) {
    const currentPath = path ? `${path}.${key}` : key;
    let actualValue: JsonValue;

    // Handle array selectors like [0], [-1], *, slice(0,3)
    if (Array.isArray(actualBody) && isArraySelector(key)) {
      actualValue = getArrayValue(actualBody, key);
    } else {
      actualValue = (actualBody as Record<string, JsonValue>)?.[key];
    }

    if (typeof expectedValue === 'object' && expectedValue !== null) {
      // Recursive validation for nested objects and arrays
      const nestedErrors = validateBodyProperties(actualValue, expectedValue, currentPath);
      errors.push(...nestedErrors);
    } else {
      // Value validation
      const validationResult = validateValue(actualValue, expectedValue, currentPath);
      if (!validationResult.isValid) {
        errors.push(validationResult.error!);
      }
    }
  }

  return errors;
}

/**
 * Validates a single value against expected value.
 */
export function validateValue(
  actualValue: JsonValue,
  expectedValue: JsonValue,
  path: string,
): ValueValidationResult {
  // Wildcard - accept any value
  if (expectedValue === '*') {
    return { isValid: true };
  }

  // Multiple acceptable values (array)
  if (Array.isArray(expectedValue)) {
    const isMatch = expectedValue.some((expected) => {
      if (expected === '*') {
        return true;
      }
      if (typeof expected === 'string' && isRegexPattern(expected)) {
        return validateRegexPattern(actualValue, expected);
      }
      if (typeof expected === 'string' && isRangePattern(expected)) {
        return validateRangePattern(actualValue, expected);
      }
      return actualValue === expected;
    });

    if (!isMatch) {
      return {
        isValid: false,
        error: `Expected ${path} to match one of ${JSON.stringify(expectedValue)}, got ${JSON.stringify(actualValue)}`,
      };
    }
    return { isValid: true };
  }

  // Regex pattern validation
  if (typeof expectedValue === 'string' && isRegexPattern(expectedValue)) {
    if (!validateRegexPattern(actualValue, expectedValue)) {
      return {
        isValid: false,
        error: `Expected ${path} to match pattern ${expectedValue}, got ${JSON.stringify(actualValue)}`,
      };
    }
    return { isValid: true };
  }

  // Numeric range validation
  if (typeof expectedValue === 'string' && isRangePattern(expectedValue)) {
    if (!validateRangePattern(actualValue, expectedValue)) {
      return {
        isValid: false,
        error: `Expected ${path} to match range ${expectedValue}, got ${JSON.stringify(actualValue)}`,
      };
    }
    return { isValid: true };
  }

  // Null handling
  if (expectedValue === 'null' || expectedValue === null) {
    if (actualValue !== null) {
      return {
        isValid: false,
        error: `Expected ${path} to be null, got ${JSON.stringify(actualValue)}`,
      };
    }
    return { isValid: true };
  }

  // Exact value comparison
  if (actualValue !== expectedValue) {
    return {
      isValid: false,
      error: `Expected ${path} to be ${JSON.stringify(expectedValue)}, got ${JSON.stringify(actualValue)}`,
    };
  }

  return { isValid: true };
}

/**
 * Checks if a string is a regex pattern.
 */
export function isRegexPattern(pattern: string): boolean {
  return (
    pattern.startsWith('^') ||
    pattern.endsWith('$') ||
    pattern.includes('\\d') ||
    pattern.includes('\\w') ||
    pattern.includes('\\s') ||
    pattern.includes('[') ||
    pattern.includes('*') ||
    pattern.includes('+') ||
    pattern.includes('?')
  );
}

/**
 * Validates a value against a regex pattern.
 */
export function validateRegexPattern(actualValue: JsonValue, pattern: string): boolean {
  const stringValue = String(actualValue);
  try {
    const regex = new RegExp(pattern);
    return regex.test(stringValue);
  } catch {
    return false;
  }
}

/**
 * Checks if a string is a range pattern.
 */
export function isRangePattern(pattern: string): boolean {
  return /^(>=?|<=?|>|<)\s*[\d.-]+(\s*,\s*(>=?|<=?|>|<)\s*[\d.-]+)*$/.test(pattern);
}

/**
 * Validates a value against a numeric range pattern.
 */
export function validateRangePattern(actualValue: JsonValue, pattern: string): boolean {
  const numValue = Number(actualValue);
  if (Number.isNaN(numValue)) {
    return false;
  }

  // Handle range like "10 - 50" or "10-50"
  const rangeMatch = pattern.match(/^([\d.-]+)\s*-\s*([\d.-]+)$/);
  if (rangeMatch) {
    const min = Number(rangeMatch[1]);
    const max = Number(rangeMatch[2]);
    return numValue >= min && numValue <= max;
  }

  // Handle comma-separated conditions like ">= 0, <= 100"
  const conditions = pattern.split(',').map((c) => c.trim());
  return conditions.every((condition) => {
    const match = condition.match(/^(>=?|<=?|>|<)\s*([\d.-]+)$/);
    if (!match) {
      return false;
    }

    const operator = match[1];
    const targetValue = Number(match[2]);

    switch (operator) {
      case '>':
        return numValue > targetValue;
      case '>=':
        return numValue >= targetValue;
      case '<':
        return numValue < targetValue;
      case '<=':
        return numValue <= targetValue;
      default:
        return false;
    }
  });
}

/**
 * Checks if a key is an array selector.
 */
export function isArraySelector(key: string): boolean {
  return /^\[.*\]$/.test(key) || key === '*' || key.startsWith('slice(');
}

/**
 * Gets a value from an array using a selector.
 */
export function getArrayValue(array: JsonValue[], selector: string): JsonValue {
  if (selector === '*') {
    return array;
  }

  if (selector.startsWith('[') && selector.endsWith(']')) {
    const index = selector.slice(1, -1);
    if (index === '*') {
      return array;
    }

    const numIndex = Number(index);
    if (!Number.isNaN(numIndex)) {
      return numIndex >= 0 ? array[numIndex] : array[array.length + numIndex];
    }
  }

  if (selector.startsWith('slice(')) {
    const match = selector.match(/slice\((\d+)(?:,(\d+))?\)/);
    if (match) {
      const start = Number(match[1]);
      const end = match[2] ? Number(match[2]) : undefined;
      return array.slice(start, end);
    }
  }

  return undefined as unknown as JsonValue;
}
