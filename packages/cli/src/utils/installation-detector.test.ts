import { describe, expect, test } from 'bun:test';
import {
  getUpgradeCommand,
  getUpgradeCommandWindows,
  isWindows,
} from './installation-detector';

describe('getUpgradeCommand', () => {
  test('returns bun command for bun source', () => {
    expect(getUpgradeCommand('bun')).toBe('bun install -g @curl-runner/cli@latest');
  });

  test('returns npm command for npm source', () => {
    expect(getUpgradeCommand('npm')).toBe('npm install -g @curl-runner/cli@latest');
  });

  test('returns curl command for curl source', () => {
    expect(getUpgradeCommand('curl')).toBe('curl -fsSL https://www.curl-runner.com/install.sh | bash');
  });

  test('returns curl command for standalone source', () => {
    expect(getUpgradeCommand('standalone')).toBe('curl -fsSL https://www.curl-runner.com/install.sh | bash');
  });
});

describe('getUpgradeCommandWindows', () => {
  test('returns bun command for bun source', () => {
    expect(getUpgradeCommandWindows('bun')).toBe('bun install -g @curl-runner/cli@latest');
  });

  test('returns npm command for npm source', () => {
    expect(getUpgradeCommandWindows('npm')).toBe('npm install -g @curl-runner/cli@latest');
  });

  test('returns PowerShell command for curl source', () => {
    expect(getUpgradeCommandWindows('curl')).toBe('irm https://www.curl-runner.com/install.ps1 | iex');
  });

  test('returns PowerShell command for standalone source', () => {
    expect(getUpgradeCommandWindows('standalone')).toBe('irm https://www.curl-runner.com/install.ps1 | iex');
  });
});

describe('isWindows', () => {
  test('returns boolean based on platform', () => {
    expect(typeof isWindows()).toBe('boolean');
  });
});
