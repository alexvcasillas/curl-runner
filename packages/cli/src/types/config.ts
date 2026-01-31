/**
 * Re-export all types from domain-specific modules.
 * This maintains backward compatibility for existing imports.
 */

// Diff/baseline types
export type {
  Baseline,
  BaselineFile,
  DiffCompareResult,
  DiffConfig,
  DiffSummary,
  GlobalDiffConfig,
  ResponseDiff,
} from './diff';
// Execution types
export type {
  ExecutionResult,
  ExecutionSummary,
  ProfileResult,
  ResponseStoreContext,
} from './execution';
// Global/Collection types
export type {
  CIExitConfig,
  CollectionConfig,
  ConnectionPoolConfig,
  GlobalConfig,
  YamlFile,
} from './global';
// JSON primitives
export type { JsonArray, JsonObject, JsonValue } from './json';
// Profile types
export type { ProfileConfig, ProfileStats } from './profile';

// Request types
export type {
  ConditionExpression,
  ConditionOperator,
  FileAttachment,
  FormDataConfig,
  FormFieldValue,
  RequestConfig,
  SSLConfig,
  StoreConfig,
  WhenCondition,
} from './request';
// Snapshot types
export type {
  GlobalSnapshotConfig,
  Snapshot,
  SnapshotCompareResult,
  SnapshotConfig,
  SnapshotDiff,
  SnapshotFile,
} from './snapshot';
// Watch types
export type { WatchConfig } from './watch';
