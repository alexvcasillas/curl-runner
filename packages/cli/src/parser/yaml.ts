import { YAML } from 'bun';
import type { RequestConfig, ResponseStoreContext, YamlFile } from '../types/config';

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

  /**
   * Interpolates variables in an object, supporting:
   * - Static variables: ${VAR_NAME}
   * - Dynamic variables: ${UUID}, ${TIMESTAMP}, ${DATE:format}, ${TIME:format}
   * - Stored response values: ${store.variableName}
   *
   * @param obj - The object to interpolate
   * @param variables - Static variables map
   * @param storeContext - Optional stored response values from previous requests
   */
  static interpolateVariables(
    obj: unknown,
    variables: Record<string, string>,
    storeContext?: ResponseStoreContext,
  ): unknown {
    if (typeof obj === 'string') {
      // Check if it's a single variable like ${VAR} (no other characters)
      const singleVarMatch = obj.match(/^\$\{([^}]+)\}$/);
      if (singleVarMatch) {
        const varName = singleVarMatch[1];
        const resolvedValue = YamlParser.resolveVariable(varName, variables, storeContext);
        return resolvedValue !== null ? resolvedValue : obj;
      }

      // Handle multiple variables in the string using regex replacement
      return obj.replace(/\$\{([^}]+)\}/g, (match, varName) => {
        const resolvedValue = YamlParser.resolveVariable(varName, variables, storeContext);
        return resolvedValue !== null ? resolvedValue : match;
      });
    }

    if (Array.isArray(obj)) {
      return obj.map((item) => YamlParser.interpolateVariables(item, variables, storeContext));
    }

    if (obj && typeof obj === 'object') {
      const result: Record<string, unknown> = {};
      for (const [key, value] of Object.entries(obj)) {
        result[key] = YamlParser.interpolateVariables(value, variables, storeContext);
      }
      return result;
    }

    return obj;
  }

  /**
   * Resolves a single variable reference.
   * Priority: store context > dynamic variables > static variables
   */
  static resolveVariable(
    varName: string,
    variables: Record<string, string>,
    storeContext?: ResponseStoreContext,
  ): string | null {
    // Check for store variable (${store.variableName})
    if (varName.startsWith('store.') && storeContext) {
      const storeVarName = varName.slice(6); // Remove 'store.' prefix
      if (storeVarName in storeContext) {
        return storeContext[storeVarName];
      }
      return null; // Store variable not found, return null to keep original
    }

    // Check for dynamic variable
    const dynamicValue = YamlParser.resolveDynamicVariable(varName);
    if (dynamicValue !== null) {
      return dynamicValue;
    }

    // Check for static variable
    if (varName in variables) {
      return variables[varName];
    }

    return null;
  }

  static resolveDynamicVariable(varName: string): string | null {
    // UUID generation
    if (varName === 'UUID') {
      return crypto.randomUUID();
    }

    // UUID:short - first segment (8 chars) of a UUID
    if (varName === 'UUID:short') {
      return crypto.randomUUID().split('-')[0];
    }

    // RANDOM:min-max - random number in range
    const randomRangeMatch = varName.match(/^RANDOM:(\d+)-(\d+)$/);
    if (randomRangeMatch) {
      const min = Number(randomRangeMatch[1]);
      const max = Number(randomRangeMatch[2]);
      return String(Math.floor(Math.random() * (max - min + 1)) + min);
    }

    // RANDOM:string:length - random alphanumeric string
    const randomStringMatch = varName.match(/^RANDOM:string:(\d+)$/);
    if (randomStringMatch) {
      const length = Number(randomStringMatch[1]);
      const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
      return Array.from({ length }, () => chars.charAt(Math.floor(Math.random() * chars.length))).join('');
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
