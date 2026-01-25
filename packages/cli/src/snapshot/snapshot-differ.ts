import type { JsonValue, Snapshot, SnapshotConfig, SnapshotDiff } from '../types/config';

interface DiffResult {
  match: boolean;
  differences: SnapshotDiff[];
}

/**
 * Compares snapshots with support for exclusions, wildcards, and regex patterns.
 */
export class SnapshotDiffer {
  private excludePaths: Set<string>;
  private matchRules: Map<string, string>;

  constructor(config: SnapshotConfig) {
    this.excludePaths = new Set(config.exclude || []);
    this.matchRules = new Map(Object.entries(config.match || {}));
  }

  /**
   * Compares two snapshots and returns differences.
   */
  compare(expected: Snapshot, received: Snapshot): DiffResult {
    const differences: SnapshotDiff[] = [];

    // Compare status
    if (expected.status !== undefined || received.status !== undefined) {
      if (expected.status !== received.status && !this.isExcluded('status')) {
        differences.push({
          path: 'status',
          expected: expected.status,
          received: received.status,
          type: 'changed',
        });
      }
    }

    // Compare headers
    if (expected.headers || received.headers) {
      const headerDiffs = this.compareObjects(
        expected.headers || {},
        received.headers || {},
        'headers',
      );
      differences.push(...headerDiffs);
    }

    // Compare body
    if (expected.body !== undefined || received.body !== undefined) {
      const bodyDiffs = this.deepCompare(expected.body, received.body, 'body');
      differences.push(...bodyDiffs);
    }

    return {
      match: differences.length === 0,
      differences,
    };
  }

  /**
   * Deep comparison of two values with path tracking.
   */
  deepCompare(expected: unknown, received: unknown, path: string): SnapshotDiff[] {
    // Check if path is excluded
    if (this.isExcluded(path)) {
      return [];
    }

    // Check match rules (wildcards, regex)
    if (this.matchesRule(path, received)) {
      return [];
    }

    // Both null/undefined
    if (expected === null && received === null) {
      return [];
    }
    if (expected === undefined && received === undefined) {
      return [];
    }

    // Type mismatch
    const expectedType = this.getType(expected);
    const receivedType = this.getType(received);
    if (expectedType !== receivedType) {
      return [
        {
          path,
          expected,
          received,
          type: 'type_mismatch',
        },
      ];
    }

    // Primitives
    if (expectedType !== 'object' && expectedType !== 'array') {
      if (expected !== received) {
        return [
          {
            path,
            expected,
            received,
            type: 'changed',
          },
        ];
      }
      return [];
    }

    // Arrays
    if (expectedType === 'array') {
      return this.compareArrays(expected as JsonValue[], received as JsonValue[], path);
    }

    // Objects
    return this.compareObjects(
      expected as Record<string, unknown>,
      received as Record<string, unknown>,
      path,
    );
  }

  /**
   * Compares two arrays.
   */
  private compareArrays(
    expected: JsonValue[],
    received: JsonValue[],
    path: string,
  ): SnapshotDiff[] {
    const differences: SnapshotDiff[] = [];

    // Check length difference
    const maxLen = Math.max(expected.length, received.length);

    for (let i = 0; i < maxLen; i++) {
      const itemPath = `${path}[${i}]`;

      if (i >= expected.length) {
        // Added item
        if (!this.isExcluded(itemPath)) {
          differences.push({
            path: itemPath,
            expected: undefined,
            received: received[i],
            type: 'added',
          });
        }
      } else if (i >= received.length) {
        // Removed item
        if (!this.isExcluded(itemPath)) {
          differences.push({
            path: itemPath,
            expected: expected[i],
            received: undefined,
            type: 'removed',
          });
        }
      } else {
        // Compare items
        const itemDiffs = this.deepCompare(expected[i], received[i], itemPath);
        differences.push(...itemDiffs);
      }
    }

    return differences;
  }

  /**
   * Compares two objects.
   */
  private compareObjects(
    expected: Record<string, unknown>,
    received: Record<string, unknown>,
    path: string,
  ): SnapshotDiff[] {
    const differences: SnapshotDiff[] = [];
    const allKeys = new Set([...Object.keys(expected), ...Object.keys(received)]);

    for (const key of allKeys) {
      const keyPath = path ? `${path}.${key}` : key;
      const hasExpected = key in expected;
      const hasReceived = key in received;

      if (!hasExpected && hasReceived) {
        // Added key
        if (!this.isExcluded(keyPath)) {
          differences.push({
            path: keyPath,
            expected: undefined,
            received: received[key],
            type: 'added',
          });
        }
      } else if (hasExpected && !hasReceived) {
        // Removed key
        if (!this.isExcluded(keyPath)) {
          differences.push({
            path: keyPath,
            expected: expected[key],
            received: undefined,
            type: 'removed',
          });
        }
      } else {
        // Compare values
        const keyDiffs = this.deepCompare(expected[key], received[key], keyPath);
        differences.push(...keyDiffs);
      }
    }

    return differences;
  }

  /**
   * Checks if a path should be excluded from comparison.
   */
  isExcluded(path: string): boolean {
    // Exact match
    if (this.excludePaths.has(path)) {
      return true;
    }

    for (const pattern of this.excludePaths) {
      // Wildcard prefix (e.g., '*.timestamp')
      if (pattern.startsWith('*.')) {
        const suffix = pattern.slice(2);
        if (path.endsWith(`.${suffix}`)) {
          return true;
        }
        // Also match root level (e.g., 'timestamp' matches '*.timestamp')
        const lastPart = path.split('.').pop();
        if (lastPart === suffix) {
          return true;
        }
      }

      // Array wildcard (e.g., 'body[*].id')
      if (pattern.includes('[*]')) {
        const regex = new RegExp(
          `^${pattern.replace(/\[\*\]/g, '\\[\\d+\\]').replace(/\./g, '\\.')}$`,
        );
        if (regex.test(path)) {
          return true;
        }
      }
    }

    return false;
  }

  /**
   * Checks if a value matches a custom rule for its path.
   */
  matchesRule(path: string, value: unknown): boolean {
    const rule = this.matchRules.get(path);
    if (!rule) {
      return false;
    }

    // Wildcard: accept any value
    if (rule === '*') {
      return true;
    }

    // Regex pattern
    if (rule.startsWith('regex:')) {
      const pattern = rule.slice(6);
      try {
        const regex = new RegExp(pattern);
        return regex.test(String(value));
      } catch {
        return false;
      }
    }

    return false;
  }

  /**
   * Gets the type of a value for comparison.
   */
  private getType(value: unknown): string {
    if (value === null) {
      return 'null';
    }
    if (value === undefined) {
      return 'undefined';
    }
    if (Array.isArray(value)) {
      return 'array';
    }
    return typeof value;
  }
}
