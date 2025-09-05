// Version management for curl-runner CLI
// This file handles version detection for both development and compiled binaries

declare const BUILD_VERSION: string | undefined;

export function getVersion(): string {
  // Check compile-time constant first (set by --define flag during build)
  if (typeof BUILD_VERSION !== 'undefined') {
    return BUILD_VERSION;
  }

  // Check environment variable (for local builds with our script)
  if (process.env.CURL_RUNNER_VERSION) {
    return process.env.CURL_RUNNER_VERSION;
  }

  // In development or npm installation, try to read from package.json
  try {
    // Try multiple paths to find package.json
    const possiblePaths = [
      '../package.json', // Development or npm installation
      './package.json', // In case we're at root
      '../../package.json', // In case of different directory structure
    ];

    for (const path of possiblePaths) {
      try {
        const packageJson = require(path);
        if (packageJson.name === '@curl-runner/cli' && packageJson.version) {
          return packageJson.version;
        }
      } catch {
        // Try next path
      }
    }

    // If no package.json found, return default
    return '0.0.0';
  } catch {
    // If all else fails, return a default version
    return '0.0.0';
  }
}
