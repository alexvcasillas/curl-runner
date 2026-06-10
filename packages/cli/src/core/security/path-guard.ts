/**
 * Filesystem path confinement guard.
 * Pure, side-effect-free validation used at execution time.
 */

import { isAbsolute, resolve, sep } from 'node:path';

export interface PathValidationResult {
  valid: boolean;
  resolved?: string;
  error?: string;
}

/**
 * Resolves and validates a filesystem path against a base directory.
 * When allow is true, all paths are permitted (opt-out of confinement).
 * Otherwise, paths must resolve within baseDir (defaults to process.cwd()).
 */
export function resolveSafePath(
  inputPath: string,
  opts?: { baseDir?: string; allow?: boolean },
): PathValidationResult {
  if (opts?.allow) {
    return { valid: true, resolved: inputPath };
  }

  const baseDir = opts?.baseDir ?? process.cwd();
  // Normalise baseDir so the prefix check is consistent
  const normalizedBase = resolve(baseDir);

  let resolved: string;
  if (isAbsolute(inputPath)) {
    resolved = resolve(inputPath);
  } else {
    resolved = resolve(normalizedBase, inputPath);
  }

  // Accept exact match (baseDir itself) or paths strictly inside it
  const isInside =
    resolved === normalizedBase || resolved.startsWith(normalizedBase + sep);

  if (!isInside) {
    return {
      valid: false,
      error: `Path "${inputPath}" escapes the working directory. Use --allow-path to permit it.`,
    };
  }

  return { valid: true, resolved };
}
