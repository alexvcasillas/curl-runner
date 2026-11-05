import { strToU8, zipSync } from 'fflate';
import type { BundleFile } from './types';

/** An entry path that could land outside the folder it is unzipped into. */
export class UnsafePathError extends Error {
  constructor(readonly path: string) {
    super(`Unsafe path in bundle: ${JSON.stringify(path)}`);
    this.name = 'UnsafePathError';
  }
}

/** Relative, forward slashes only, no `..`, no NUL, not empty. */
export function isSafeEntryPath(path: string): boolean {
  if (path === '' || path.startsWith('/') || path.includes('\\') || path.includes('\0')) {
    return false;
  }
  return !path.split('/').some((segment) => segment === '..');
}

/**
 * A zip of a version's files, each at its own path. Builds the whole archive in
 * memory with `zipSync`, which is fine: the server caps a bundle at 25 MiB of
 * decoded files, so there is nothing to stream. A path that could escape the
 * folder it is unzipped into fails the whole download, never a partial zip.
 */
export function zipFiles(files: BundleFile[]): Uint8Array {
  const entries: Record<string, Uint8Array> = {};
  for (const file of files) {
    if (!isSafeEntryPath(file.path)) {
      throw new UnsafePathError(file.path);
    }
    entries[file.path] = strToU8(file.text);
  }
  return zipSync(entries);
}
