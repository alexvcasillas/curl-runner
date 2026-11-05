import { strToU8, zipSync } from 'fflate';
import type { BundleFile } from './types';

/** A zip of a version's files, each at its own path. */
export function zipFiles(files: BundleFile[]): Uint8Array {
  const entries: Record<string, Uint8Array> = {};
  for (const file of files) {
    entries[file.path] = strToU8(file.text);
  }
  return zipSync(entries);
}
