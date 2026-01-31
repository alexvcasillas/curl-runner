/**
 * Output formatting utilities.
 */

import type { AnsiColors, ColorName } from './types';

/**
 * ANSI color codes for terminal output.
 */
export const ANSI_COLORS: AnsiColors = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  dim: '\x1b[2m',
  underscore: '\x1b[4m',
  black: '\x1b[30m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
  white: '\x1b[37m',
  bgBlack: '\x1b[40m',
  bgRed: '\x1b[41m',
  bgGreen: '\x1b[42m',
  bgYellow: '\x1b[43m',
  bgBlue: '\x1b[44m',
  bgMagenta: '\x1b[45m',
  bgCyan: '\x1b[46m',
  bgWhite: '\x1b[47m',
};

/**
 * Wraps text with ANSI color codes.
 */
export function colorize(text: string, color: ColorName): string {
  return `${ANSI_COLORS[color]}${text}${ANSI_COLORS.reset}`;
}

/**
 * Formats duration in milliseconds to human-readable string.
 */
export function formatDuration(ms: number): string {
  if (ms < 1000) {
    return `${ms.toFixed(0)}ms`;
  }
  return `${(ms / 1000).toFixed(2)}s`;
}

/**
 * Formats latency with microsecond precision.
 */
export function formatLatency(ms: number): string {
  if (ms < 1) {
    return `${(ms * 1000).toFixed(0)}µs`;
  }
  if (ms < 1000) {
    return `${ms.toFixed(1)}ms`;
  }
  return `${(ms / 1000).toFixed(2)}s`;
}

/**
 * Formats byte size to human-readable string.
 */
export function formatSize(bytes: number | undefined): string {
  if (!bytes) {
    return '0 B';
  }
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / 1024 ** i).toFixed(2)} ${sizes[i]}`;
}

/**
 * Formats JSON data based on format type.
 */
export function formatJson(data: unknown, format: 'raw' | 'json' | 'pretty' = 'pretty'): string {
  if (format === 'raw') {
    return typeof data === 'string' ? data : JSON.stringify(data);
  }
  if (format === 'json') {
    return JSON.stringify(data);
  }
  return JSON.stringify(data, null, 2);
}

/**
 * Truncates a string to max length with ellipsis.
 */
export function truncate(str: string, maxLength: number): string {
  if (str.length <= maxLength) {
    return str;
  }
  return `${str.slice(0, maxLength - 3)}...`;
}

/**
 * Extracts short filename from path.
 */
export function getShortFilename(filePath: string): string {
  return filePath.replace(/.*\//, '').replace('.yaml', '');
}

/**
 * Formats a percentage value.
 */
export function formatPercent(value: number, decimals: number = 1): string {
  return `${(value * 100).toFixed(decimals)}%`;
}

/**
 * Pads string to fixed width.
 */
export function padStart(str: string, width: number): string {
  return str.padStart(width);
}

/**
 * Pads string to fixed width (end).
 */
export function padEnd(str: string, width: number): string {
  return str.padEnd(width);
}

/**
 * Creates a repeated character line.
 */
export function repeatChar(char: string, count: number): string {
  return char.repeat(count);
}
