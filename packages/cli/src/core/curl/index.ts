/**
 * Curl command building and response parsing module.
 * Shared utilities for constructing curl commands and parsing responses.
 */

// Args builder
export {
  buildCurlArgs,
  createBatchMarker,
  formatArgsForDisplay,
  METRICS_MARKER_END,
  METRICS_MARKER_START,
} from './args-builder';
// Response parser
export {
  extractBatchMetrics,
  extractMetricsFromOutput,
  getStatusCode,
  isSuccessStatus,
  parseHeadersFromStderr,
  parseMetrics,
} from './response-parser';
// Types
export {
  type CurlArgsOptions,
  type CurlArgsResult,
  type CurlExecutionResult,
  type CurlMetrics,
  isFileAttachment,
  type ProcessedMetrics,
} from './types';
