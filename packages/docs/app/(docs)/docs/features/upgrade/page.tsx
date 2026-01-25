import { ArrowUpCircle, CheckCircle, Download, RefreshCw, Terminal } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'Upgrade',
  description:
    'Easily upgrade curl-runner to the latest version with automatic installation source detection.',
  keywords: [
    'curl-runner upgrade',
    'update cli',
    'auto upgrade',
    'version update',
    'installation detection',
    'npm upgrade',
    'bun upgrade',
    'curl installer',
  ],
  openGraph: {
    title: 'Upgrade | curl-runner Documentation',
    description:
      'Easily upgrade curl-runner to the latest version with automatic installation source detection.',
    url: 'https://www.curl-runner.com/docs/features/upgrade',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Upgrade | curl-runner Documentation',
    description: 'Learn how to upgrade curl-runner to the latest version.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/upgrade',
  },
};

const basicUpgradeExample = `# Upgrade to latest version (auto-detects installation method)
curl-runner upgrade

# Preview what would happen without executing
curl-runner upgrade --dry-run

# Force reinstall even if already on latest version
curl-runner upgrade --force`;

const detectionExample = `# curl-runner automatically detects how it was installed:
#
# bun (global)     → runs: bun install -g @curl-runner/cli@latest
# npm (global)     → runs: npm install -g @curl-runner/cli@latest
# curl installer   → runs: curl -fsSL https://www.curl-runner.com/install.sh | bash
# standalone       → runs: curl -fsSL https://www.curl-runner.com/install.sh | bash`;

const versionCheckExample = `# curl-runner checks for updates on every run
# If a new version is available, you'll see:

╭───────────────────────────────────────────────╮
│                                               │
│  Update available! 1.15.0 → 1.16.0            │
│                                               │
│  Run curl-runner upgrade to update            │
│                                               │
╰───────────────────────────────────────────────╯`;

const outputExample = `$ curl-runner upgrade

curl-runner upgrade

Installation: bun (global)
Current version: 1.15.0
Latest version: 1.16.0

Upgrading...

bun install v1.1.0 (abc1234)
+ @curl-runner/cli@1.16.0

Upgrade complete!

Run \`curl-runner --version\` to verify.`;

const manualUpgradeExample = `# If automatic upgrade fails, use manual commands:

# For bun installations
bun install -g @curl-runner/cli@latest

# For npm installations
npm install -g @curl-runner/cli@latest

# For curl/standalone installations (Unix)
curl -fsSL https://www.curl-runner.com/install.sh | bash

# For curl/standalone installations (Windows PowerShell)
irm https://www.curl-runner.com/install.ps1 | iex`;

const permissionExample = `# If you get permission errors on Unix:
sudo npm install -g @curl-runner/cli@latest

# Or for curl installer, you may need to change install directory:
CURL_RUNNER_INSTALL_DIR=/usr/local/bin curl -fsSL https://www.curl-runner.com/install.sh | bash`;

export default function UpgradePage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Upgrade"
          text="Easily upgrade curl-runner to the latest version with automatic installation source detection."
        />

        <div className="space-y-12">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground text-lg mb-6">
              The <code className="font-mono">curl-runner upgrade</code> command automatically
              detects how curl-runner was installed and uses the appropriate method to upgrade to
              the latest version.
            </p>

            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <RefreshCw className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Auto-Detection</h4>
                    <p className="text-sm text-muted-foreground">
                      Detects bun, npm, or curl installation
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <ArrowUpCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">One Command</h4>
                    <p className="text-sm text-muted-foreground">
                      Simple upgrade regardless of install method
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Download className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Version Check</h4>
                    <p className="text-sm text-muted-foreground">
                      Automatic notification when updates available
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Basic Usage */}
          <section>
            <H2 id="basic-usage">Basic Usage</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Run the upgrade command to update to the latest version.
            </p>

            <CodeBlockServer language="bash" filename="Upgrade Commands">
              {basicUpgradeExample}
            </CodeBlockServer>
          </section>

          {/* Options */}
          <section>
            <H2 id="options">Options</H2>
            <p className="text-muted-foreground text-lg mb-6">Available options for the upgrade command.</p>

            <div className="grid gap-4 mb-6">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Terminal className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        -n, --dry-run
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Preview the upgrade command without executing it. Useful for seeing what would
                      happen.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <RefreshCw className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <code className="bg-muted px-2 py-1 rounded text-sm font-mono">
                        -f, --force
                      </code>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Force reinstall even if already on the latest version. Useful for
                      troubleshooting or reinstalling a clean copy.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Installation Detection */}
          <section>
            <H2 id="installation-detection">Installation Detection</H2>
            <p className="text-muted-foreground text-lg mb-6">
              curl-runner automatically detects the installation source and uses the appropriate
              upgrade method.
            </p>

            <div className="grid gap-2 mb-6 text-sm">
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <div className="flex items-center gap-2">
                  <Badge variant="secondary">bun</Badge>
                  <span className="font-mono text-xs">bun install -g @curl-runner/cli@latest</span>
                </div>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <div className="flex items-center gap-2">
                  <Badge variant="secondary">npm</Badge>
                  <span className="font-mono text-xs">npm install -g @curl-runner/cli@latest</span>
                </div>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <div className="flex items-center gap-2">
                  <Badge variant="secondary">curl</Badge>
                  <span className="font-mono text-xs">
                    curl -fsSL https://www.curl-runner.com/install.sh | bash
                  </span>
                </div>
              </div>
              <div className="flex items-center justify-between py-2 px-3 bg-muted/50 rounded">
                <div className="flex items-center gap-2">
                  <Badge variant="secondary">standalone</Badge>
                  <span className="font-mono text-xs">
                    curl -fsSL https://www.curl-runner.com/install.sh | bash
                  </span>
                </div>
              </div>
            </div>

            <CodeBlockServer language="bash" filename="Detection Logic">
              {detectionExample}
            </CodeBlockServer>
          </section>

          {/* Version Check */}
          <section>
            <H2 id="version-check">Automatic Version Check</H2>
            <p className="text-muted-foreground text-lg mb-6">
              curl-runner automatically checks for updates when you run commands. If a newer version
              is available, you'll see a notification.
            </p>

            <CodeBlockServer language="text" filename="Update Notification">
              {versionCheckExample}
            </CodeBlockServer>

            <div className="mt-4 rounded-lg border bg-card p-4">
              <div className="flex items-start gap-3">
                <div className="rounded-full bg-blue-500/10 p-2">
                  <CheckCircle className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                </div>
                <div>
                  <h4 className="font-medium mb-1">Non-blocking Check</h4>
                  <p className="text-sm text-muted-foreground">
                    Version checks run in the background and don't slow down your commands. Results
                    are cached for 24 hours. Checks are skipped in CI environments.
                  </p>
                </div>
              </div>
            </div>
          </section>

          {/* Output Example */}
          <section>
            <H2 id="output-example">Output Example</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Here's what a successful upgrade looks like.
            </p>

            <CodeBlockServer language="text" filename="Upgrade Output">
              {outputExample}
            </CodeBlockServer>
          </section>

          {/* Manual Upgrade */}
          <section>
            <H2 id="manual-upgrade">Manual Upgrade</H2>
            <p className="text-muted-foreground text-lg mb-6">
              If automatic upgrade doesn't work, you can use these manual commands.
            </p>

            <CodeBlockServer language="bash" filename="Manual Upgrade Commands">
              {manualUpgradeExample}
            </CodeBlockServer>
          </section>

          {/* Permissions */}
          <section>
            <H2 id="permissions">Permission Issues</H2>
            <p className="text-muted-foreground text-lg mb-6">
              If you encounter permission errors during upgrade, try these solutions.
            </p>

            <CodeBlockServer language="bash" filename="Permission Solutions">
              {permissionExample}
            </CodeBlockServer>
          </section>
        </div>
      </div>

      {/* Table of Contents */}
      <div className="hidden text-sm xl:block">
        <div className="sticky top-16 -mt-10 pt-4">
          <TableOfContents />
        </div>
      </div>
    </main>
  );
}
