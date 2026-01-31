/**
 * Variable interpolation module.
 * Handles variable substitution in request configs.
 */

export type { InterpolateOptions, StoreContext, VariableRef, Variables } from './types';

export {
  extractVariables,
  formatDate,
  formatTime,
  interpolate,
  resolveDynamicVariable,
  resolveVariable,
} from './variable-resolver';
