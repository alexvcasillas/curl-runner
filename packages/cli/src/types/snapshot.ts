/**
 * Snapshot testing types.
 */

import type { JsonValue } from './json';

/**
 * Configuration for snapshot testing.
 * Snapshots save response data and compare future runs against them.
 */
export interface SnapshotConfig {
  /** Enable snapshot testing for this request. */
  enabled?: boolean;
  /** Custom snapshot name (defaults to request name). */
  name?: string;
  /** What to include in snapshot. Default: ['body'] */
  include?: ('body' | 'status' | 'headers')[];
  /** Paths to exclude from comparison (e.g., 'body.timestamp'). */
  exclude?: string[];
  /** Match rules for dynamic values (path -> '*' or 'regex:pattern'). */
  match?: Record<string, string>;
}

/**
 * Global snapshot configuration.
 */
export interface GlobalSnapshotConfig extends SnapshotConfig {
  /** Directory for snapshot files. Default: '__snapshots__' */
  dir?: string;
  /** Update mode: 'none' | 'all' | 'failing'. Default: 'none' */
  updateMode?: 'none' | 'all' | 'failing';
  /** CI mode: fail if snapshot is missing. Default: false */
  ci?: boolean;
}

/**
 * Stored snapshot data for a single request.
 */
export interface Snapshot {
  status?: number;
  headers?: Record<string, string>;
  body?: JsonValue;
  hash: string;
  updatedAt: string;
}

/**
 * Snapshot file format.
 */
export interface SnapshotFile {
  version: number;
  snapshots: Record<string, Snapshot>;
}

/**
 * Result of comparing a response against a snapshot.
 */
export interface SnapshotDiff {
  path: string;
  expected: unknown;
  received: unknown;
  type: 'added' | 'removed' | 'changed' | 'type_mismatch';
}

/**
 * Result of snapshot comparison.
 */
export interface SnapshotCompareResult {
  match: boolean;
  isNew: boolean;
  updated: boolean;
  differences: SnapshotDiff[];
}
