/**
 * Secret redaction utility for masking sensitive values in output.
 * Replaces secret values with [REDACTED] to prevent accidental exposure.
 */

import type { RequestConfig } from '../types/config';

const REDACTED = '[REDACTED]';

/**
 * Header names whose values should be treated as secrets.
 */
const SENSITIVE_HEADER_RE = /authorization|api[-_]?key|token|secret|cookie/i;

/**
 * Common secret patterns to auto-detect.
 * These match known API key formats from popular services.
 */
const SECRET_PATTERNS: RegExp[] = [
  // Stripe
  /sk_live_[a-zA-Z0-9]{24,}/g,
  /sk_test_[a-zA-Z0-9]{24,}/g,
  /pk_live_[a-zA-Z0-9]{24,}/g,
  /pk_test_[a-zA-Z0-9]{24,}/g,
  /rk_live_[a-zA-Z0-9]{24,}/g,
  /rk_test_[a-zA-Z0-9]{24,}/g,
  // AWS
  /AKIA[0-9A-Z]{16}/g,
  // GitHub
  /ghp_[a-zA-Z0-9]{36,}/g,
  /gho_[a-zA-Z0-9]{36,}/g,
  /ghu_[a-zA-Z0-9]{36,}/g,
  /ghs_[a-zA-Z0-9]{36,}/g,
  /ghr_[a-zA-Z0-9]{36,}/g,
  // NPM
  /npm_[a-zA-Z0-9]{36,}/g,
  // Slack
  /xox[baprs]-[a-zA-Z0-9-]{10,}/g,
  // Paddle
  /pdl_[a-zA-Z0-9]{20,}/g,
  // OpenAI
  /sk-[a-zA-Z0-9]{48,}/g,
  // Anthropic
  /sk-ant-[a-zA-Z0-9-]{40,}/g,
  // Generic Bearer tokens (long alphanumeric)
  /Bearer [a-zA-Z0-9_-]{40,}/g,
  // JWT (three base64url segments)
  /eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+/g,
  // Google API key
  /AIza[0-9A-Za-z_-]{35}/g,
  // Generic api_key / token / secret query/form assignment — redact value only
  /(?:api[_-]?key|token|secret)=([^&\s"']+)/g,
];

export class SecretRedactor {
  private secrets: Map<string, string> = new Map();
  private sortedSecrets: string[] = [];
  private patternRedaction = true;

  /**
   * Registers secrets for redaction.
   * Longer secrets are matched first to handle overlapping values.
   */
  addSecrets(secrets: Record<string, string>): void {
    for (const [key, value] of Object.entries(secrets)) {
      if (value && value.length > 0) {
        this.secrets.set(key, value);
      }
    }
    this.updateSortedSecrets();
  }

  /**
   * Registers a single secret value.
   */
  addSecret(key: string, value: string): void {
    if (value && value.length > 0) {
      this.secrets.set(key, value);
      this.updateSortedSecrets();
    }
  }

  /**
   * Enable/disable pattern-based redaction.
   */
  setPatternRedaction(enabled: boolean): void {
    this.patternRedaction = enabled;
  }

  /**
   * Clears all registered secrets.
   */
  clear(): void {
    this.secrets.clear();
    this.sortedSecrets = [];
  }

  /**
   * Updates sorted secrets list (longest first for proper replacement).
   */
  private updateSortedSecrets(): void {
    this.sortedSecrets = Array.from(this.secrets.values())
      .filter((v) => v.length > 0)
      .sort((a, b) => b.length - a.length);
  }

  /**
   * Redacts all registered secret values from a string.
   * Also applies pattern-based redaction for common API key formats.
   */
  redact(input: string): string {
    let result = input;

    // First, redact explicit secrets (longest first)
    for (const secret of this.sortedSecrets) {
      result = result.split(secret).join(REDACTED);
    }

    // Then apply pattern-based redaction
    if (this.patternRedaction) {
      for (const pattern of SECRET_PATTERNS) {
        // Reset lastIndex for global regexes
        pattern.lastIndex = 0;
        // Patterns with capture groups: redact only the captured value (group 1)
        if (pattern.source.includes('(')) {
          result = result.replace(pattern, (match, group1) =>
            group1 ? match.replace(group1, REDACTED) : REDACTED,
          );
        } else {
          result = result.replace(pattern, REDACTED);
        }
      }
    }

    return result;
  }

  /**
   * Returns true if any secrets are registered.
   */
  hasSecrets(): boolean {
    return this.secrets.size > 0;
  }

  /**
   * Returns count of registered secrets.
   */
  get secretCount(): number {
    return this.secrets.size;
  }
}

// Global singleton for easy access
let globalRedactor: SecretRedactor | null = null;

export function getGlobalRedactor(): SecretRedactor {
  if (!globalRedactor) {
    globalRedactor = new SecretRedactor();
  }
  return globalRedactor;
}

export function resetGlobalRedactor(): void {
  globalRedactor = null;
}

/**
 * Registers inline secrets found in request configs into the global redactor.
 * Call after variable interpolation and before execution/logging.
 */
export function registerRequestSecrets(requests: RequestConfig[]): void {
  const redactor = getGlobalRedactor();

  for (const req of requests) {
    const auth = req.auth;
    if (auth) {
      if (auth.type === 'bearer' && auth.token) {
        redactor.addSecret(`__req_bearer_${auth.token.slice(0, 8)}`, auth.token);
      } else if (auth.type === 'basic') {
        if (auth.password) {
          redactor.addSecret(`__req_basic_pass_${auth.password.slice(0, 8)}`, auth.password);
        }
        if (auth.username && auth.password) {
          const combined = `${auth.username}:${auth.password}`;
          redactor.addSecret(`__req_basic_combined_${combined.slice(0, 8)}`, combined);
        }
      }
    }

    if (req.headers) {
      for (const [name, value] of Object.entries(req.headers)) {
        if (SENSITIVE_HEADER_RE.test(name) && value) {
          redactor.addSecret(`__req_header_${name.toLowerCase()}_${value.slice(0, 8)}`, value);
          // For Authorization: Bearer <tok>, also register the bare token
          const bearerMatch = /^Bearer\s+(\S+)$/i.exec(value);
          if (bearerMatch) {
            const bare = bearerMatch[1];
            redactor.addSecret(`__req_header_bearer_bare_${bare.slice(0, 8)}`, bare);
          }
        }
      }
    }
  }
}
