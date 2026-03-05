import { describe, expect, test } from 'bun:test';
import { resolveConfig } from './resolver';

describe('resolver quiet flag', () => {
  test('--quiet sets output.quiet: true', async () => {
    const result = await resolveConfig(['--quiet'], {
      onInfo: () => {},
      onWarning: () => {},
    });
    expect(result.config.output?.quiet).toBe(true);
  });

  test('-q sets output.quiet: true', async () => {
    const result = await resolveConfig(['-q'], {
      onInfo: () => {},
      onWarning: () => {},
    });
    expect(result.config.output?.quiet).toBe(true);
  });

  test('--quiet also sets output.verbose: false', async () => {
    const result = await resolveConfig(['--quiet'], {
      onInfo: () => {},
      onWarning: () => {},
    });
    expect(result.config.output?.verbose).toBe(false);
  });
});
