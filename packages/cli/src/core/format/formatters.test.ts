import { describe, expect, test } from 'bun:test';
import {
  ANSI_COLORS,
  colorize,
  formatDuration,
  formatJson,
  formatLatency,
  formatPercent,
  formatSize,
  getShortFilename,
  padEnd,
  padStart,
  repeatChar,
  truncate,
} from './formatters';

describe('ANSI_COLORS', () => {
  test('has reset code', () => {
    expect(ANSI_COLORS.reset).toBe('\x1b[0m');
  });

  test('has standard colors', () => {
    expect(ANSI_COLORS.red).toBe('\x1b[31m');
    expect(ANSI_COLORS.green).toBe('\x1b[32m');
    expect(ANSI_COLORS.blue).toBe('\x1b[34m');
  });

  test('has background colors', () => {
    expect(ANSI_COLORS.bgRed).toBe('\x1b[41m');
    expect(ANSI_COLORS.bgGreen).toBe('\x1b[42m');
  });
});

describe('colorize', () => {
  test('wraps text with color codes', () => {
    const result = colorize('hello', 'red');
    expect(result).toBe('\x1b[31mhello\x1b[0m');
  });

  test('handles different colors', () => {
    expect(colorize('test', 'green')).toContain('\x1b[32m');
    expect(colorize('test', 'blue')).toContain('\x1b[34m');
    expect(colorize('test', 'yellow')).toContain('\x1b[33m');
  });
});

describe('formatDuration', () => {
  test('formats milliseconds', () => {
    expect(formatDuration(100)).toBe('100ms');
    expect(formatDuration(500)).toBe('500ms');
    expect(formatDuration(999)).toBe('999ms');
  });

  test('formats seconds', () => {
    expect(formatDuration(1000)).toBe('1.00s');
    expect(formatDuration(2500)).toBe('2.50s');
    expect(formatDuration(10000)).toBe('10.00s');
  });

  test('handles zero', () => {
    expect(formatDuration(0)).toBe('0ms');
  });

  test('handles fractional milliseconds', () => {
    expect(formatDuration(50.7)).toBe('51ms');
  });
});

describe('formatLatency', () => {
  test('formats microseconds for sub-millisecond values', () => {
    expect(formatLatency(0.5)).toBe('500µs');
    expect(formatLatency(0.1)).toBe('100µs');
  });

  test('formats milliseconds', () => {
    expect(formatLatency(1)).toBe('1.0ms');
    expect(formatLatency(50.5)).toBe('50.5ms');
    expect(formatLatency(999)).toBe('999.0ms');
  });

  test('formats seconds for large values', () => {
    expect(formatLatency(1000)).toBe('1.00s');
    expect(formatLatency(2500)).toBe('2.50s');
  });
});

describe('formatSize', () => {
  test('formats bytes', () => {
    expect(formatSize(100)).toBe('100.00 B');
    expect(formatSize(500)).toBe('500.00 B');
  });

  test('formats kilobytes', () => {
    expect(formatSize(1024)).toBe('1.00 KB');
    expect(formatSize(2048)).toBe('2.00 KB');
  });

  test('formats megabytes', () => {
    expect(formatSize(1048576)).toBe('1.00 MB');
    expect(formatSize(5242880)).toBe('5.00 MB');
  });

  test('handles undefined', () => {
    expect(formatSize(undefined)).toBe('0 B');
  });

  test('handles zero', () => {
    expect(formatSize(0)).toBe('0 B');
  });
});

describe('formatJson', () => {
  test('formats with pretty (default)', () => {
    const result = formatJson({ a: 1 });
    expect(result).toBe('{\n  "a": 1\n}');
  });

  test('formats as raw', () => {
    const result = formatJson({ a: 1 }, 'raw');
    expect(result).toBe('{"a":1}');
  });

  test('formats as json (compact)', () => {
    const result = formatJson({ a: 1 }, 'json');
    expect(result).toBe('{"a":1}');
  });

  test('returns string as-is for raw format', () => {
    const result = formatJson('hello', 'raw');
    expect(result).toBe('hello');
  });

  test('handles arrays', () => {
    const result = formatJson([1, 2, 3], 'json');
    expect(result).toBe('[1,2,3]');
  });

  test('handles nested objects', () => {
    const result = formatJson({ a: { b: 1 } }, 'pretty');
    expect(result).toContain('"a"');
    expect(result).toContain('"b"');
  });
});

describe('truncate', () => {
  test('returns short strings unchanged', () => {
    expect(truncate('hello', 10)).toBe('hello');
  });

  test('truncates long strings with ellipsis', () => {
    expect(truncate('hello world', 8)).toBe('hello...');
  });

  test('handles exact length', () => {
    expect(truncate('hello', 5)).toBe('hello');
  });

  test('handles very short max length', () => {
    expect(truncate('hello world', 4)).toBe('h...');
  });
});

describe('getShortFilename', () => {
  test('removes directory path', () => {
    expect(getShortFilename('/path/to/file.yaml')).toBe('file');
  });

  test('removes .yaml extension', () => {
    expect(getShortFilename('api.yaml')).toBe('api');
  });

  test('handles nested paths', () => {
    expect(getShortFilename('/home/user/projects/api/tests/users.yaml')).toBe('users');
  });

  test('handles files without .yaml extension', () => {
    expect(getShortFilename('/path/to/file.json')).toBe('file.json');
  });
});

describe('formatPercent', () => {
  test('formats percentage with default decimals', () => {
    expect(formatPercent(0.5)).toBe('50.0%');
    expect(formatPercent(1)).toBe('100.0%');
    expect(formatPercent(0.123)).toBe('12.3%');
  });

  test('formats with custom decimals', () => {
    expect(formatPercent(0.5, 0)).toBe('50%');
    expect(formatPercent(0.123, 2)).toBe('12.30%');
  });

  test('handles zero', () => {
    expect(formatPercent(0)).toBe('0.0%');
  });
});

describe('padStart', () => {
  test('pads string to width', () => {
    expect(padStart('hi', 5)).toBe('   hi');
    expect(padStart('hello', 5)).toBe('hello');
  });
});

describe('padEnd', () => {
  test('pads string to width', () => {
    expect(padEnd('hi', 5)).toBe('hi   ');
    expect(padEnd('hello', 5)).toBe('hello');
  });
});

describe('repeatChar', () => {
  test('repeats character', () => {
    expect(repeatChar('-', 5)).toBe('-----');
    expect(repeatChar('=', 3)).toBe('===');
  });

  test('handles zero count', () => {
    expect(repeatChar('-', 0)).toBe('');
  });
});
