/**
 * Format module.
 * Provides output formatting utilities.
 */

export {
  ANSI_COLORS,
  colorize,
  formatDuration,
  formatJson,
  formatLatency,
  formatPercent,
  formatSize,
  getShortFilename,
  padEnd,
  padStart,
  repeatChar,
  truncate,
} from './formatters';
export type { AnsiColors, ColorName } from './types';
