import { getVersion } from '../version';
import { color } from './colors';

interface VersionCheckCache {
  lastCheck: number;
  latestVersion: string;
}

const CACHE_FILE = `${process.env.HOME}/.curl-runner-version-cache.json`;
const CACHE_DURATION = 24 * 60 * 60 * 1000; // 24 hours
const NPM_REGISTRY_URL = 'https://registry.npmjs.org/@curl-runner/cli/latest';

export class VersionChecker {
  async checkForUpdates(skipCache = false): Promise<void> {
    try {
      // Don't check in CI environments
      if (process.env.CI) {
        return;
      }

      const currentVersion = getVersion();

      // Don't check for development versions
      if (currentVersion === '0.0.0') {
        return;
      }

      // Check cache first
      if (!skipCache) {
        const cached = await this.getCachedVersion();
        if (cached && Date.now() - cached.lastCheck < CACHE_DURATION) {
          this.compareVersions(currentVersion, cached.latestVersion);
          return;
        }
      }

      // Fetch latest version from npm registry
      const latestVersion = await this.fetchLatestVersion();
      if (latestVersion) {
        // Update cache
        await this.setCachedVersion(latestVersion);

        // Compare versions
        this.compareVersions(currentVersion, latestVersion);
      }
    } catch {
      // Silently fail - we don't want to interrupt the CLI usage
      // due to version check failures
    }
  }

  private async fetchLatestVersion(): Promise<string | null> {
    try {
      const response = await fetch(NPM_REGISTRY_URL, {
        signal: AbortSignal.timeout(3000), // 3 second timeout
      });

      if (!response.ok) {
        return null;
      }

      const data = (await response.json()) as { version: string };
      return data.version;
    } catch {
      return null;
    }
  }

  private compareVersions(current: string, latest: string): void {
    if (this.isNewerVersion(current, latest)) {
      console.log();
      console.log(color('╭───────────────────────────────────────────────╮', 'yellow'));
      console.log(
        color('│', 'yellow') +
          '                                               ' +
          color('│', 'yellow'),
      );
      console.log(
        color('│', 'yellow') +
          '  ' +
          color('Update available!', 'bright') +
          ` ${color(current, 'red')} → ${color(latest, 'green')}` +
          '        ' +
          color('│', 'yellow'),
      );
      console.log(
        color('│', 'yellow') +
          '                                               ' +
          color('│', 'yellow'),
      );
      console.log(
        color('│', 'yellow') +
          '  Run ' +
          color('curl-runner upgrade', 'cyan') +
          ' to update          ' +
          color('│', 'yellow'),
      );
      console.log(
        color('│', 'yellow') +
          '                                               ' +
          color('│', 'yellow'),
      );
      console.log(color('╰───────────────────────────────────────────────╯', 'yellow'));
      console.log();
    }
  }

  private isNewerVersion(current: string, latest: string): boolean {
    try {
      // Remove 'v' prefix if present
      const currentVersion = current.replace(/^v/, '');
      const latestVersion = latest.replace(/^v/, '');

      const currentParts = currentVersion.split('.').map(Number);
      const latestParts = latestVersion.split('.').map(Number);

      for (let i = 0; i < Math.max(currentParts.length, latestParts.length); i++) {
        const currentPart = currentParts[i] || 0;
        const latestPart = latestParts[i] || 0;

        if (latestPart > currentPart) {
          return true;
        }
        if (latestPart < currentPart) {
          return false;
        }
      }

      return false;
    } catch {
      return false;
    }
  }

  private async getCachedVersion(): Promise<VersionCheckCache | null> {
    try {
      const file = Bun.file(CACHE_FILE);
      if (await file.exists()) {
        return JSON.parse(await file.text());
      }
    } catch {
      // Ignore cache read errors
    }
    return null;
  }

  private async setCachedVersion(latestVersion: string): Promise<void> {
    try {
      const cache: VersionCheckCache = {
        lastCheck: Date.now(),
        latestVersion,
      };
      await Bun.write(CACHE_FILE, JSON.stringify(cache));
    } catch {
      // Ignore cache write errors
    }
  }
}
