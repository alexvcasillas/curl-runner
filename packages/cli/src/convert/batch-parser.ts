/**
 * Batch script parser: extracts curl commands from shell scripts.
 * Uses heuristic detection to find curl invocations.
 */

/**
 * Extract curl command strings from a shell script.
 * Handles multiline commands (backslash continuations) and basic pipe chains.
 */
export function extractCurlCommands(script: string): string[] {
  const commands: string[] = [];
  const lines = script.split('\n');
  let i = 0;

  while (i < lines.length) {
    const trimmed = lines[i].trim();

    // Skip empty, comments, shebangs
    if (!trimmed || trimmed.startsWith('#') || trimmed.startsWith('//')) {
      i++;
      continue;
    }

    // Detect line starting with curl or containing curl invocation
    const curlStart = findCurlStart(trimmed);
    if (curlStart >= 0) {
      let cmd = trimmed.slice(curlStart);

      // Collect continuation lines
      while (cmd.endsWith('\\') && i + 1 < lines.length) {
        cmd = cmd.slice(0, -1); // remove trailing backslash
        i++;
        cmd += ' ' + lines[i].trim();
      }

      // Strip trailing pipe and everything after (e.g., | jq .)
      const pipeIdx = findUnquotedPipe(cmd);
      if (pipeIdx > 0) {
        cmd = cmd.slice(0, pipeIdx).trim();
      }

      // Strip trailing semicolons
      cmd = cmd.replace(/;\s*$/, '').trim();

      if (cmd.length > 4) {
        commands.push(cmd);
      }
    }

    i++;
  }

  return commands;
}

/**
 * Find the index of 'curl ' (or 'curl\t') at the start of a line or after
 * common shell constructs like $(), ``, &&, ||, ;, etc.
 */
function findCurlStart(line: string): number {
  // Direct curl command
  if (line.startsWith('curl ') || line === 'curl') return 0;

  // After shell operators: &&, ||, ;, |, $(, `
  const patterns = [
    /(?:^|&&\s*|;\s*|\|\|\s*|\$\(\s*|`\s*)curl\s/,
  ];

  for (const p of patterns) {
    const match = p.exec(line);
    if (match) {
      const idx = line.indexOf('curl', match.index);
      if (idx >= 0) return idx;
    }
  }

  return -1;
}

/**
 * Find unquoted pipe character for stripping piped output.
 */
function findUnquotedPipe(s: string): number {
  let inSingle = false;
  let inDouble = false;

  for (let i = 0; i < s.length; i++) {
    const ch = s[i];

    if (ch === '\\' && !inSingle) {
      i++; // skip escaped char
      continue;
    }

    if (ch === "'" && !inDouble) {
      inSingle = !inSingle;
      continue;
    }

    if (ch === '"' && !inSingle) {
      inDouble = !inDouble;
      continue;
    }

    if (ch === '|' && !inSingle && !inDouble) {
      return i;
    }
  }

  return -1;
}
