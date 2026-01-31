/**
 * Types for output formatting.
 */

export interface AnsiColors {
  reset: string;
  bright: string;
  dim: string;
  underscore: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  bgBlack: string;
  bgRed: string;
  bgGreen: string;
  bgYellow: string;
  bgBlue: string;
  bgMagenta: string;
  bgCyan: string;
  bgWhite: string;
}

export type ColorName = keyof AnsiColors;
