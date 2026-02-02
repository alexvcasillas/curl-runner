/**
 * Global and collection configuration types.
 */

import type { GlobalDiffConfig } from './diff';
import type { ProfileConfig } from './profile';
import type { RequestConfig, SSLConfig } from './request';
import type { GlobalSnapshotConfig } from './snapshot';
import type { WatchConfig } from './watch';

export interface CollectionConfig {
  name: string;
  description?: string;
  variables?: Record<string, string>;
  defaults?: Partial<RequestConfig>;
  requests: RequestConfig[];
}

/**
 * CI exit code configuration options.
 * These options control how curl-runner exits in CI/CD pipelines.
 */
export interface CIExitConfig {
  /**
   * When true, exit with code 1 if any validation failures occur,
   * regardless of the continueOnError setting.
   * This is useful for CI/CD pipelines that need strict validation.
   */
  strictExit?: boolean;
  /**
   * Maximum number of failures allowed before exiting with code 1.
   * If set to 0, any failure will cause a non-zero exit.
   * If undefined and strictExit is true, any failure causes non-zero exit.
   */
  failOn?: number;
  /**
   * Maximum percentage of failures allowed before exiting with code 1.
   * Value should be between 0 and 100.
   * If set to 10, up to 10% of requests can fail without causing a non-zero exit.
   */
  failOnPercentage?: number;
}

/**
 * Configuration for TCP connection pooling with HTTP/2 multiplexing.
 * Groups requests by host and executes them in batched curl processes
 * to reuse TCP/TLS connections and leverage HTTP/2 stream multiplexing.
 */
export interface ConnectionPoolConfig {
  /**
   * Enable connection pooling. When enabled, requests to the same host
   * are batched into a single curl process with HTTP/2 multiplexing.
   * Default: false
   */
  enabled?: boolean;
  /**
   * Maximum concurrent streams per host connection.
   * Maps to curl's --parallel-max flag.
   * Default: 10
   */
  maxStreamsPerHost?: number;
  /**
   * TCP keepalive time in seconds.
   * Maps to curl's --keepalive-time flag.
   * Default: 60
   */
  keepaliveTime?: number;
  /**
   * Connection timeout in seconds for establishing new connections.
   * Maps to curl's --connect-timeout flag.
   * Default: 30
   */
  connectTimeout?: number;
}

/**
 * Environment file loading configuration.
 */
export interface EnvConfig {
  /**
   * Environment name for loading .env.{environment} files.
   * E.g., 'production', 'staging', 'development'
   */
  environment?: string;
  /**
   * Redact secret values in output (variables starting with SECRET_).
   * Default: true
   */
  redactSecrets?: boolean;
}

export interface GlobalConfig {
  /**
   * Environment file configuration.
   * Controls .env file loading and secret redaction.
   */
  env?: EnvConfig;
  execution?: 'sequential' | 'parallel';
  /**
   * Maximum number of concurrent requests when using parallel execution.
   * If not set, all requests will execute simultaneously.
   * Useful for avoiding rate limiting or overwhelming target servers.
   */
  maxConcurrency?: number;
  continueOnError?: boolean;
  /**
   * Dry run mode: show curl commands without executing them.
   * Useful for previewing what commands would be run.
   */
  dryRun?: boolean;
  /**
   * Use HTTP/2 protocol with multiplexing support.
   * Passes --http2 flag to curl.
   */
  http2?: boolean;
  /**
   * TCP connection pooling configuration.
   * Enables request batching and HTTP/2 multiplexing for same-host requests.
   */
  connectionPool?: ConnectionPoolConfig;
  /**
   * CI/CD exit code configuration.
   * Controls when curl-runner should exit with non-zero status codes.
   */
  ci?: CIExitConfig;
  /**
   * Global SSL/TLS certificate configuration.
   * Applied to all requests unless overridden at the request level.
   */
  ssl?: SSLConfig;
  /**
   * Watch mode configuration.
   * Automatically re-runs requests when YAML files change.
   */
  watch?: WatchConfig;
  /**
   * Performance profiling mode configuration.
   * Runs requests multiple times to collect p50/p95/p99 latency stats.
   */
  profile?: ProfileConfig;
  /**
   * Snapshot testing configuration.
   * Saves response snapshots and compares future runs against them.
   */
  snapshot?: GlobalSnapshotConfig;
  /**
   * Response diffing configuration.
   * Compare responses between environments or runs to detect API drift.
   */
  diff?: GlobalDiffConfig;
  variables?: Record<string, string>;
  output?: {
    verbose?: boolean;
    showHeaders?: boolean;
    showBody?: boolean;
    showMetrics?: boolean;
    format?: 'json' | 'pretty' | 'raw';
    prettyLevel?: 'minimal' | 'standard' | 'detailed';
    saveToFile?: string;
    /** Internal flag set when dry-run mode is enabled to always show commands */
    dryRun?: boolean;
  };
  defaults?: Partial<RequestConfig>;
}

export interface YamlFile {
  version?: string;
  global?: GlobalConfig;
  collection?: CollectionConfig;
  requests?: RequestConfig[];
  request?: RequestConfig;
}
