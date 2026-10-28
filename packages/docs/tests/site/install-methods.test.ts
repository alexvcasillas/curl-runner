import { describe, expect, it } from 'vitest';
import { INSTALL_METHODS } from '@/lib/install-methods';

describe('INSTALL_METHODS', () => {
  it('has unique ids', () => {
    const ids = INSTALL_METHODS.map((m) => m.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('uses the tap for Homebrew', () => {
    expect(INSTALL_METHODS.find((m) => m.id === 'brew')?.command).toBe(
      'brew install alexvcasillas/tap/curl-runner',
    );
  });

  it('never installs through bun', () => {
    for (const m of INSTALL_METHODS) {
      expect(m.command).not.toContain('bun');
    }
  });

  it('installs with the shell script on macOS / Linux', () => {
    const curl = INSTALL_METHODS.find((m) => m.id === 'curl');
    expect(curl?.label).toBe('macOS / Linux');
    expect(curl?.command).toBe('curl -fsSL https://www.curl-runner.com/install.sh | bash');
  });
});
