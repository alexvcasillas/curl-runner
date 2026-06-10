/**
 * URL protocol allow-list guard.
 * Pure, side-effect-free validation used at execution time.
 */

export const DEFAULT_ALLOWED_PROTOCOLS = ['http', 'https'];

export interface UrlValidationResult {
  valid: boolean;
  error?: string;
}

/**
 * Validates that a URL uses an allowed protocol.
 * Call after interpolation so store-injected URLs are also checked.
 */
export function validateUrl(url: string, allowedProtocols: string[]): UrlValidationResult {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return { valid: false, error: `Invalid URL: "${url}"` };
  }

  // URL.protocol includes the trailing colon (e.g. "https:")
  const proto = parsed.protocol.slice(0, -1).toLowerCase();

  if (!allowedProtocols.includes(proto)) {
    const allowed = allowedProtocols.join(', ');
    return {
      valid: false,
      error: `Blocked protocol "${proto}" (allowed: ${allowed}). Use --allow-protocol to permit it.`,
    };
  }

  return { valid: true };
}
