// Upgrade command for curl-runner
// Automatically detects installation source and upgrades to latest version

import { color } from '../utils/colors';
import {
  type DetectionResult,
  type InstallationSource,
  detectInstallationSource,
  getUpgradeCommand,
  getUpgradeCommandWindows,
  isWindows,
} from '../utils/installation-detector';
import { getVersion } from '../version';

const NPM_REGISTRY_URL = 'https://registry.npmjs.org/@curl-runner/cli/latest';
const GITHUB_API_URL = 'https://api.github.com/repos/alexvcasillas/curl-runner/releases/latest';

interface UpgradeOptions {
  dryRun?: boolean;
  force?: boolean;
}

export class UpgradeCommand {
  async run(args: string[]): Promise<void> {
    const options = this.parseArgs(args);

    console.log();
    console.log(color('curl-runner upgrade', 'bright'));
    console.log();

    // Detect installation source
    const detection = detectInstallationSource();
    console.log(`${color('Installation:', 'cyan')} ${this.formatSource(detection.source)}`);
    console.log(`${color('Current version:', 'cyan')} ${getVersion()}`);

    // Fetch latest version
    const latestVersion = await this.fetchLatestVersion(detection.source);
    if (!latestVersion) {
      console.log(color('Failed to fetch latest version', 'red'));
      process.exit(1);
    }
    console.log(`${color('Latest version:', 'cyan')} ${latestVersion}`);
    console.log();

    // Compare versions
    const currentVersion = getVersion();
    if (!options.force && !this.isNewerVersion(currentVersion, latestVersion)) {
      console.log(color('Already up to date!', 'green'));
      return;
    }

    // Get upgrade command
    const upgradeCmd = isWindows()
      ? getUpgradeCommandWindows(detection.source)
      : getUpgradeCommand(detection.source);

    if (options.dryRun) {
      console.log(color('Dry run - would execute:', 'yellow'));
      console.log(`  ${color(upgradeCmd, 'cyan')}`);
      return;
    }

    // Execute upgrade
    console.log(`${color('Upgrading...', 'yellow')}`);
    console.log();

    await this.executeUpgrade(detection, upgradeCmd);
  }

  private parseArgs(args: string[]): UpgradeOptions {
    const options: UpgradeOptions = {};

    for (const arg of args) {
      if (arg === '--dry-run' || arg === '-n') {
        options.dryRun = true;
      } else if (arg === '--force' || arg === '-f') {
        options.force = true;
      }
    }

    return options;
  }

  private formatSource(source: InstallationSource): string {
    switch (source) {
      case 'bun':
        return 'bun (global)';
      case 'npm':
        return 'npm (global)';
      case 'curl':
        return 'curl installer';
      case 'standalone':
        return 'standalone binary';
    }
  }

  private async fetchLatestVersion(source: InstallationSource): Promise<string | null> {
    try {
      // For npm/bun, use npm registry
      if (source === 'bun' || source === 'npm') {
        const response = await fetch(NPM_REGISTRY_URL, {
          signal: AbortSignal.timeout(5000),
        });
        if (!response.ok) return null;
        const data = (await response.json()) as { version: string };
        return data.version;
      }

      // For curl/standalone, use GitHub releases
      const response = await fetch(GITHUB_API_URL, {
        signal: AbortSignal.timeout(5000),
        headers: {
          Accept: 'application/vnd.github.v3+json',
        },
      });
      if (!response.ok) return null;
      const data = (await response.json()) as { tag_name: string };
      return data.tag_name.replace(/^v/, '');
    } catch {
      return null;
    }
  }

  private isNewerVersion(current: string, latest: string): boolean {
    const currentVersion = current.replace(/^v/, '');
    const latestVersion = latest.replace(/^v/, '');

    const currentParts = currentVersion.split('.').map(Number);
    const latestParts = latestVersion.split('.').map(Number);

    for (let i = 0; i < Math.max(currentParts.length, latestParts.length); i++) {
      const currentPart = currentParts[i] || 0;
      const latestPart = latestParts[i] || 0;

      if (latestPart > currentPart) return true;
      if (latestPart < currentPart) return false;
    }

    return false;
  }

  private async executeUpgrade(detection: DetectionResult, upgradeCmd: string): Promise<void> {
    const { source } = detection;

    try {
      if (source === 'bun') {
        await this.runBunUpgrade();
      } else if (source === 'npm') {
        await this.runNpmUpgrade();
      } else {
        // curl or standalone - run shell command
        await this.runShellUpgrade(upgradeCmd);
      }
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);

      // Check for permission errors
      if (errorMsg.includes('EACCES') || errorMsg.includes('permission')) {
        console.log();
        console.log(color('Permission denied. Try running with sudo:', 'yellow'));
        console.log(`  ${color(`sudo ${upgradeCmd}`, 'cyan')}`);
        process.exit(1);
      }

      console.log(color(`Upgrade failed: ${errorMsg}`, 'red'));
      console.log();
      console.log(color('Manual upgrade:', 'yellow'));
      console.log(`  ${color(upgradeCmd, 'cyan')}`);
      process.exit(1);
    }
  }

  private async runBunUpgrade(): Promise<void> {
    const proc = Bun.spawn(['bun', 'install', '-g', '@curl-runner/cli@latest'], {
      stdout: 'inherit',
      stderr: 'inherit',
    });

    const exitCode = await proc.exited;
    if (exitCode !== 0) {
      throw new Error(`bun install failed with exit code ${exitCode}`);
    }

    this.showSuccess();
  }

  private async runNpmUpgrade(): Promise<void> {
    const proc = Bun.spawn(['npm', 'install', '-g', '@curl-runner/cli@latest'], {
      stdout: 'inherit',
      stderr: 'inherit',
    });

    const exitCode = await proc.exited;
    if (exitCode !== 0) {
      throw new Error(`npm install failed with exit code ${exitCode}`);
    }

    this.showSuccess();
  }

  private async runShellUpgrade(cmd: string): Promise<void> {
    let proc: ReturnType<typeof Bun.spawn>;

    if (isWindows()) {
      // PowerShell for Windows
      proc = Bun.spawn(['powershell', '-Command', cmd], {
        stdout: 'inherit',
        stderr: 'inherit',
      });
    } else {
      // Bash for Unix
      proc = Bun.spawn(['bash', '-c', cmd], {
        stdout: 'inherit',
        stderr: 'inherit',
      });
    }

    const exitCode = await proc.exited;
    if (exitCode !== 0) {
      throw new Error(`Upgrade script failed with exit code ${exitCode}`);
    }

    this.showSuccess();
  }

  private showSuccess(): void {
    console.log();
    console.log(color('Upgrade complete!', 'green'));
    console.log();
    console.log('Run `curl-runner --version` to verify.');
  }
}

export function showUpgradeHelp(): void {
  console.log(`
${color('curl-runner upgrade', 'bright')}

Automatically upgrade curl-runner to the latest version.
Detects installation source (bun, npm, curl) and uses appropriate method.

${color('USAGE:', 'yellow')}
  curl-runner upgrade [options]

${color('OPTIONS:', 'yellow')}
  -n, --dry-run    Show what would be executed without running
  -f, --force      Force upgrade even if already on latest version
  -h, --help       Show this help message

${color('EXAMPLES:', 'yellow')}
  curl-runner upgrade           # Upgrade to latest
  curl-runner upgrade --dry-run # Preview upgrade command
  curl-runner upgrade --force   # Force reinstall latest
`);
}
