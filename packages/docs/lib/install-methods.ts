export type InstallMethodId = 'curl' | 'powershell' | 'brew' | 'npm' | 'cargo';

export interface InstallMethod {
  id: InstallMethodId;
  label: string;
  command: string;
  note: string;
}

/** Homebrew asks you to trust third-party taps before it installs from them. */
export const BREW_TRUST_COMMAND = 'brew trust alexvcasillas/tap';

export const INSTALL_METHODS: readonly InstallMethod[] = [
  {
    id: 'curl',
    label: 'macOS / Linux',
    command: 'curl -fsSL https://www.curl-runner.com/install.sh | bash',
    note: 'Installs to ~/.local/bin and verifies the checksum. Upgrade later with curl-runner upgrade.',
  },
  {
    id: 'powershell',
    label: 'Windows',
    command: 'irm https://www.curl-runner.com/install.ps1 | iex',
    note: 'PowerShell installer. Verifies the checksum and adds curl-runner to your PATH.',
  },
  {
    id: 'brew',
    label: 'Homebrew',
    command: `${BREW_TRUST_COMMAND} && brew install alexvcasillas/tap/curl-runner`,
    note: 'macOS and Linux. Homebrew asks you to trust the tap first. Upgrade with brew upgrade curl-runner.',
  },
  {
    id: 'npm',
    label: 'npm',
    command: 'npm i -g @curl-runner/cli',
    note: 'Ships the native binary for your platform; works with bun and pnpm too. Node is not needed to run it.',
  },
  {
    id: 'cargo',
    label: 'cargo',
    command: 'cargo install curl-runner --locked',
    note: 'Builds from source; needs a C compiler and cmake.',
  },
];
