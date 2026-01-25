import type {
  ConditionExpression,
  ConditionOperator,
  ResponseStoreContext,
  WhenCondition,
} from '../types/config';
import { getValueByPath, valueToString } from './response-store';

/**
 * Result of evaluating a condition.
 */
export interface ConditionResult {
  shouldRun: boolean;
  reason?: string;
}

/**
 * Parses a string shorthand condition into a ConditionExpression.
 * Supports formats like:
 * - "store.status == 200"
 * - "store.userId exists"
 * - "store.body.type contains user"
 * - "store.version >= 2"
 *
 * @param condition - String condition to parse
 * @returns Parsed ConditionExpression or null if invalid
 */
export function parseStringCondition(condition: string): ConditionExpression | null {
  const trimmed = condition.trim();

  // Check for exists/not-exists operators first (unary)
  const existsMatch = trimmed.match(/^(.+?)\s+(exists|not-exists)$/i);
  if (existsMatch) {
    return {
      left: existsMatch[1].trim(),
      operator: existsMatch[2].toLowerCase() as ConditionOperator,
    };
  }

  // Binary operators pattern
  const operatorPattern = /^(.+?)\s*(==|!=|>=|<=|>|<|contains|matches)\s*(.+)$/i;
  const match = trimmed.match(operatorPattern);

  if (!match) {
    return null;
  }

  const [, left, operator, right] = match;
  const rightValue = parseRightValue(right.trim());

  return {
    left: left.trim(),
    operator: operator.toLowerCase() as ConditionOperator,
    right: rightValue,
  };
}

/**
 * Parses the right-hand value, converting to appropriate type.
 */
function parseRightValue(value: string): string | number | boolean {
  // Boolean
  if (value === 'true') return true;
  if (value === 'false') return false;

  // Number
  const num = Number(value);
  if (!Number.isNaN(num) && value !== '') return num;

  // String - remove quotes if present
  if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
    return value.slice(1, -1);
  }

  return value;
}

/**
 * Gets a value from the store context using a path.
 * Handles "store.x" prefix by stripping it.
 */
function getStoreValue(path: string, context: ResponseStoreContext): unknown {
  // Strip "store." prefix if present
  const normalizedPath = path.startsWith('store.') ? path.slice(6) : path;

  // First check if it's a direct key in context
  if (normalizedPath in context) {
    const value = context[normalizedPath];
    // Try to parse JSON if it looks like an object/array
    if (value.startsWith('{') || value.startsWith('[')) {
      try {
        return JSON.parse(value);
      } catch {
        return value;
      }
    }
    // Try to parse as number
    const num = Number(value);
    if (!Number.isNaN(num) && value !== '') return num;
    // Try to parse as boolean
    if (value === 'true') return true;
    if (value === 'false') return false;
    return value;
  }

  // Handle nested paths like "body.data.id" where context has "body" as JSON string
  const parts = normalizedPath.split('.');
  const rootKey = parts[0];

  if (rootKey in context) {
    const rootValue = context[rootKey];
    // Try to parse as JSON and navigate
    try {
      const parsed = JSON.parse(rootValue);
      return getValueByPath(parsed, parts.slice(1).join('.'));
    } catch {
      // Not JSON, can't navigate further
      return undefined;
    }
  }

  return undefined;
}

/**
 * Compares two values based on case sensitivity setting.
 */
function compareStrings(a: string, b: string, caseSensitive: boolean): boolean {
  if (caseSensitive) {
    return a === b;
  }
  return a.toLowerCase() === b.toLowerCase();
}

/**
 * Evaluates a single condition expression against the store context.
 */
export function evaluateExpression(
  expr: ConditionExpression,
  context: ResponseStoreContext,
): { passed: boolean; description: string } {
  const leftValue = getStoreValue(expr.left, context);
  const caseSensitive = expr.caseSensitive ?? false;

  const formatValue = (v: unknown): string => {
    if (v === undefined) return 'undefined';
    if (v === null) return 'null';
    if (typeof v === 'string') return `"${v}"`;
    return String(v);
  };

  const description = `${expr.left} ${expr.operator}${expr.right !== undefined ? ` ${formatValue(expr.right)}` : ''}`;

  switch (expr.operator) {
    case 'exists': {
      const passed = leftValue !== undefined && leftValue !== null && leftValue !== '';
      return { passed, description };
    }

    case 'not-exists': {
      const passed = leftValue === undefined || leftValue === null || leftValue === '';
      return { passed, description };
    }

    case '==': {
      let passed: boolean;
      if (typeof leftValue === 'string' && typeof expr.right === 'string') {
        passed = compareStrings(leftValue, expr.right, caseSensitive);
      } else {
        // biome-ignore lint/suspicious/noDoubleEquals: intentional loose equality for type coercion
        passed = leftValue == expr.right;
      }
      return { passed, description };
    }

    case '!=': {
      let passed: boolean;
      if (typeof leftValue === 'string' && typeof expr.right === 'string') {
        passed = !compareStrings(leftValue, expr.right, caseSensitive);
      } else {
        // biome-ignore lint/suspicious/noDoubleEquals: intentional loose equality for type coercion
        passed = leftValue != expr.right;
      }
      return { passed, description };
    }

    case '>': {
      const passed = Number(leftValue) > Number(expr.right);
      return { passed, description };
    }

    case '<': {
      const passed = Number(leftValue) < Number(expr.right);
      return { passed, description };
    }

    case '>=': {
      const passed = Number(leftValue) >= Number(expr.right);
      return { passed, description };
    }

    case '<=': {
      const passed = Number(leftValue) <= Number(expr.right);
      return { passed, description };
    }

    case 'contains': {
      const leftStr = valueToString(leftValue);
      const rightStr = String(expr.right ?? '');
      const passed = caseSensitive
        ? leftStr.includes(rightStr)
        : leftStr.toLowerCase().includes(rightStr.toLowerCase());
      return { passed, description };
    }

    case 'matches': {
      const leftStr = valueToString(leftValue);
      const pattern = String(expr.right ?? '');
      try {
        const flags = caseSensitive ? '' : 'i';
        const regex = new RegExp(pattern, flags);
        const passed = regex.test(leftStr);
        return { passed, description };
      } catch {
        // Invalid regex pattern
        return { passed: false, description: `${description} (invalid regex)` };
      }
    }

    default:
      return { passed: false, description: `unknown operator: ${expr.operator}` };
  }
}

/**
 * Evaluates a WhenCondition against the store context.
 *
 * @param condition - The condition to evaluate
 * @param context - The store context with values from previous requests
 * @returns Result indicating whether the request should run
 */
export function evaluateCondition(
  condition: WhenCondition | string,
  context: ResponseStoreContext,
): ConditionResult {
  // Handle string shorthand
  if (typeof condition === 'string') {
    const parsed = parseStringCondition(condition);
    if (!parsed) {
      return { shouldRun: false, reason: `invalid condition syntax: "${condition}"` };
    }
    const result = evaluateExpression(parsed, context);
    return {
      shouldRun: result.passed,
      reason: result.passed ? undefined : `condition not met: ${result.description}`,
    };
  }

  // Handle compound "all" condition (AND logic with short-circuit)
  if (condition.all && condition.all.length > 0) {
    for (const expr of condition.all) {
      const result = evaluateExpression(expr, context);
      if (!result.passed) {
        return {
          shouldRun: false,
          reason: `condition not met: ${result.description}`,
        };
      }
    }
    return { shouldRun: true };
  }

  // Handle compound "any" condition (OR logic with short-circuit)
  if (condition.any && condition.any.length > 0) {
    const descriptions: string[] = [];
    for (const expr of condition.any) {
      const result = evaluateExpression(expr, context);
      if (result.passed) {
        return { shouldRun: true };
      }
      descriptions.push(result.description);
    }
    return {
      shouldRun: false,
      reason: `no conditions met: ${descriptions.join(', ')}`,
    };
  }

  // Handle single condition (inline left/operator/right)
  if (condition.left && condition.operator) {
    const expr: ConditionExpression = {
      left: condition.left,
      operator: condition.operator,
      right: condition.right,
      caseSensitive: condition.caseSensitive,
    };
    const result = evaluateExpression(expr, context);
    return {
      shouldRun: result.passed,
      reason: result.passed ? undefined : `condition not met: ${result.description}`,
    };
  }

  // No valid condition found - default to run
  return { shouldRun: true };
}
