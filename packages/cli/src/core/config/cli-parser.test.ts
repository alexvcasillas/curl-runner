import { describe, expect, test } from 'bun:test';
import {
  detectEarlyExit,
  detectSubcommand,
  parseCliArgs,
  parseEditArgs,
  parseInitArgs,
} from './cli-parser';

describe('parseCliArgs', () => {
  test('returns empty files array for no args', () => {
    const result = parseCliArgs([]);
    expect(result.files).toEqual([]);
  });

  test('parses positional file arguments', () => {
    const result = parseCliArgs(['api.yaml', 'test.yaml']);
    expect(result.files).toEqual(['api.yaml', 'test.yaml']);
  });

  describe('boolean flags', () => {
    test('parses --help', () => {
      expect(parseCliArgs(['--help']).help).toBe(true);
    });

    test('parses --version', () => {
      expect(parseCliArgs(['--version']).version).toBe(true);
    });

    test('parses --all', () => {
      expect(parseCliArgs(['--all']).all).toBe(true);
    });

    test('parses --dry-run', () => {
      expect(parseCliArgs(['--dry-run']).dryRun).toBe(true);
    });

    test('parses --http2', () => {
      expect(parseCliArgs(['--http2']).http2).toBe(true);
    });

    test('parses --connection-pool', () => {
      expect(parseCliArgs(['--connection-pool']).connectionPool).toBe(true);
    });

    test('parses --quiet', () => {
      expect(parseCliArgs(['--quiet']).quiet).toBe(true);
    });

    test('parses --show-headers', () => {
      expect(parseCliArgs(['--show-headers']).showHeaders).toBe(true);
    });

    test('parses --show-body', () => {
      expect(parseCliArgs(['--show-body']).showBody).toBe(true);
    });

    test('parses --show-metrics', () => {
      expect(parseCliArgs(['--show-metrics']).showMetrics).toBe(true);
    });

    test('parses --no-retry', () => {
      expect(parseCliArgs(['--no-retry']).noRetry).toBe(true);
    });

    test('parses --watch', () => {
      expect(parseCliArgs(['--watch']).watch).toBe(true);
    });

    test('parses --watch-clear', () => {
      expect(parseCliArgs(['--watch-clear']).watchClear).toBe(true);
    });

    test('parses --no-watch-clear', () => {
      expect(parseCliArgs(['--no-watch-clear']).watchClear).toBe(false);
    });

    test('parses --profile-histogram', () => {
      expect(parseCliArgs(['--profile-histogram']).profileHistogram).toBe(true);
    });

    test('parses --snapshot', () => {
      expect(parseCliArgs(['--snapshot']).snapshot).toBe(true);
    });

    test('parses --update-snapshots', () => {
      expect(parseCliArgs(['--update-snapshots']).snapshotUpdate).toBe('all');
    });

    test('parses --update-failing', () => {
      expect(parseCliArgs(['--update-failing']).snapshotUpdate).toBe('failing');
    });

    test('parses --ci-snapshot', () => {
      expect(parseCliArgs(['--ci-snapshot']).snapshotCi).toBe(true);
    });

    test('parses --diff', () => {
      expect(parseCliArgs(['--diff']).diff).toBe(true);
    });

    test('parses --diff-save', () => {
      expect(parseCliArgs(['--diff-save']).diffSave).toBe(true);
    });

    test('parses --strict-exit', () => {
      expect(parseCliArgs(['--strict-exit']).strictExit).toBe(true);
    });

    test('parses --verbose without value', () => {
      expect(parseCliArgs(['--verbose']).verbose).toBe(true);
    });

    test('parses --continue-on-error without value', () => {
      expect(parseCliArgs(['--continue-on-error']).continueOnError).toBe(true);
    });
  });

  describe('value flags', () => {
    test('parses --execution sequential', () => {
      expect(parseCliArgs(['--execution', 'sequential']).execution).toBe('sequential');
    });

    test('parses --execution parallel', () => {
      expect(parseCliArgs(['--execution', 'parallel']).execution).toBe('parallel');
    });

    test('parses --max-concurrent', () => {
      expect(parseCliArgs(['--max-concurrent', '5']).maxConcurrency).toBe(5);
    });

    test('ignores invalid --max-concurrent', () => {
      expect(parseCliArgs(['--max-concurrent', '0']).maxConcurrency).toBeUndefined();
      expect(parseCliArgs(['--max-concurrent', '-1']).maxConcurrency).toBeUndefined();
    });

    test('parses --max-streams', () => {
      expect(parseCliArgs(['--max-streams', '10']).maxStreams).toBe(10);
    });

    test('parses --keepalive-time', () => {
      expect(parseCliArgs(['--keepalive-time', '60']).keepaliveTime).toBe(60);
    });

    test('parses --connect-timeout', () => {
      expect(parseCliArgs(['--connect-timeout', '30']).connectTimeout).toBe(30);
    });

    test('parses --verbose true', () => {
      expect(parseCliArgs(['--verbose', 'true']).verbose).toBe(true);
    });

    test('parses --verbose false', () => {
      expect(parseCliArgs(['--verbose', 'false']).verbose).toBe(false);
    });

    test('parses --output', () => {
      expect(parseCliArgs(['--output', 'result.json']).output).toBe('result.json');
    });

    test('parses --output-format json', () => {
      expect(parseCliArgs(['--output-format', 'json']).outputFormat).toBe('json');
    });

    test('parses --output-format pretty', () => {
      expect(parseCliArgs(['--output-format', 'pretty']).outputFormat).toBe('pretty');
    });

    test('parses --output-format raw', () => {
      expect(parseCliArgs(['--output-format', 'raw']).outputFormat).toBe('raw');
    });

    test('parses --pretty-level minimal', () => {
      expect(parseCliArgs(['--pretty-level', 'minimal']).prettyLevel).toBe('minimal');
    });

    test('parses --pretty-level standard', () => {
      expect(parseCliArgs(['--pretty-level', 'standard']).prettyLevel).toBe('standard');
    });

    test('parses --pretty-level detailed', () => {
      expect(parseCliArgs(['--pretty-level', 'detailed']).prettyLevel).toBe('detailed');
    });

    test('parses --timeout', () => {
      expect(parseCliArgs(['--timeout', '30']).timeout).toBe(30);
    });

    test('parses --retries', () => {
      expect(parseCliArgs(['--retries', '3']).retries).toBe(3);
    });

    test('parses --retry-delay', () => {
      expect(parseCliArgs(['--retry-delay', '1000']).retryDelay).toBe(1000);
    });

    test('parses --continue-on-error true', () => {
      expect(parseCliArgs(['--continue-on-error', 'true']).continueOnError).toBe(true);
    });

    test('parses --watch-debounce', () => {
      expect(parseCliArgs(['--watch-debounce', '500']).watchDebounce).toBe(500);
    });

    test('parses --profile', () => {
      expect(parseCliArgs(['--profile', '100']).profile).toBe(100);
    });

    test('parses --profile-warmup', () => {
      expect(parseCliArgs(['--profile-warmup', '5']).profileWarmup).toBe(5);
    });

    test('parses --profile-concurrency', () => {
      expect(parseCliArgs(['--profile-concurrency', '2']).profileConcurrency).toBe(2);
    });

    test('parses --profile-export', () => {
      expect(parseCliArgs(['--profile-export', 'results.json']).profileExport).toBe('results.json');
    });

    test('parses --snapshot-dir', () => {
      expect(parseCliArgs(['--snapshot-dir', '.snapshots']).snapshotDir).toBe('.snapshots');
    });

    test('parses --diff-label', () => {
      expect(parseCliArgs(['--diff-label', 'prod']).diffLabel).toBe('prod');
    });

    test('parses --diff-compare', () => {
      expect(parseCliArgs(['--diff-compare', 'baseline']).diffCompare).toBe('baseline');
    });

    test('parses --diff-dir', () => {
      expect(parseCliArgs(['--diff-dir', '.baselines']).diffDir).toBe('.baselines');
    });

    test('parses --diff-output terminal', () => {
      expect(parseCliArgs(['--diff-output', 'terminal']).diffOutput).toBe('terminal');
    });

    test('parses --diff-output json', () => {
      expect(parseCliArgs(['--diff-output', 'json']).diffOutput).toBe('json');
    });

    test('parses --diff-output markdown', () => {
      expect(parseCliArgs(['--diff-output', 'markdown']).diffOutput).toBe('markdown');
    });

    test('parses --fail-on', () => {
      expect(parseCliArgs(['--fail-on', '2']).failOn).toBe(2);
    });

    test('parses --fail-on-percentage', () => {
      expect(parseCliArgs(['--fail-on-percentage', '10.5']).failOnPercentage).toBe(10.5);
    });

    test('ignores invalid --fail-on-percentage', () => {
      expect(parseCliArgs(['--fail-on-percentage', '150']).failOnPercentage).toBeUndefined();
      expect(parseCliArgs(['--fail-on-percentage', '-5']).failOnPercentage).toBeUndefined();
    });
  });

  describe('short flags', () => {
    test('parses -h', () => {
      expect(parseCliArgs(['-h']).help).toBe(true);
    });

    test('parses -v', () => {
      expect(parseCliArgs(['-v']).verbose).toBe(true);
    });

    test('parses -p', () => {
      expect(parseCliArgs(['-p']).execution).toBe('parallel');
    });

    test('parses -q', () => {
      expect(parseCliArgs(['-q']).quiet).toBe(true);
    });

    test('parses -w', () => {
      expect(parseCliArgs(['-w']).watch).toBe(true);
    });

    test('parses -s', () => {
      expect(parseCliArgs(['-s']).snapshot).toBe(true);
    });

    test('parses -u', () => {
      expect(parseCliArgs(['-u']).snapshotUpdate).toBe('all');
    });

    test('parses -d', () => {
      expect(parseCliArgs(['-d']).diff).toBe(true);
    });

    test('parses -n', () => {
      expect(parseCliArgs(['-n']).dryRun).toBe(true);
    });

    test('parses -o with value', () => {
      expect(parseCliArgs(['-o', 'out.json']).output).toBe('out.json');
    });

    test('parses -P with value', () => {
      expect(parseCliArgs(['-P', '50']).profile).toBe(50);
    });

    test('parses combined short flags -vq', () => {
      const result = parseCliArgs(['-vq']);
      expect(result.verbose).toBe(true);
      expect(result.quiet).toBe(true);
    });

    test('parses combined short flags -pws', () => {
      const result = parseCliArgs(['-pws']);
      expect(result.execution).toBe('parallel');
      expect(result.watch).toBe(true);
      expect(result.snapshot).toBe(true);
    });
  });

  describe('mixed arguments', () => {
    test('parses files with boolean flags', () => {
      const result = parseCliArgs(['api.yaml', '--dry-run', 'test.yaml', '--quiet']);
      expect(result.files).toEqual(['api.yaml', 'test.yaml']);
      expect(result.dryRun).toBe(true);
      expect(result.quiet).toBe(true);
    });

    test('parses complex command line', () => {
      const result = parseCliArgs([
        'api.yaml',
        '--execution',
        'parallel',
        '--max-concurrent',
        '5',
        '--timeout',
        '30',
        '-v',
        '--http2',
      ]);
      expect(result.files).toEqual(['api.yaml']);
      expect(result.execution).toBe('parallel');
      expect(result.maxConcurrency).toBe(5);
      expect(result.timeout).toBe(30);
      expect(result.verbose).toBe(true);
      expect(result.http2).toBe(true);
    });
  });
});

describe('detectSubcommand', () => {
  test('detects upgrade subcommand', () => {
    expect(detectSubcommand(['upgrade'])).toBe('upgrade');
  });

  test('detects diff subcommand with 3+ args', () => {
    expect(detectSubcommand(['diff', 'baseline', 'current'])).toBe('diff-subcommand');
  });

  test('does not detect diff with < 3 args', () => {
    expect(detectSubcommand(['diff'])).toBeNull();
    expect(detectSubcommand(['diff', 'baseline'])).toBeNull();
  });

  test('detects init subcommand', () => {
    expect(detectSubcommand(['init'])).toBe('init');
    expect(detectSubcommand(['init', '--wizard'])).toBe('init');
    expect(detectSubcommand(['init', 'https://api.example.com'])).toBe('init');
  });

  test('detects edit subcommand', () => {
    expect(detectSubcommand(['edit'])).toBe('edit');
    expect(detectSubcommand(['edit', 'api.yaml'])).toBe('edit');
    expect(detectSubcommand(['edit', 'api.yaml', '-o', 'new.yaml'])).toBe('edit');
  });

  test('returns null for normal args', () => {
    expect(detectSubcommand(['api.yaml'])).toBeNull();
    expect(detectSubcommand(['--help'])).toBeNull();
  });
});

describe('parseInitArgs', () => {
  test('parses --wizard flag', () => {
    const result = parseInitArgs(['init', '--wizard']);
    expect(result.wizard).toBe(true);
  });

  test('parses -w flag', () => {
    const result = parseInitArgs(['init', '-w']);
    expect(result.wizard).toBe(true);
  });

  test('parses URL argument', () => {
    const result = parseInitArgs(['init', 'https://api.example.com']);
    expect(result.url).toBe('https://api.example.com');
  });

  test('parses --output flag', () => {
    const result = parseInitArgs(['init', '--output', 'api.yaml']);
    expect(result.outputPath).toBe('api.yaml');
  });

  test('parses -o flag', () => {
    const result = parseInitArgs(['init', '-o', 'api.yaml']);
    expect(result.outputPath).toBe('api.yaml');
  });

  test('parses all options together', () => {
    const result = parseInitArgs(['init', '--wizard', '-o', 'api.yaml']);
    expect(result.wizard).toBe(true);
    expect(result.outputPath).toBe('api.yaml');
  });

  test('parses URL with output', () => {
    const result = parseInitArgs(['init', 'https://api.example.com', '-o', 'api.yaml']);
    expect(result.url).toBe('https://api.example.com');
    expect(result.outputPath).toBe('api.yaml');
  });
});

describe('parseEditArgs', () => {
  test('parses file argument', () => {
    const result = parseEditArgs(['edit', 'api.yaml']);
    expect(result.file).toBe('api.yaml');
  });

  test('parses --output flag', () => {
    const result = parseEditArgs(['edit', 'api.yaml', '--output', 'new.yaml']);
    expect(result.file).toBe('api.yaml');
    expect(result.outputPath).toBe('new.yaml');
  });

  test('parses -o flag', () => {
    const result = parseEditArgs(['edit', 'api.yaml', '-o', 'new.yaml']);
    expect(result.file).toBe('api.yaml');
    expect(result.outputPath).toBe('new.yaml');
  });

  test('returns empty file for no args', () => {
    const result = parseEditArgs(['edit']);
    expect(result.file).toBe('');
  });
});

describe('detectEarlyExit', () => {
  test('detects help flag', () => {
    expect(detectEarlyExit({ files: [], help: true })).toBe('help');
  });

  test('detects version flag', () => {
    expect(detectEarlyExit({ files: [], version: true })).toBe('version');
  });

  test('prefers help over version', () => {
    expect(detectEarlyExit({ files: [], help: true, version: true })).toBe('help');
  });

  test('returns null when no early exit flags', () => {
    expect(detectEarlyExit({ files: [] })).toBeNull();
    expect(detectEarlyExit({ files: [], verbose: true })).toBeNull();
  });
});
