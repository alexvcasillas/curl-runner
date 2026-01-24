import * as path from 'node:path';
import type {
  ExecutionResult,
  GlobalSnapshotConfig,
  JsonValue,
  Snapshot,
  SnapshotCompareResult,
  SnapshotConfig,
  SnapshotFile,
} from '../types/config';
import { SnapshotDiffer } from './snapshot-differ';

const SNAPSHOT_VERSION = 1;
const DEFAULT_SNAPSHOT_DIR = '__snapshots__';

/**
 * Manages snapshot files: reading, writing, comparing, and updating.
 */
export class SnapshotManager {
  private snapshotDir: string;
  private globalConfig: GlobalSnapshotConfig;
  private writeLocks: Map<string, Promise<void>> = new Map();

  constructor(globalConfig: GlobalSnapshotConfig = {}) {
    this.globalConfig = globalConfig;
    this.snapshotDir = globalConfig.dir || DEFAULT_SNAPSHOT_DIR;
  }

  /**
   * Gets the snapshot file path for a YAML file.
   */
  getSnapshotPath(yamlPath: string): string {
    const dir = path.dirname(yamlPath);
    const basename = path.basename(yamlPath, path.extname(yamlPath));
    return path.join(dir, this.snapshotDir, `${basename}.snap.json`);
  }

  /**
   * Loads snapshot file for a YAML file.
   */
  async load(yamlPath: string): Promise<SnapshotFile | null> {
    const snapshotPath = this.getSnapshotPath(yamlPath);
    try {
      const file = Bun.file(snapshotPath);
      if (!(await file.exists())) {
        return null;
      }
      const content = await file.text();
      return JSON.parse(content) as SnapshotFile;
    } catch {
      return null;
    }
  }

  /**
   * Saves snapshot file with write queue for parallel safety.
   */
  async save(yamlPath: string, data: SnapshotFile): Promise<void> {
    const snapshotPath = this.getSnapshotPath(yamlPath);

    // Queue writes to prevent race conditions
    const existingLock = this.writeLocks.get(snapshotPath);
    const writePromise = (async () => {
      if (existingLock) {
        await existingLock;
      }

      // Ensure directory exists
      const dir = path.dirname(snapshotPath);
      const fs = await import('node:fs/promises');
      await fs.mkdir(dir, { recursive: true });

      // Write with pretty formatting
      const content = JSON.stringify(data, null, 2);
      await Bun.write(snapshotPath, content);
    })();

    this.writeLocks.set(snapshotPath, writePromise);
    await writePromise;
    this.writeLocks.delete(snapshotPath);
  }

  /**
   * Gets a single snapshot by request name.
   */
  async get(yamlPath: string, requestName: string): Promise<Snapshot | null> {
    const file = await this.load(yamlPath);
    return file?.snapshots[requestName] || null;
  }

  /**
   * Creates a snapshot from execution result.
   */
  createSnapshot(result: ExecutionResult, config: SnapshotConfig): Snapshot {
    const include = config.include || ['body'];
    const snapshot: Snapshot = {
      hash: '',
      updatedAt: new Date().toISOString(),
    };

    if (include.includes('status') && result.status !== undefined) {
      snapshot.status = result.status;
    }

    if (include.includes('headers') && result.headers) {
      // Normalize headers: lowercase keys, sorted
      snapshot.headers = this.normalizeHeaders(result.headers);
    }

    if (include.includes('body') && result.body !== undefined) {
      snapshot.body = result.body;
    }

    // Generate hash from content
    snapshot.hash = this.hash(snapshot);

    return snapshot;
  }

  /**
   * Normalizes headers for consistent comparison.
   */
  private normalizeHeaders(headers: Record<string, string>): Record<string, string> {
    const normalized: Record<string, string> = {};
    const sortedKeys = Object.keys(headers).sort();
    for (const key of sortedKeys) {
      normalized[key.toLowerCase()] = headers[key];
    }
    return normalized;
  }

  /**
   * Generates a hash for snapshot content.
   */
  hash(content: unknown): string {
    const str = JSON.stringify(content);
    const hasher = new Bun.CryptoHasher('md5');
    hasher.update(str);
    return hasher.digest('hex').slice(0, 8);
  }

  /**
   * Compares execution result against stored snapshot and optionally updates.
   */
  async compareAndUpdate(
    yamlPath: string,
    requestName: string,
    result: ExecutionResult,
    config: SnapshotConfig,
  ): Promise<SnapshotCompareResult> {
    const snapshotName = config.name || requestName;
    const existingSnapshot = await this.get(yamlPath, snapshotName);
    const newSnapshot = this.createSnapshot(result, config);

    // No existing snapshot
    if (!existingSnapshot) {
      if (this.globalConfig.ci) {
        // CI mode: fail on missing snapshot
        return {
          match: false,
          isNew: true,
          updated: false,
          differences: [
            {
              path: '',
              expected: 'snapshot',
              received: 'none',
              type: 'removed',
            },
          ],
        };
      }

      // Create new snapshot
      await this.updateSnapshot(yamlPath, snapshotName, newSnapshot);
      return {
        match: true,
        isNew: true,
        updated: true,
        differences: [],
      };
    }

    // Compare snapshots
    const differ = new SnapshotDiffer(config);
    const diffResult = differ.compare(existingSnapshot, newSnapshot);

    if (diffResult.match) {
      return {
        match: true,
        isNew: false,
        updated: false,
        differences: [],
      };
    }

    // Handle update modes
    const updateMode = this.globalConfig.updateMode || 'none';
    if (updateMode === 'all' || updateMode === 'failing') {
      await this.updateSnapshot(yamlPath, snapshotName, newSnapshot);
      return {
        match: true,
        isNew: false,
        updated: true,
        differences: diffResult.differences,
      };
    }

    return {
      match: false,
      isNew: false,
      updated: false,
      differences: diffResult.differences,
    };
  }

  /**
   * Updates a single snapshot in the file.
   */
  private async updateSnapshot(
    yamlPath: string,
    snapshotName: string,
    snapshot: Snapshot,
  ): Promise<void> {
    let file = await this.load(yamlPath);
    if (!file) {
      file = {
        version: SNAPSHOT_VERSION,
        snapshots: {},
      };
    }

    file.snapshots[snapshotName] = snapshot;
    await this.save(yamlPath, file);
  }

  /**
   * Merges request-level config with global config.
   */
  static mergeConfig(
    globalConfig: GlobalSnapshotConfig | undefined,
    requestConfig: SnapshotConfig | boolean | undefined,
  ): SnapshotConfig | null {
    // Not enabled
    if (!requestConfig && !globalConfig?.enabled) {
      return null;
    }

    // Simple boolean enable
    if (requestConfig === true) {
      return {
        enabled: true,
        include: globalConfig?.include || ['body'],
        exclude: globalConfig?.exclude || [],
        match: globalConfig?.match || {},
      };
    }

    // Detailed config
    if (typeof requestConfig === 'object' && requestConfig.enabled !== false) {
      return {
        enabled: true,
        name: requestConfig.name,
        include: requestConfig.include || globalConfig?.include || ['body'],
        exclude: [...(globalConfig?.exclude || []), ...(requestConfig.exclude || [])],
        match: { ...(globalConfig?.match || {}), ...(requestConfig.match || {}) },
      };
    }

    // Global enabled but request not specified
    if (globalConfig?.enabled && requestConfig === undefined) {
      return {
        enabled: true,
        include: globalConfig.include || ['body'],
        exclude: globalConfig.exclude || [],
        match: globalConfig.match || {},
      };
    }

    return null;
  }
}

/**
 * Extracts body content for snapshot, applying exclusions.
 */
export function filterSnapshotBody(body: JsonValue, exclude: string[]): JsonValue {
  if (body === null || typeof body !== 'object') {
    return body;
  }

  const bodyExcludes = exclude.filter((p) => p.startsWith('body.')).map((p) => p.slice(5)); // Remove 'body.' prefix

  if (bodyExcludes.length === 0) {
    return body;
  }

  return filterObject(body, bodyExcludes, '');
}

function filterObject(obj: JsonValue, excludes: string[], currentPath: string): JsonValue {
  if (obj === null || typeof obj !== 'object') {
    return obj;
  }

  if (Array.isArray(obj)) {
    return obj.map((item, index) => {
      const itemPath = currentPath ? `${currentPath}[${index}]` : `[${index}]`;
      return filterObject(item, excludes, itemPath);
    });
  }

  const result: Record<string, JsonValue> = {};
  for (const [key, value] of Object.entries(obj)) {
    const fullPath = currentPath ? `${currentPath}.${key}` : key;

    // Check if this path should be excluded
    const shouldExclude = excludes.some((pattern) => {
      // Exact match
      if (pattern === fullPath) {
        return true;
      }
      // Wildcard match (e.g., '*.timestamp' matches 'user.timestamp')
      if (pattern.startsWith('*.')) {
        const suffix = pattern.slice(2);
        return fullPath.endsWith(`.${suffix}`) || fullPath === suffix;
      }
      // Array wildcard (e.g., '[*].id')
      if (pattern.includes('[*]')) {
        const regex = new RegExp(`^${pattern.replace(/\[\*\]/g, '\\[\\d+\\]')}$`);
        return regex.test(fullPath);
      }
      return false;
    });

    if (!shouldExclude) {
      result[key] = filterObject(value, excludes, fullPath);
    }
  }

  return result;
}
