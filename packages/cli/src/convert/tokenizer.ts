/**
 * Shell-safe tokenizer for curl commands.
 * Handles single quotes, double quotes, escapes, backslash line continuations.
 */

/** Tokenize a shell command string into an array of tokens. */
export function tokenize(input: string): string[] {
  const tokens: string[] = [];
  let current = '';
  let i = 0;
  const len = input.length;

  while (i < len) {
    const ch = input[i];

    // Backslash continuation (\ followed by newline)
    if (ch === '\\' && i + 1 < len && (input[i + 1] === '\n' || input[i + 1] === '\r')) {
      i += 2;
      // Skip \r\n
      if (i < len && input[i] === '\n') {
        i++;
      }
      // Skip leading whitespace on next line
      while (i < len && (input[i] === ' ' || input[i] === '\t')) {
        i++;
      }
      continue;
    }

    // Whitespace outside quotes = token boundary
    if (ch === ' ' || ch === '\t' || ch === '\n' || ch === '\r') {
      if (current.length > 0) {
        tokens.push(current);
        current = '';
      }
      i++;
      continue;
    }

    // Single-quoted string (no escape processing inside)
    if (ch === "'") {
      i++;
      while (i < len && input[i] !== "'") {
        current += input[i];
        i++;
      }
      if (i < len) {
        i++; // skip closing quote
      }
      continue;
    }

    // Double-quoted string (with escape processing)
    if (ch === '"') {
      i++;
      while (i < len && input[i] !== '"') {
        if (input[i] === '\\' && i + 1 < len) {
          const next = input[i + 1];
          // Only these chars are special in double quotes
          if (next === '"' || next === '\\' || next === '$' || next === '`' || next === '\n') {
            if (next === '\n') {
              // line continuation inside double quotes
              i += 2;
              continue;
            }
            current += next;
            i += 2;
            continue;
          }
        }
        current += input[i];
        i++;
      }
      if (i < len) {
        i++; // skip closing quote
      }
      continue;
    }

    // Backslash escape outside quotes
    if (ch === '\\' && i + 1 < len) {
      current += input[i + 1];
      i += 2;
      continue;
    }

    // Regular character
    current += ch;
    i++;
  }

  if (current.length > 0) {
    tokens.push(current);
  }

  return tokens;
}
