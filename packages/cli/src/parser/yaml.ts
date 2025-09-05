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
      // Check if it's a single variable like ${VAR} (no other characters)
      const singleVarMatch = obj.match(/^\$\{([^}]+)\}$/);
      if (singleVarMatch) {
        const varName = singleVarMatch[1];
        const dynamicValue = YamlParser.resolveDynamicVariable(varName);
        return dynamicValue !== null ? dynamicValue : variables[varName] || obj;
      }

      // Handle multiple variables in the string using regex replacement
      return obj.replace(/\$\{([^}]+)\}/g, (match, varName) => {
        const dynamicValue = YamlParser.resolveDynamicVariable(varName);
        return dynamicValue !== null ? dynamicValue : variables[varName] || match;
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

  static resolveDynamicVariable(varName: string): string | null {
    // UUID generation
    if (varName === 'UUID') {
      return crypto.randomUUID();
    }

    // Current timestamp variations
    if (varName === 'CURRENT_TIME' || varName === 'TIMESTAMP') {
      return Date.now().toString();
    }

    // Date formatting - ${DATE:YYYY-MM-DD}
    if (varName.startsWith('DATE:')) {
      const format = varName.slice(5); // Remove 'DATE:'
      return YamlParser.formatDate(new Date(), format);
    }

    // Time formatting - ${TIME:HH:mm:ss}
    if (varName.startsWith('TIME:')) {
      const format = varName.slice(5); // Remove 'TIME:'
      return YamlParser.formatTime(new Date(), format);
    }

    return null; // Not a dynamic variable
  }

  static formatDate(date: Date, format: string): string {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');

    return format.replace('YYYY', year.toString()).replace('MM', month).replace('DD', day);
  }

  static formatTime(date: Date, format: string): string {
    const hours = String(date.getHours()).padStart(2, '0');
    const minutes = String(date.getMinutes()).padStart(2, '0');
    const seconds = String(date.getSeconds()).padStart(2, '0');

    return format.replace('HH', hours).replace('mm', minutes).replace('ss', seconds);
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
