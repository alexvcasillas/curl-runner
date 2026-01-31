/**
 * Response body parsing utilities.
 */

import type { JsonValue } from '../../types/config';

/**
 * Parses response body, auto-detecting JSON content.
 * Returns parsed JSON if content is JSON, otherwise returns raw string.
 */
export function parseResponseBody(
  body: string | undefined,
  contentType?: string,
): JsonValue | string | undefined {
  if (!body) {
    return body;
  }

  try {
    const isJsonContentType = contentType?.includes('application/json');
    const looksLikeJson = body.trim().startsWith('{') || body.trim().startsWith('[');

    if (isJsonContentType || looksLikeJson) {
      return JSON.parse(body) as JsonValue;
    }
  } catch {
    // Keep as string if JSON parse fails
  }

  return body;
}

/**
 * Checks if content type indicates JSON.
 */
export function isJsonContentType(contentType?: string): boolean {
  return contentType?.includes('application/json') ?? false;
}

/**
 * Checks if string looks like JSON (starts with { or [).
 */
export function looksLikeJson(str: string): boolean {
  const trimmed = str.trim();
  return trimmed.startsWith('{') || trimmed.startsWith('[');
}
