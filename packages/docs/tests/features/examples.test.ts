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

  it.each(getAllFeatures().map((f) => [f.slug, f] as const))('%s', (slug, feature) => {
    const file = join(dir, `${slug}.yaml`);
    writeFileSync(file, feature.example.yaml);
    const result = spawnSync(bin as string, ['validate', '--strict', file], {
      encoding: 'utf8',
      env: { ...process.env, NO_COLOR: '1' },
    });
    expect(result.error).toBeUndefined();
    expect(result.status, `${result.stdout}\n${result.stderr}`).toBe(0);
  });
});
