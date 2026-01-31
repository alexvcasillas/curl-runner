/**
 * Variable interpolation and resolution logic.
 * Supports static variables, dynamic variables, store context, defaults, and transforms.
 */

import type { InterpolateOptions, StoreContext, VariableRef, Variables } from './types';

/**
 * Reserved prefixes for dynamic variable generation.
 * These cannot be used as regular variable names with default values.
 */
const DYNAMIC_PREFIXES = ['DATE', 'TIME', 'UUID', 'RANDOM'] as const;

/**
 * Interpolates variables in an object recursively.
 *
 * Supports:
 * - Static variables: ${VAR_NAME}
 * - Dynamic variables: ${UUID}, ${TIMESTAMP}, ${DATE:format}, ${TIME:format}
 * - Stored response values: ${store.variableName}
 * - Default values: ${VAR_NAME:default}
 * - Nested defaults: ${VAR1:${VAR2:fallback}}
 * - String transforms: ${VAR:upper}, ${VAR:lower}
 */
export function interpolate(obj: unknown, options: InterpolateOptions = {}): unknown {
  const { variables = {}, storeContext, resolveEnv = true } = options;

  if (typeof obj === 'string') {
    return interpolateString(obj, variables, storeContext, resolveEnv);
  }

  if (Array.isArray(obj)) {
    return obj.map((item) => interpolate(item, options));
  }

  if (obj && typeof obj === 'object') {
    const result: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(obj)) {
      result[key] = interpolate(value, options);
    }
    return result;
  }

  return obj;
}

/**
 * Interpolates variables in a string.
 */
function interpolateString(
  str: string,
  variables: Variables,
  storeContext?: StoreContext,
  resolveEnv = true,
): unknown {
  const refs = extractVariables(str);

  if (refs.length === 0) {
    return str;
  }

  // Single variable spanning entire string - preserve type
  if (refs.length === 1 && refs[0].start === 0 && refs[0].end === str.length) {
    const resolved = resolveVariable(refs[0].name, variables, storeContext, resolveEnv);
    return resolved !== null ? resolved : str;
  }

  // Multiple variables or partial string - concatenate as string
  let result = '';
  let lastEnd = 0;

  for (const ref of refs) {
    result += str.slice(lastEnd, ref.start);
    const resolved = resolveVariable(ref.name, variables, storeContext, resolveEnv);
    result += resolved !== null ? resolved : str.slice(ref.start, ref.end);
    lastEnd = ref.end;
  }

  result += str.slice(lastEnd);
  return result;
}

/**
 * Resolves a single variable reference.
 *
 * Priority:
 * 1. Store context (${store.variableName})
 * 2. String transforms (${VAR:upper}, ${VAR:lower})
 * 3. Variables with defaults (${VAR:default})
 * 4. Dynamic variables (${UUID}, ${TIMESTAMP}, etc.)
 * 5. Static variables
 * 6. Environment variables (if enabled)
 */
export function resolveVariable(
  varName: string,
  variables: Variables,
  storeContext?: StoreContext,
  resolveEnv = true,
): string | null {
  // 1. Store context
  if (varName.startsWith('store.') && storeContext) {
    const storeKey = varName.slice(6);
    if (storeKey in storeContext) {
      return storeContext[storeKey];
    }
    return null;
  }

  // 2. String transforms
  const transformMatch = varName.match(/^([^:]+):(upper|lower)$/);
  if (transformMatch) {
    const [, baseVar, transform] = transformMatch;
    const baseValue = variables[baseVar] ?? (resolveEnv ? process.env[baseVar] : undefined);

    if (baseValue) {
      return transform === 'upper' ? baseValue.toUpperCase() : baseValue.toLowerCase();
    }
    return null;
  }

  // 3. Default value syntax
  const colonIdx = varName.indexOf(':');
  if (colonIdx !== -1) {
    const actualVar = varName.slice(0, colonIdx);
    const defaultVal = varName.slice(colonIdx + 1);

    // Skip if it's a dynamic prefix
    if (!DYNAMIC_PREFIXES.includes(actualVar as (typeof DYNAMIC_PREFIXES)[number])) {
      const resolved = resolveVariable(actualVar, variables, storeContext, resolveEnv);
      if (resolved !== null) {
        return resolved;
      }

      // Handle nested variable in default
      if (defaultVal.startsWith('${')) {
        const nestedVar = defaultVal.endsWith('}') ? defaultVal.slice(2, -1) : defaultVal.slice(2);
        const nestedResolved = resolveVariable(nestedVar, variables, storeContext, resolveEnv);
        return nestedResolved !== null ? nestedResolved : defaultVal;
      }

      return defaultVal;
    }
  }

  // 4. Dynamic variables
  const dynamicValue = resolveDynamicVariable(varName);
  if (dynamicValue !== null) {
    return dynamicValue;
  }

  // 5. Static variables
  if (varName in variables) {
    return variables[varName];
  }

  // 6. Environment variables
  if (resolveEnv && varName in process.env) {
    return process.env[varName]!;
  }

  return null;
}

/**
 * Resolves dynamic variable patterns.
 */
export function resolveDynamicVariable(varName: string): string | null {
  // UUID generation
  if (varName === 'UUID') {
    return crypto.randomUUID();
  }

  // UUID:short - first segment (8 chars)
  if (varName === 'UUID:short') {
    return crypto.randomUUID().split('-')[0];
  }

  // RANDOM:min-max
  const rangeMatch = varName.match(/^RANDOM:(\d+)-(\d+)$/);
  if (rangeMatch) {
    const min = Number(rangeMatch[1]);
    const max = Number(rangeMatch[2]);
    return String(Math.floor(Math.random() * (max - min + 1)) + min);
  }

  // RANDOM:string:length
  const stringMatch = varName.match(/^RANDOM:string:(\d+)$/);
  if (stringMatch) {
    const length = Number(stringMatch[1]);
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    return Array.from({ length }, () =>
      chars.charAt(Math.floor(Math.random() * chars.length)),
    ).join('');
  }

  // Timestamp
  if (varName === 'CURRENT_TIME' || varName === 'TIMESTAMP') {
    return Date.now().toString();
  }

  // DATE:format
  if (varName.startsWith('DATE:')) {
    return formatDate(new Date(), varName.slice(5));
  }

  // TIME:format
  if (varName.startsWith('TIME:')) {
    return formatTime(new Date(), varName.slice(5));
  }

  return null;
}

/**
 * Formats a date using a format string.
 * Supports: YYYY, MM, DD
 */
export function formatDate(date: Date, format: string): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');

  return format.replace('YYYY', year.toString()).replace('MM', month).replace('DD', day);
}

/**
 * Formats a time using a format string.
 * Supports: HH, mm, ss
 */
export function formatTime(date: Date, format: string): string {
  const hours = String(date.getHours()).padStart(2, '0');
  const minutes = String(date.getMinutes()).padStart(2, '0');
  const seconds = String(date.getSeconds()).padStart(2, '0');

  return format.replace('HH', hours).replace('mm', minutes).replace('ss', seconds);
}

/**
 * Extracts variable references from a string with proper brace matching.
 * Handles nested braces like ${VAR:${OTHER:default}}.
 */
export function extractVariables(str: string): VariableRef[] {
  const refs: VariableRef[] = [];
  let i = 0;

  while (i < str.length) {
    if (str[i] === '$' && str[i + 1] === '{') {
      const start = i;
      i += 2;
      let braceCount = 1;
      const nameStart = i;

      while (i < str.length && braceCount > 0) {
        if (str[i] === '{') {
          braceCount++;
        } else if (str[i] === '}') {
          braceCount--;
        }
        i++;
      }

      if (braceCount === 0) {
        refs.push({
          start,
          end: i,
          name: str.slice(nameStart, i - 1),
        });
      }
    } else {
      i++;
    }
  }

  return refs;
}
