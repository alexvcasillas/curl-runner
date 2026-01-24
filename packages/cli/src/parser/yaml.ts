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
   * - Default values: ${VAR_NAME:default} - uses 'default' if VAR_NAME is not found
   * - Nested defaults: ${VAR1:${VAR2:fallback}} - tries VAR1, then VAR2, then 'fallback'
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
      // Extract all variable references with proper brace matching
      const extractedVars = YamlParser.extractVariables(obj);

      if (extractedVars.length === 0) {
        return obj;
      }

      // Check if it's a single variable that spans the entire string
      if (extractedVars.length === 1 && extractedVars[0].start === 0 && extractedVars[0].end === obj.length) {
        const varName = extractedVars[0].name;
        const resolvedValue = YamlParser.resolveVariable(varName, variables, storeContext);
        return resolvedValue !== null ? resolvedValue : obj;
      }

      // Handle multiple variables in the string
      let result = '';
      let lastEnd = 0;
      for (const varRef of extractedVars) {
        result += obj.slice(lastEnd, varRef.start);
        const resolvedValue = YamlParser.resolveVariable(varRef.name, variables, storeContext);
        result += resolvedValue !== null ? resolvedValue : obj.slice(varRef.start, varRef.end);
        lastEnd = varRef.end;
      }
      result += obj.slice(lastEnd);
      return result;
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
   * Priority: store context > dynamic variables > static variables > default values
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

    // Check for default value syntax: ${VAR:default}
    // Must check before dynamic variables to properly handle defaults
    const colonIndex = varName.indexOf(':');
    if (colonIndex !== -1) {
      const actualVarName = varName.slice(0, colonIndex);
      const defaultValue = varName.slice(colonIndex + 1);

      // Don't confuse with DATE:, TIME:, UUID:, RANDOM: patterns
      // These are reserved prefixes for dynamic variable generation
      const reservedPrefixes = ['DATE', 'TIME', 'UUID', 'RANDOM'];
      if (!reservedPrefixes.includes(actualVarName)) {
        // Try to resolve the actual variable name
        const resolved = YamlParser.resolveVariable(actualVarName, variables, storeContext);
        if (resolved !== null) {
          return resolved;
        }
        // Variable not found, use the default value
        // The default value might itself be a variable reference like ${OTHER_VAR:fallback}
        // Note: Due to the regex in interpolateVariables using [^}]+, nested braces
        // get truncated (e.g., "${VAR:${OTHER:default}}" captures "VAR:${OTHER:default")
        // So we check for both complete ${...} and truncated ${... patterns
        if (defaultValue.startsWith('${')) {
          // Handle both complete ${VAR} and truncated ${VAR (from nested braces)
          const nestedVarName = defaultValue.endsWith('}')
            ? defaultValue.slice(2, -1)
            : defaultValue.slice(2);
          const nestedResolved = YamlParser.resolveVariable(nestedVarName, variables, storeContext);
          return nestedResolved !== null ? nestedResolved : defaultValue;
        }
        return defaultValue;
      }
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
      return Array.from({ length }, () =>
        chars.charAt(Math.floor(Math.random() * chars.length)),
      ).join('');
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

  /**
   * Extracts variable references from a string, properly handling nested braces.
   * For example, "${VAR:${OTHER:default}}" is extracted as a single variable reference.
   */
  static extractVariables(str: string): Array<{ start: number; end: number; name: string }> {
    const variables: Array<{ start: number; end: number; name: string }> = [];
    let i = 0;

    while (i < str.length) {
      // Look for ${
      if (str[i] === '$' && str[i + 1] === '{') {
        const start = i;
        i += 2; // Skip past ${
        let braceCount = 1;
        const nameStart = i;

        // Find the matching closing brace
        while (i < str.length && braceCount > 0) {
          if (str[i] === '{') {
            braceCount++;
          } else if (str[i] === '}') {
            braceCount--;
          }
          i++;
        }

        if (braceCount === 0) {
          // Found matching closing brace
          const name = str.slice(nameStart, i - 1); // Exclude the closing }
          variables.push({ start, end: i, name });
        }
        // If braceCount > 0, we have unmatched braces - skip this variable
      } else {
        i++;
      }
    }

    return variables;
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
