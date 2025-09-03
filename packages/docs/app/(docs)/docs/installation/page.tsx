import { DocsPageHeader } from "@/components/docs-page-header"
import { CodeBlockServer } from "@/components/code-block-server"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { CheckCircle, AlertCircle, Monitor, Laptop, HardDrive } from "lucide-react"

const installCommands = {
  bun: `# Install globally with Bun (recommended)
bun install -g @curl-runner/cli

# Or install locally in your project
bun add @curl-runner/cli`,
  
  npm: `# Install globally with npm
npm install -g @curl-runner/cli

# Or install locally in your project
npm install @curl-runner/cli`,
  
  binary: `# Download pre-built binary (Linux/macOS/Windows)
curl -fsSL https://install.curl-runner.dev | sh

# Or download from GitHub releases
# https://github.com/yourusername/curl-runner/releases`
}

const verifyInstall = `# Verify installation
curl-runner --version

# Show help
curl-runner --help

# Run example (if you have example files)
curl-runner examples/simple.yaml`

export default function InstallationPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Installation"
          text="Install curl-runner on your system using your preferred package manager or download a pre-built binary."
        />

        <div className="space-y-8">
          {/* Prerequisites */}
          <section>
            <h2 className="text-2xl font-semibold tracking-tight mb-4">Prerequisites</h2>
            <div className="space-y-3">
              <div className="flex items-center space-x-2">
                <CheckCircle className="h-5 w-5 text-green-500" />
                <h3 className="text-xl font-semibold">Bun Runtime</h3>
                <Badge variant="secondary">Recommended</Badge>
              </div>
              <p className="text-muted-foreground">
                <code className="font-mono">curl-runner</code> is optimized for Bun and provides the best performance when used with Bun runtime.
              </p>
              <CodeBlockServer language="bash">
                {`# Install Bun (if not already installed)
curl -fsSL https://bun.sh/install | bash

# Verify Bun installation
bun --version`}
              </CodeBlockServer>
            </div>
          </section>

          {/* Installation Methods */}
          <section>
            <h2 className="text-2xl font-semibold tracking-tight mb-4">Installation Methods</h2>
            
            <div className="space-y-8">
              {/* Bun Installation */}
              <div className="space-y-3">
                <div className="flex items-center space-x-2">
                  <h3 className="text-xl font-semibold">Bun Package Manager</h3>
                  <Badge variant="default">Recommended</Badge>
                </div>
                <p className="text-muted-foreground">
                  Install using Bun for optimal performance and compatibility.
                </p>
                <CodeBlockServer language="bash">
                  {installCommands.bun}
                </CodeBlockServer>
              </div>

              {/* npm Installation */}
              <div className="space-y-3">
                <h3 className="text-xl font-semibold">npm Package Manager</h3>
                <p className="text-muted-foreground">
                  Install using npm if you prefer Node.js ecosystem tools.
                </p>
                <CodeBlockServer language="bash">
                  {installCommands.npm}
                </CodeBlockServer>
              </div>

              {/* Binary Installation */}
              <div className="space-y-3">
                <h3 className="text-xl font-semibold">Pre-built Binary</h3>
                <p className="text-muted-foreground">
                  Download a standalone executable for your platform.
                </p>
                <CodeBlockServer language="bash">
                  {installCommands.binary}
                </CodeBlockServer>
              </div>
            </div>
          </section>

          {/* Verification */}
          <section>
            <h2 className="text-2xl font-semibold tracking-tight mb-4">Verify Installation</h2>
            <p className="text-muted-foreground mb-4">
              After installation, verify that <code className="font-mono">curl-runner</code> is working correctly:
            </p>
            <CodeBlockServer language="bash">
              {verifyInstall}
            </CodeBlockServer>
          </section>

          {/* Platform Support */}
          <section>
            <h2 className="text-2xl font-semibold tracking-tight mb-4">Platform Support</h2>
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <HardDrive className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      Linux <CheckCircle className="h-3 w-3 text-green-500" />
                    </h4>
                    <p className="text-sm text-muted-foreground">
                      Ubuntu 20.04+, CentOS 8+, Alpine Linux
                    </p>
                  </div>
                </div>
              </div>
              
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Laptop className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      macOS <CheckCircle className="h-3 w-3 text-green-500" />
                    </h4>
                    <p className="text-sm text-muted-foreground">
                      macOS 11+ (Intel & Apple Silicon)
                    </p>
                  </div>
                </div>
              </div>
              
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Monitor className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2 flex items-center gap-2">
                      Windows <CheckCircle className="h-3 w-3 text-green-500" />
                    </h4>
                    <p className="text-sm text-muted-foreground">
                      Windows 10+, WSL2 supported
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Troubleshooting */}
          <section>
            <h2 className="text-2xl font-semibold tracking-tight mb-4">Troubleshooting</h2>
            <div className="space-y-6">
              <div className="flex items-center space-x-2 text-amber-600">
                <AlertCircle className="h-5 w-5" />
                <h3 className="text-xl font-semibold">Common Issues</h3>
              </div>
              
              <div className="space-y-4">
                <div>
                  <h4 className="font-medium mb-2">Command not found</h4>
                  <p className="text-sm text-muted-foreground mb-2">
                    If you get "curl-runner: command not found", ensure that the installation directory is in your PATH.
                  </p>
                  <CodeBlockServer language="bash">
                    {`# Check if curl-runner is in PATH
which curl-runner

# Add to PATH if needed (replace with your installation path)
export PATH="$PATH:~/.bun/bin"`}
                  </CodeBlockServer>
                </div>
                
                <div>
                  <h4 className="font-medium mb-2">Permission denied</h4>
                  <p className="text-sm text-muted-foreground mb-2">
                    On Unix systems, you may need to make the binary executable:
                  </p>
                  <CodeBlockServer language="bash">
                    {`chmod +x /path/to/curl-runner`}
                  </CodeBlockServer>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>

      {/* Table of Contents */}
      <div className="hidden text-sm xl:block">
        <div className="sticky top-16 -mt-10 pt-4">
          <div className="space-y-2">
            <p className="font-medium">On This Page</p>
            <ul className="m-0 list-none">
              <li className="mt-0 pt-2">
                <a
                  href="#prerequisites"
                  className="inline-block no-underline transition-colors hover:text-foreground text-muted-foreground"
                >
                  Prerequisites
                </a>
              </li>
              <li className="mt-0 pt-2">
                <a
                  href="#installation-methods"
                  className="inline-block no-underline transition-colors hover:text-foreground text-muted-foreground"
                >
                  Installation Methods
                </a>
              </li>
              <li className="mt-0 pt-2">
                <a
                  href="#verify-installation"
                  className="inline-block no-underline transition-colors hover:text-foreground text-muted-foreground"
                >
                  Verify Installation
                </a>
              </li>
              <li className="mt-0 pt-2">
                <a
                  href="#platform-support"
                  className="inline-block no-underline transition-colors hover:text-foreground text-muted-foreground"
                >
                  Platform Support
                </a>
              </li>
              <li className="mt-0 pt-2">
                <a
                  href="#troubleshooting"
                  className="inline-block no-underline transition-colors hover:text-foreground text-muted-foreground"
                >
                  Troubleshooting
                </a>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </main>
  )
}