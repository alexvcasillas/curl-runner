/**
 * .env file loader with environment-specific overrides.
 * Loads .env, then .env.{environment} (if specified).
 * Variables prefixed with SECRET_ are marked for redaction.
 */

import { join } from 'node:path';

export interface EnvFileResult {
  /** All loaded variables */
  variables: Record<string, string>;
  /** Keys that should be redacted in output (SECRET_* prefix) */
  secretKeys: string[];
}

/**
 * Parses .env file content into key-value pairs.
 * Handles quotes, multiline values, and comments.
 */
function parseEnvContent(content: string): Record<string, string> {
  const result: Record<string, string> = {};
  const lines = content.split('\n');

  for (const line of lines) {
    const trimmed = line.trim();

    // Skip empty lines and comments
    if (!trimmed || trimmed.startsWith('#')) {
      continue;
    }

    const eqIdx = trimmed.indexOf('=');
    if (eqIdx === -1) {
      continue;
    }

    const key = trimmed.slice(0, eqIdx).trim();
    let value = trimmed.slice(eqIdx + 1).trim();

    // Handle quoted values
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }

    // Handle escape sequences in double-quoted strings
    if (
      trimmed
        .slice(eqIdx + 1)
        .trim()
        .startsWith('"')
    ) {
      value = value
        .replace(/\\n/g, '\n')
        .replace(/\\r/g, '\r')
        .replace(/\\t/g, '\t')
        .replace(/\\\\/g, '\\');
    }

    if (key) {
      result[key] = value;
    }
  }

  return result;
}

/**
 * Loads .env file from given path.
 */
async function loadEnvFile(filePath: string): Promise<Record<string, string>> {
  const file = Bun.file(filePath);
  if (!(await file.exists())) {
    return {};
  }

  try {
    const content = await file.text();
    return parseEnvContent(content);
  } catch {
    return {};
  }
}

/**
 * Extracts secret keys (variables starting with SECRET_).
 */
function extractSecretKeys(variables: Record<string, string>): string[] {
  return Object.keys(variables).filter((key) => key.startsWith('SECRET_'));
}

/**
 * Loads environment variables from .env files.
 *
 * Priority (highest to lowest):
 * 1. .env.{environment}.local (if env specified)
 * 2. .env.{environment} (if env specified)
 * 3. .env.local
 * 4. .env
 *
 * @param cwd Working directory to search for .env files
 * @param environment Optional environment name (e.g., 'production', 'staging')
 */
export async function loadEnvFiles(
  cwd: string = process.cwd(),
  environment?: string,
): Promise<EnvFileResult> {
  const filesToLoad: string[] = [join(cwd, '.env'), join(cwd, '.env.local')];

  if (environment) {
    filesToLoad.push(join(cwd, `.env.${environment}`), join(cwd, `.env.${environment}.local`));
  }

  let variables: Record<string, string> = {};

  // Load files in order, later files override earlier ones
  for (const filePath of filesToLoad) {
    const fileVars = await loadEnvFile(filePath);
    variables = { ...variables, ...fileVars };
  }

  return {
    variables,
    secretKeys: extractSecretKeys(variables),
  };
}

/**
 * Gets list of .env files that exist and would be loaded.
 * Useful for logging/debugging.
 */
export async function getEnvFilePaths(cwd: string, environment?: string): Promise<string[]> {
  const paths = [join(cwd, '.env'), join(cwd, '.env.local')];

  if (environment) {
    paths.push(join(cwd, `.env.${environment}`), join(cwd, `.env.${environment}.local`));
  }

  const existing: string[] = [];
  for (const p of paths) {
    if (await Bun.file(p).exists()) {
      existing.push(p);
    }
  }
  return existing;
}
