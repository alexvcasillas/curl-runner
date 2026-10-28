import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterAll, describe, expect, it } from 'vitest';
import { getAllFeatures } from '@/lib/features-data';

// Validates every feature page's YAML example with the real CLI. Needs a built
// binary: CURL_RUNNER_BIN=$PWD/target/debug/curl-runner pnpm --filter @curl-runner/docs test examples
const bin = process.env.CURL_RUNNER_BIN;

describe.skipIf(!bin)('feature examples pass `curl-runner validate --strict`', () => {
  const dir = mkdtempSync(join(tmpdir(), 'curl-runner-examples-'));
  afterAll(() => rmSync(dir, { recursive: true, force: true }));

  const samples = getAllFeatures().flatMap((f) => [
    [f.slug, f.example.yaml] as const,
    ...(f.fromCurl ? [[`${f.slug}-from-curl`, f.fromCurl.yaml] as const] : []),
  ]);

  it.each(samples)('%s', (name, source) => {
    const file = join(dir, `${name}.yaml`);
    writeFileSync(file, source);
    const result = spawnSync(bin as string, ['validate', '--strict', file], {
      encoding: 'utf8',
      env: { ...process.env, NO_COLOR: '1' },
    });
    expect(result.error).toBeUndefined();
    expect(result.status, `${result.stdout}\n${result.stderr}`).toBe(0);
  });
});
