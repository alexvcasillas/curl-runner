import type {
  Baseline,
  DiffCompareResult,
  DiffConfig,
  DiffSummary,
  ExecutionResult,
  GlobalDiffConfig,
  JsonValue,
  ResponseDiff,
} from '../types/config';
import { BaselineManager } from './baseline-manager';

/**
 * Compares responses against baselines with support for exclusions and match rules.
 */
export class ResponseDiffer {
  private excludePaths: Set<string>;
  private matchRules: Map<string, string>;
  private includeTimings: boolean;

  constructor(config: DiffConfig) {
    this.excludePaths = new Set(config.exclude || []);
    this.matchRules = new Map(Object.entries(config.match || {}));
    this.includeTimings = config.includeTimings || false;
  }

  /**
   * Compares current response against baseline.
   */
  compare(
    baseline: Baseline,
    current: Baseline,
    baselineLabel: string,
    currentLabel: string,
    requestName: string,
  ): DiffCompareResult {
    const differences: ResponseDiff[] = [];

    // Compare status
    if (baseline.status !== undefined || current.status !== undefined) {
      if (baseline.status !== current.status && !this.isExcluded('status')) {
        differences.push({
          path: 'status',
          baseline: baseline.status,
          current: current.status,
          type: 'changed',
        });
      }
    }

    // Compare headers
    if (baseline.headers || current.headers) {
      const headerDiffs = this.compareObjects(
        baseline.headers || {},
        current.headers || {},
        'headers',
      );
      differences.push(...headerDiffs);
    }

    // Compare body
    if (baseline.body !== undefined || current.body !== undefined) {
      const bodyDiffs = this.deepCompare(baseline.body, current.body, 'body');
      differences.push(...bodyDiffs);
    }

    const result: DiffCompareResult = {
      requestName,
      hasDifferences: differences.length > 0,
      isNewBaseline: false,
      baselineLabel,
      currentLabel,
      differences,
    };

    // Add timing diff if enabled
    if (this.includeTimings && baseline.timing !== undefined && current.timing !== undefined) {
      const changePercent = ((current.timing - baseline.timing) / baseline.timing) * 100;
      result.timingDiff = {
        baseline: baseline.timing,
        current: current.timing,
        changePercent,
      };
    }

    return result;
  }

  /**
   * Deep comparison of two values with path tracking.
   */
  deepCompare(baseline: unknown, current: unknown, path: string): ResponseDiff[] {
    if (this.isExcluded(path)) {
      return [];
    }

    if (this.matchesRule(path, current)) {
      return [];
    }

    if (baseline === null && current === null) {
      return [];
    }
    if (baseline === undefined && current === undefined) {
      return [];
    }

    const baselineType = this.getType(baseline);
    const currentType = this.getType(current);

    if (baselineType !== currentType) {
      return [
        {
          path,
          baseline,
          current,
          type: 'type_mismatch',
        },
      ];
    }

    if (baselineType !== 'object' && baselineType !== 'array') {
      if (baseline !== current) {
        return [
          {
            path,
            baseline,
            current,
            type: 'changed',
          },
        ];
      }
      return [];
    }

    if (baselineType === 'array') {
      return this.compareArrays(baseline as JsonValue[], current as JsonValue[], path);
    }

    return this.compareObjects(
      baseline as Record<string, unknown>,
      current as Record<string, unknown>,
      path,
    );
  }

  /**
   * Compares two arrays.
   */
  private compareArrays(baseline: JsonValue[], current: JsonValue[], path: string): ResponseDiff[] {
    const differences: ResponseDiff[] = [];
    const maxLen = Math.max(baseline.length, current.length);

    for (let i = 0; i < maxLen; i++) {
      const itemPath = `${path}[${i}]`;

      if (i >= baseline.length) {
        if (!this.isExcluded(itemPath)) {
          differences.push({
            path: itemPath,
            baseline: undefined,
            current: current[i],
            type: 'added',
          });
        }
      } else if (i >= current.length) {
        if (!this.isExcluded(itemPath)) {
          differences.push({
            path: itemPath,
            baseline: baseline[i],
            current: undefined,
            type: 'removed',
          });
        }
      } else {
        const itemDiffs = this.deepCompare(baseline[i], current[i], itemPath);
        differences.push(...itemDiffs);
      }
    }

    return differences;
  }

  /**
   * Compares two objects.
   */
  private compareObjects(
    baseline: Record<string, unknown>,
    current: Record<string, unknown>,
    path: string,
  ): ResponseDiff[] {
    const differences: ResponseDiff[] = [];
    const allKeys = new Set([...Object.keys(baseline), ...Object.keys(current)]);

    for (const key of allKeys) {
      const keyPath = path ? `${path}.${key}` : key;
      const hasBaseline = key in baseline;
      const hasCurrent = key in current;

      if (!hasBaseline && hasCurrent) {
        if (!this.isExcluded(keyPath)) {
          differences.push({
            path: keyPath,
            baseline: undefined,
            current: current[key],
            type: 'added',
          });
        }
      } else if (hasBaseline && !hasCurrent) {
        if (!this.isExcluded(keyPath)) {
          differences.push({
            path: keyPath,
            baseline: baseline[key],
            current: undefined,
            type: 'removed',
          });
        }
      } else {
        const keyDiffs = this.deepCompare(baseline[key], current[key], keyPath);
        differences.push(...keyDiffs);
      }
    }

    return differences;
  }

  /**
   * Checks if a path should be excluded from comparison.
   */
  isExcluded(path: string): boolean {
    if (this.excludePaths.has(path)) {
      return true;
    }

    for (const pattern of this.excludePaths) {
      if (pattern.startsWith('*.')) {
        const suffix = pattern.slice(2);
        if (path.endsWith(`.${suffix}`)) {
          return true;
        }
        const lastPart = path.split('.').pop();
        if (lastPart === suffix) {
          return true;
        }
      }

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

    if (rule === '*') {
      return true;
    }

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

/**
 * Orchestrates response diffing between runs.
 */
export class DiffOrchestrator {
  private manager: BaselineManager;

  constructor(globalConfig: GlobalDiffConfig = {}) {
    this.manager = new BaselineManager(globalConfig);
  }

  /**
   * Compares execution results against a baseline.
   */
  async compareWithBaseline(
    yamlPath: string,
    results: ExecutionResult[],
    currentLabel: string,
    baselineLabel: string,
    config: DiffConfig,
  ): Promise<DiffSummary> {
    const baselineFile = await this.manager.load(yamlPath, baselineLabel);
    const diffResults: DiffCompareResult[] = [];

    for (const result of results) {
      if (result.skipped || !result.success) {
        continue;
      }

      const requestName = result.request.name || result.request.url;
      const currentBaseline = this.manager.createBaseline(result, config);

      if (!baselineFile) {
        // No baseline exists - mark as new
        diffResults.push({
          requestName,
          hasDifferences: false,
          isNewBaseline: true,
          baselineLabel,
          currentLabel,
          differences: [],
        });
        continue;
      }

      const storedBaseline = baselineFile.baselines[requestName];

      if (!storedBaseline) {
        // Request not in baseline - mark as new
        diffResults.push({
          requestName,
          hasDifferences: false,
          isNewBaseline: true,
          baselineLabel,
          currentLabel,
          differences: [],
        });
        continue;
      }

      const differ = new ResponseDiffer(config);
      const compareResult = differ.compare(
        storedBaseline,
        currentBaseline,
        baselineLabel,
        currentLabel,
        requestName,
      );
      diffResults.push(compareResult);
    }

    return {
      totalRequests: diffResults.length,
      unchanged: diffResults.filter((r) => !r.hasDifferences && !r.isNewBaseline).length,
      changed: diffResults.filter((r) => r.hasDifferences).length,
      newBaselines: diffResults.filter((r) => r.isNewBaseline).length,
      results: diffResults,
    };
  }

  /**
   * Compares two stored baselines (offline comparison).
   */
  async compareTwoBaselines(
    yamlPath: string,
    label1: string,
    label2: string,
    config: DiffConfig,
  ): Promise<DiffSummary> {
    const file1 = await this.manager.load(yamlPath, label1);
    const file2 = await this.manager.load(yamlPath, label2);

    if (!file1) {
      throw new Error(`Baseline '${label1}' not found`);
    }
    if (!file2) {
      throw new Error(`Baseline '${label2}' not found`);
    }

    const allRequestNames = new Set([
      ...Object.keys(file1.baselines),
      ...Object.keys(file2.baselines),
    ]);

    const diffResults: DiffCompareResult[] = [];
    const differ = new ResponseDiffer(config);

    for (const requestName of allRequestNames) {
      const baseline1 = file1.baselines[requestName];
      const baseline2 = file2.baselines[requestName];

      if (!baseline1) {
        diffResults.push({
          requestName,
          hasDifferences: true,
          isNewBaseline: false,
          baselineLabel: label1,
          currentLabel: label2,
          differences: [
            {
              path: '',
              baseline: undefined,
              current: 'exists',
              type: 'added',
            },
          ],
        });
        continue;
      }

      if (!baseline2) {
        diffResults.push({
          requestName,
          hasDifferences: true,
          isNewBaseline: false,
          baselineLabel: label1,
          currentLabel: label2,
          differences: [
            {
              path: '',
              baseline: 'exists',
              current: undefined,
              type: 'removed',
            },
          ],
        });
        continue;
      }

      const compareResult = differ.compare(baseline1, baseline2, label1, label2, requestName);
      diffResults.push(compareResult);
    }

    return {
      totalRequests: diffResults.length,
      unchanged: diffResults.filter((r) => !r.hasDifferences).length,
      changed: diffResults.filter((r) => r.hasDifferences).length,
      newBaselines: 0,
      results: diffResults,
    };
  }

  /**
   * Saves current results as baseline.
   */
  async saveBaseline(
    yamlPath: string,
    label: string,
    results: ExecutionResult[],
    config: DiffConfig,
  ): Promise<void> {
    await this.manager.saveBaseline(yamlPath, label, results, config);
  }

  /**
   * Lists available baseline labels.
   */
  async listLabels(yamlPath: string): Promise<string[]> {
    return this.manager.listLabels(yamlPath);
  }

  /**
   * Gets the baseline manager instance.
   */
  getManager(): BaselineManager {
    return this.manager;
  }
}
