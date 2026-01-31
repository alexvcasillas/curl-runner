import { interpolate } from '../core/interpolator';
import type { RequestConfig, ResponseStoreContext, YamlFile } from '../types/config';

/**
 * YAML file parser with variable interpolation support.
 * Uses core/interpolator for variable resolution.
 */
export class YamlParser {
  static async parseFile(filepath: string): Promise<YamlFile> {
    const file = Bun.file(filepath);
    const content = await file.text();
    return Bun.YAML.parse(content) as YamlFile;
  }

  static parse(content: string): YamlFile {
    return Bun.YAML.parse(content) as YamlFile;
  }

  /**
   * Interpolates variables in an object.
   * Delegates to core/interpolator for the actual work.
   */
  static interpolateVariables(
    obj: unknown,
    variables: Record<string, string>,
    storeContext?: ResponseStoreContext,
  ): unknown {
    return interpolate(obj, { variables, storeContext });
  }

  static mergeConfigs(base: Partial<RequestConfig>, override: RequestConfig): RequestConfig {
    return {
      ...base,
      ...override,
      headers: { ...base.headers, ...override.headers },
      params: { ...base.params, ...override.params },
      variables: { ...base.variables, ...override.variables },
    };
  }
}
