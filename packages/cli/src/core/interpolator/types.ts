/**
 * Types for variable interpolation system.
 */

/**
 * Context from stored response values.
 */
export type StoreContext = Record<string, string>;

/**
 * Static variables map.
 */
export type Variables = Record<string, string>;

/**
 * Extracted variable reference from a string.
 */
export interface VariableRef {
  /** Start index in the original string */
  start: number;
  /** End index in the original string (exclusive) */
  end: number;
  /** Variable name (content between ${ and }) */
  name: string;
}

/**
 * Options for interpolation.
 */
export interface InterpolateOptions {
  /** Static variables */
  variables?: Variables;
  /** Stored response values from previous requests */
  storeContext?: StoreContext;
  /** Whether to resolve environment variables (default: true) */
  resolveEnv?: boolean;
}
