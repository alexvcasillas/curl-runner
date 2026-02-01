/**
 * Secret redaction utility for masking sensitive values in output.
 * Replaces secret values with [REDACTED] to prevent accidental exposure.
 */

const REDACTED = '[REDACTED]';

export class SecretRedactor {
  private secrets: Map<string, string> = new Map();
  private sortedSecrets: string[] = [];

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
   */
  redact(input: string): string {
    if (this.sortedSecrets.length === 0) {
      return input;
    }

    let result = input;
    for (const secret of this.sortedSecrets) {
      // Use global replace for all occurrences
      result = result.split(secret).join(REDACTED);
    }
    return result;
  }

  /**
   * Redacts secrets from an object (deep).
   */
  redactObject<T>(obj: T): T {
    if (obj === null || obj === undefined) {
      return obj;
    }

    if (typeof obj === 'string') {
      return this.redact(obj) as T;
    }

    if (Array.isArray(obj)) {
      return obj.map((item) => this.redactObject(item)) as T;
    }

    if (typeof obj === 'object') {
      const result: Record<string, unknown> = {};
      for (const [key, value] of Object.entries(obj)) {
        result[key] = this.redactObject(value);
      }
      return result as T;
    }

    return obj;
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
