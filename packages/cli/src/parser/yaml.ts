import { YAML } from 'bun';
import type { RequestConfig, YamlFile } from '../types/config';

// Using class for organization, but could be refactored to functions
export class YamlParser {
  static async parseFile(filepath: string): Promise<YamlFile> {
    const file = Bun.file(filepath);
    const content = await file.text();
    return YAML.parse(content) as YamlFile;
  }

  static parse(content: string): YamlFile {
    return YAML.parse(content) as YamlFile;
  }

  static interpolateVariables(obj: unknown, variables: Record<string, string>): unknown {
    if (typeof obj === 'string') {
      if (obj.startsWith('${') && obj.endsWith('}')) {
        const varName = obj.slice(2, -1);
        return variables[varName] || obj;
      }
      return obj.replace(/\$\{([^}]+)\}/g, (match, varName) => {
        return variables[varName] || match;
      });
    }

    if (Array.isArray(obj)) {
      return obj.map((item) => YamlParser.interpolateVariables(item, variables));
    }

    if (obj && typeof obj === 'object') {
      const result: Record<string, unknown> = {};
      for (const [key, value] of Object.entries(obj)) {
        result[key] = YamlParser.interpolateVariables(value, variables);
      }
      return result;
    }

    return obj;
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
