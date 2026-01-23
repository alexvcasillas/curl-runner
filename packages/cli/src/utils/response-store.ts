import type { ExecutionResult, ResponseStoreContext, StoreConfig } from '../types/config';

/**
 * Extracts a value from an object using a dot-notation path.
 * Supports paths like: "body.id", "body.data.token", "headers.content-type", "status"
 *
 * @param obj - The object to extract from
 * @param path - Dot-notation path to the value
 * @returns The extracted value or undefined if not found
 */
export function getValueByPath(obj: unknown, path: string): unknown {
  const parts = path.split('.');
  let current: unknown = obj;

  for (const part of parts) {
    if (current === null || current === undefined) {
      return undefined;
    }

    if (typeof current !== 'object') {
      return undefined;
    }

    // Handle array index access like "items.0.id" or "items[0].id"
    const arrayMatch = part.match(/^(\w+)\[(\d+)\]$/);
    if (arrayMatch) {
      const [, key, indexStr] = arrayMatch;
      const index = Number.parseInt(indexStr, 10);
      current = (current as Record<string, unknown>)[key];
      if (Array.isArray(current)) {
        current = current[index];
      } else {
        return undefined;
      }
    } else if (/^\d+$/.test(part) && Array.isArray(current)) {
      // Direct numeric index for arrays
      current = current[Number.parseInt(part, 10)];
    } else {
      current = (current as Record<string, unknown>)[part];
    }
  }

  return current;
}

/**
 * Converts a value to a string for storage.
 * Objects and arrays are JSON stringified.
 */
export function valueToString(value: unknown): string {
  if (value === undefined || value === null) {
    return '';
  }
  if (typeof value === 'string') {
    return value;
  }
  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value);
  }
  return JSON.stringify(value);
}

/**
 * Extracts values from an execution result based on the store configuration.
 *
 * @param result - The execution result to extract from
 * @param storeConfig - Configuration mapping variable names to JSON paths
 * @returns Object containing the extracted values as strings
 */
export function extractStoreValues(
  result: ExecutionResult,
  storeConfig: StoreConfig,
): ResponseStoreContext {
  const extracted: ResponseStoreContext = {};

  // Build an object that represents the full response structure
  const responseObj: Record<string, unknown> = {
    status: result.status,
    headers: result.headers || {},
    body: result.body,
    metrics: result.metrics,
  };

  for (const [varName, path] of Object.entries(storeConfig)) {
    const value = getValueByPath(responseObj, path);
    extracted[varName] = valueToString(value);
  }

  return extracted;
}

/**
 * Creates a new response store context.
 */
export function createStoreContext(): ResponseStoreContext {
  return {};
}

/**
 * Merges new values into an existing store context.
 * New values override existing ones with the same key.
 */
export function mergeStoreContext(
  existing: ResponseStoreContext,
  newValues: ResponseStoreContext,
): ResponseStoreContext {
  return { ...existing, ...newValues };
}
