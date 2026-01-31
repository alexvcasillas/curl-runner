/**
 * Configuration file loader for curl-runner.
 * Discovers and loads configuration from standard config file paths.
 */

import { YamlParser } from '../../parser/yaml';
import type { GlobalConfig } from '../../types/config';

/**
 * Standard configuration file paths searched in order.
 */
export const CONFIG_FILE_PATHS = [
  'curl-runner.yaml',
  'curl-runner.yml',
  '.curl-runner.yaml',
  '.curl-runner.yml',
] as const;

export interface ConfigFileResult {
  config: Partial<GlobalConfig>;
  path: string | null;
}

/**
 * Loads configuration from the first found config file.
 * Searches standard paths in order and returns the first valid config.
 *
 * @param logger Optional logger for info/warning messages
 * @returns Configuration object and path (or null if not found)
 */
export async function loadConfigFile(
  onInfo?: (msg: string) => void,
  onWarning?: (msg: string) => void,
): Promise<ConfigFileResult> {
  for (const filename of CONFIG_FILE_PATHS) {
    try {
      const file = Bun.file(filename);
      if (await file.exists()) {
        const yamlContent = await YamlParser.parseFile(filename);
        const config = yamlContent.global || {};
        onInfo?.(`Loaded configuration from ${filename}`);
        return { config: config as Partial<GlobalConfig>, path: filename };
      }
    } catch (error) {
      onWarning?.(`Failed to load configuration from ${filename}: ${error}`);
    }
  }

  return { config: {}, path: null };
}
