import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import type {
  Baseline,
  BaselineFile,
  DiffConfig,
  ExecutionResult,
  GlobalDiffConfig,
} from '../types/config';

const BASELINE_VERSION = 1;
const DEFAULT_BASELINE_DIR = '__baselines__';

/**
 * Manages baseline files: reading, writing, and listing.
 */
export class BaselineManager {
  private baselineDir: string;
  private writeLocks: Map<string, Promise<void>> = new Map();

  constructor(globalConfig: GlobalDiffConfig = {}) {
    this.baselineDir = globalConfig.dir || DEFAULT_BASELINE_DIR;
  }

  /**
   * Gets the baseline file path for a label.
   */
  getBaselinePath(yamlPath: string, label: string): string {
    const dir = path.dirname(yamlPath);
    const basename = path.basename(yamlPath, path.extname(yamlPath));
    return path.join(dir, this.baselineDir, `${basename}.${label}.baseline.json`);
  }

  /**
   * Gets the baseline directory for a YAML file.
   */
  getBaselineDir(yamlPath: string): string {
    return path.join(path.dirname(yamlPath), this.baselineDir);
  }

  /**
   * Loads baseline file for a specific label.
   */
  async load(yamlPath: string, label: string): Promise<BaselineFile | null> {
    const baselinePath = this.getBaselinePath(yamlPath, label);
    try {
      const file = Bun.file(baselinePath);
      if (!(await file.exists())) {
        return null;
      }
      const content = await file.text();
      return JSON.parse(content) as BaselineFile;
    } catch {
      return null;
    }
  }

  /**
   * Saves baseline file with write queue for parallel safety.
   */
  async save(yamlPath: string, label: string, data: BaselineFile): Promise<void> {
    const baselinePath = this.getBaselinePath(yamlPath, label);

    const existingLock = this.writeLocks.get(baselinePath);
    const writePromise = (async () => {
      if (existingLock) {
        await existingLock;
      }

      const dir = path.dirname(baselinePath);
      await fs.mkdir(dir, { recursive: true });

      const content = JSON.stringify(data, null, 2);
      await Bun.write(baselinePath, content);
    })();

    this.writeLocks.set(baselinePath, writePromise);
    await writePromise;
    this.writeLocks.delete(baselinePath);
  }

  /**
   * Gets a single baseline by request name.
   */
  async get(yamlPath: string, label: string, requestName: string): Promise<Baseline | null> {
    const file = await this.load(yamlPath, label);
    return file?.baselines[requestName] || null;
  }

  /**
   * Lists all available baseline labels for a YAML file.
   */
  async listLabels(yamlPath: string): Promise<string[]> {
    const dir = this.getBaselineDir(yamlPath);
    const basename = path.basename(yamlPath, path.extname(yamlPath));

    try {
      const files = await fs.readdir(dir);
      const labels: string[] = [];

      for (const file of files) {
        const match = file.match(new RegExp(`^${basename}\\.(.+)\\.baseline\\.json$`));
        if (match) {
          labels.push(match[1]);
        }
      }

      return labels.sort();
    } catch {
      return [];
    }
  }

  /**
   * Creates a baseline from execution result.
   */
  createBaseline(result: ExecutionResult, config: DiffConfig): Baseline {
    const baseline: Baseline = {
      hash: '',
      capturedAt: new Date().toISOString(),
    };

    if (result.status !== undefined) {
      baseline.status = result.status;
    }

    if (result.headers) {
      baseline.headers = this.normalizeHeaders(result.headers);
    }

    if (result.body !== undefined) {
      baseline.body = result.body;
    }

    if (config.includeTimings && result.metrics?.duration !== undefined) {
      baseline.timing = result.metrics.duration;
    }

    baseline.hash = this.hash(baseline);

    return baseline;
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
   * Generates a hash for baseline content.
   */
  hash(content: unknown): string {
    const str = JSON.stringify(content);
    const hasher = new Bun.CryptoHasher('md5');
    hasher.update(str);
    return hasher.digest('hex').slice(0, 8);
  }

  /**
   * Saves execution results as a baseline.
   */
  async saveBaseline(
    yamlPath: string,
    label: string,
    results: ExecutionResult[],
    config: DiffConfig,
  ): Promise<void> {
    const baselines: Record<string, Baseline> = {};

    for (const result of results) {
      if (result.skipped || !result.success) {
        continue;
      }
      const name = result.request.name || result.request.url;
      baselines[name] = this.createBaseline(result, config);
    }

    const file: BaselineFile = {
      version: BASELINE_VERSION,
      label,
      capturedAt: new Date().toISOString(),
      baselines,
    };

    await this.save(yamlPath, label, file);
  }

  /**
   * Deletes baselines older than specified days.
   */
  async cleanOldBaselines(yamlPath: string, olderThanDays: number): Promise<number> {
    const dir = this.getBaselineDir(yamlPath);
    const cutoff = Date.now() - olderThanDays * 24 * 60 * 60 * 1000;
    let deleted = 0;

    try {
      const files = await fs.readdir(dir);

      for (const file of files) {
        if (!file.endsWith('.baseline.json')) {
          continue;
        }

        const filePath = path.join(dir, file);
        const stat = await fs.stat(filePath);

        if (stat.mtimeMs < cutoff) {
          await fs.unlink(filePath);
          deleted++;
        }
      }
    } catch {
      // Directory doesn't exist or other error
    }

    return deleted;
  }

  /**
   * Merges request-level config with global config.
   */
  static mergeConfig(
    globalConfig: GlobalDiffConfig | undefined,
    requestConfig: DiffConfig | boolean | undefined,
  ): DiffConfig | null {
    if (!requestConfig && !globalConfig?.enabled) {
      return null;
    }

    if (requestConfig === true) {
      return {
        enabled: true,
        exclude: globalConfig?.exclude || [],
        match: globalConfig?.match || {},
        includeTimings: globalConfig?.includeTimings || false,
      };
    }

    if (typeof requestConfig === 'object' && requestConfig.enabled !== false) {
      return {
        enabled: true,
        exclude: [...(globalConfig?.exclude || []), ...(requestConfig.exclude || [])],
        match: { ...(globalConfig?.match || {}), ...(requestConfig.match || {}) },
        includeTimings: requestConfig.includeTimings ?? globalConfig?.includeTimings ?? false,
      };
    }

    if (globalConfig?.enabled && requestConfig === undefined) {
      return {
        enabled: true,
        exclude: globalConfig.exclude || [],
        match: globalConfig.match || {},
        includeTimings: globalConfig.includeTimings || false,
      };
    }

    return null;
  }
}
