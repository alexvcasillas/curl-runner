/**
 * Response post-processing: validation and snapshot testing.
 */

import type { SnapshotManager } from '../../snapshot/snapshot-manager';
import type { ExecutionResult, RequestConfig, SnapshotConfig } from '../../types/config';
import { validateResponse } from '../validation';

export interface PostProcessOptions {
  snapshotManager?: SnapshotManager;
  snapshotConfig?: SnapshotConfig | null;
}

/**
 * Applies validation and snapshot testing to an execution result.
 * Mutates the result in place and returns it for chaining.
 */
export async function postProcessResult(
  result: ExecutionResult,
  config: RequestConfig,
  options: PostProcessOptions = {},
): Promise<ExecutionResult> {
  // Apply validation if configured
  if (result.success && config.expect) {
    const validationResult = validateResponse(result, config.expect);
    if (!validationResult.success) {
      result.success = false;
      result.error = validationResult.error;
    }
  }

  // Snapshot testing
  const { snapshotManager, snapshotConfig } = options;
  if (snapshotManager && snapshotConfig && config.sourceFile && result.success) {
    const snapshotResult = await snapshotManager.compareAndUpdate(
      config.sourceFile,
      config.name || 'Request',
      result,
      snapshotConfig,
    );
    result.snapshotResult = snapshotResult;

    if (!snapshotResult.match && !snapshotResult.updated) {
      result.success = false;
      if (!result.error) {
        result.error = 'Snapshot mismatch';
      }
    }
  }

  return result;
}
