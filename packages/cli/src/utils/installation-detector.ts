// Installation source detection for curl-runner upgrade command

export type InstallationSource = 'bun' | 'npm' | 'curl' | 'standalone';

export interface DetectionResult {
  source: InstallationSource;
  path: string;
  canAutoUpgrade: boolean;
}

const CURL_INSTALL_PATHS = [
  `${process.env.HOME}/.local/bin/curl-runner`,
  `${process.env.USERPROFILE}\\.local\\bin\\curl-runner.exe`,
];

export function detectInstallationSource(): DetectionResult {
  const execPath = process.execPath;
  const argv0 = process.argv[0];

  // Check for bun installation
  if (isBunInstall(execPath)) {
    return {
      source: 'bun',
      path: execPath,
      canAutoUpgrade: true,
    };
  }

  // Check for npm installation
  if (isNpmInstall(execPath, argv0)) {
    return {
      source: 'npm',
      path: execPath,
      canAutoUpgrade: true,
    };
  }

  // Check for curl installation (binary in ~/.local/bin)
  if (isCurlInstall(execPath)) {
    return {
      source: 'curl',
      path: execPath,
      canAutoUpgrade: true,
    };
  }

  // Standalone binary
  return {
    source: 'standalone',
    path: execPath,
    canAutoUpgrade: true,
  };
}

function isBunInstall(execPath: string): boolean {
  // Check if running from bun's global install directory
  if (execPath.includes('.bun')) {
    return true;
  }
  if (process.env.BUN_INSTALL && execPath.includes(process.env.BUN_INSTALL)) {
    return true;
  }

  // Check for bun-specific paths
  const bunPaths = ['/bun/install/', '/.bun/bin/', '/bun/bin/'];
  return bunPaths.some((p) => execPath.includes(p));
}

function isNpmInstall(execPath: string, argv0: string): boolean {
  // Check for npm global install indicators
  if (execPath.includes('node_modules')) {
    return true;
  }
  if (argv0.includes('node_modules')) {
    return true;
  }

  // Check for npm config env vars (set when running via npm)
  if (process.env.npm_config_prefix) {
    return true;
  }
  if (process.env.npm_execpath) {
    return true;
  }

  // Check common npm global paths
  const npmPaths = ['/lib/node_modules/', '/node_modules/.bin/'];
  return npmPaths.some((p) => execPath.includes(p) || argv0.includes(p));
}

function isCurlInstall(execPath: string): boolean {
  // Check if binary is in curl installer's default location
  return CURL_INSTALL_PATHS.some((p) => execPath === p || execPath.startsWith(p));
}

export function getUpgradeCommand(source: InstallationSource): string {
  switch (source) {
    case 'bun':
      return 'bun install -g @curl-runner/cli@latest';
    case 'npm':
      return 'npm install -g @curl-runner/cli@latest';
    case 'curl':
      return 'curl -fsSL https://www.curl-runner.com/install.sh | bash';
    case 'standalone':
      return 'curl -fsSL https://www.curl-runner.com/install.sh | bash';
  }
}

export function getUpgradeCommandWindows(source: InstallationSource): string {
  switch (source) {
    case 'bun':
      return 'bun install -g @curl-runner/cli@latest';
    case 'npm':
      return 'npm install -g @curl-runner/cli@latest';
    case 'curl':
    case 'standalone':
      return 'irm https://www.curl-runner.com/install.ps1 | iex';
  }
}

export function isWindows(): boolean {
  return process.platform === 'win32';
}
