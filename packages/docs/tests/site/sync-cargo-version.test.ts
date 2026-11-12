import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

// The repo's version step: `changeset version && bash scripts/sync-cargo-version.sh`.
const script = resolve(__dirname, '../../../../scripts/sync-cargo-version.sh');

const CARGO_TOML = `[workspace]
members = ["crates/a", "crates/b"]

[workspace.package]
# Synced from packages/cli/package.json by scripts/sync-cargo-version.sh.
version = "2.7.1"

[workspace.dependencies]
curl-runner-core = { path = "crates/curl-runner-core", version = "=2.7.1" }
curl-runner-cloud-proto = { path = "crates/curl-runner-cloud-proto", version = "=2.7.1" }
curl-runner-new-crate = { path = "crates/curl-runner-new-crate", version = "=2.7.1" }
serde = { version = "=1.0.0" }
`;

const CARGO_LOCK = `[[package]]
name = "curl-runner-core"
version = "2.7.1"

[[package]]
name = "serde"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
`;

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), 'sync-cargo-version-'));
  mkdirSync(join(root, 'packages/cli'), { recursive: true });
  writeFileSync(join(root, 'packages/cli/package.json'), '{"version":"2.8.0"}');
  writeFileSync(join(root, 'Cargo.toml'), CARGO_TOML);
  writeFileSync(join(root, 'Cargo.lock'), CARGO_LOCK);
});

afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

const run = () => spawnSync('bash', [script, root], { encoding: 'utf8' });

describe('sync-cargo-version.sh', () => {
  it('pins every workspace crate dependency, new ones included', () => {
    expect(run().status).toBe(0);
    const toml = readFileSync(join(root, 'Cargo.toml'), 'utf8');
    expect(toml).toContain('version = "2.8.0"');
    expect(toml).not.toContain('=2.7.1');
    expect(toml).toContain(
      'curl-runner-new-crate = { path = "crates/curl-runner-new-crate", version = "=2.8.0" }',
    );
    // Registry dependencies keep their own versions.
    expect(toml).toContain('serde = { version = "=1.0.0" }');
  });

  it('bumps workspace crates in Cargo.lock, not registry ones', () => {
    expect(run().status).toBe(0);
    const lock = readFileSync(join(root, 'Cargo.lock'), 'utf8');
    expect(lock).toContain('name = "curl-runner-core"\nversion = "2.8.0"');
    expect(lock).toContain('name = "serde"\nversion = "1.0.0"');
  });

  it('refuses a malformed version', () => {
    writeFileSync(join(root, 'packages/cli/package.json'), '{"version":"next"}');
    const out = run();
    expect(out.status).not.toBe(0);
    expect(out.stderr).toContain("unexpected version 'next'");
  });
});
