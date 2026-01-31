/**
 * Watch mode types.
 */

/**
 * Configuration for watch mode.
 * Watch mode automatically re-runs requests when YAML files change.
 */
export interface WatchConfig {
  /** Enable watch mode. Default: false */
  enabled?: boolean;
  /** Debounce delay in milliseconds. Default: 300 */
  debounce?: number;
  /** Clear screen between runs. Default: true */
  clear?: boolean;
}
