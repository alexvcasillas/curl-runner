import { AlertCircle, CheckCircle, FileLock, KeyRound, ShieldCheck } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';

export const metadata: Metadata = {
  title: 'Security',
  description:
    'Security controls in curl-runner: URL protocol allow-list, filesystem path confinement, and automatic secret redaction. Secure defaults with explicit opt-outs.',
  keywords: [
    'curl-runner security',
    'SSRF protection',
    'protocol allow-list',
    'path confinement',
    'secret redaction',
    'allow-protocol',
    'allow-path',
    'secure defaults',
    'API testing security',
  ],
  openGraph: {
    title: 'Security | curl-runner Documentation',
    description:
      'URL protocol allow-list, filesystem path confinement, and secret redaction in curl-runner.',
    url: 'https://www.curl-runner.com/docs/security',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Security | curl-runner Documentation',
    description: 'Protocol allow-list, path confinement, and secret redaction in curl-runner.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/security',
  },
};

const protocolYaml = `# Allow extra protocols for a whole run (config file)
global:
  security:
    # Default is [http, https]. Add others only if you need them.
    allowedProtocols: [http, https, ftp]`;

const protocolCli = `# Permit additional protocols on the command line (repeatable)
curl-runner tests/ --allow-protocol ftp --allow-protocol file

# Or via environment variable (comma-separated)
CURL_RUNNER_ALLOWED_PROTOCOLS=http,https,ftp curl-runner tests/

# Blocked by default — fails before curl runs:
#   url: file:///etc/passwd
#   url: gopher://127.0.0.1:6379/`;

const pathYaml = `# By default these paths must stay inside the working directory
request:
  name: Upload report
  url: https://api.example.com/upload
  method: POST
  formData:
    file:
      file: ./report.pdf        # OK: inside the working directory
      # file: /etc/passwd       # Blocked unless --allow-path is set
  output: ./results/upload.json  # OK
  # output: ../outside.json      # Blocked unless --allow-path is set`;

const pathCli = `# Allow paths outside the working directory (opt-out)
curl-runner upload.yaml --allow-path

# Or via environment variable
CURL_RUNNER_ALLOW_PATHS=true curl-runner upload.yaml`;

const redactionExample = `# Secrets in your config are masked in console output and saved files
request:
  name: Get profile
  url: https://api.example.com/me
  auth:
    type: bearer
    token: \${API_TOKEN}        # masked as [REDACTED] in output
  headers:
    X-Api-Key: \${SECRET_KEY}   # sensitive header names are masked too

# Disable redaction if you really need raw output
# curl-runner api.yaml --no-redact`;

export default function SecurityPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Security"
          text="curl-runner ships with secure defaults that limit what a request can reach and reveal, with explicit opt-outs when you need them."
        />

        <div className="space-y-12">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Because curl-runner builds and runs curl commands from YAML — and can feed values from
              one response into later requests — untrusted configs or responses could otherwise
              reach internal services, read or overwrite local files, or leak credentials. The
              controls below are on by default and can be relaxed per run.
            </p>

            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-red-500/10 p-2">
                    <ShieldCheck className="h-4 w-4 text-red-600 dark:text-red-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Protocol Allow-List</h4>
                    <p className="text-sm text-muted-foreground">
                      Only http/https run unless you opt in to more
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-amber-500/10 p-2">
                    <FileLock className="h-4 w-4 text-amber-600 dark:text-amber-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Path Confinement</h4>
                    <p className="text-sm text-muted-foreground">
                      File reads/writes stay in the working directory
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <KeyRound className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Secret Redaction</h4>
                    <p className="text-sm text-muted-foreground">
                      Credentials are masked in output and saved files
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Protocol allow-list */}
          <section>
            <H2 id="protocol-allow-list">URL Protocol Allow-List</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Requests may only use <code className="font-mono">http</code> and{' '}
              <code className="font-mono">https</code> by default. Any other scheme (
              <code className="font-mono">file</code>, <code className="font-mono">ftp</code>,{' '}
              <code className="font-mono">gopher</code>, …) is rejected before curl is invoked, and
              redirects are likewise prevented from crossing into a disallowed protocol. This guards
              against SSRF and local file disclosure — including via values interpolated from a
              previous response.
            </p>

            <div className="space-y-4">
              <CodeBlockServer language="yaml" filename="curl-runner.yaml">
                {protocolYaml}
              </CodeBlockServer>
              <CodeBlockServer language="bash" filename="CLI & Environment">
                {protocolCli}
              </CodeBlockServer>
            </div>
          </section>

          {/* Path confinement */}
          <section>
            <H2 id="path-confinement">Filesystem Path Confinement</H2>
            <p className="text-muted-foreground text-lg mb-6">
              The <code className="font-mono">output</code> path, the global{' '}
              <code className="font-mono">saveToFile</code> path, and{' '}
              <code className="font-mono">formData</code> file attachments must resolve inside the
              current working directory. Paths that escape via <code className="font-mono">..</code>{' '}
              or an absolute location are blocked unless you opt out. SSL{' '}
              <code className="font-mono">ca</code>/<code className="font-mono">cert</code>/
              <code className="font-mono">key</code> paths are exempt: they are TLS handshake
              material and are never transmitted to the server.
            </p>

            <div className="space-y-4">
              <CodeBlockServer language="yaml" filename="upload.yaml">
                {pathYaml}
              </CodeBlockServer>
              <CodeBlockServer language="bash" filename="CLI & Environment">
                {pathCli}
              </CodeBlockServer>
            </div>
          </section>

          {/* Secret redaction */}
          <section>
            <H2 id="secret-redaction">Secret Redaction</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Credentials are automatically masked as <code className="font-mono">[REDACTED]</code>{' '}
              in console output and in saved result files. This covers{' '}
              <code className="font-mono">auth.token</code> and{' '}
              <code className="font-mono">auth.password</code>, header values whose name looks
              sensitive (authorization, api-key, token, secret, cookie), environment variables
              prefixed with <code className="font-mono">SECRET_</code>, and a set of well-known key
              formats (Stripe, AWS, GitHub, JWTs, and more). Redaction is best-effort, not a
              guarantee — treat it as a safety net, not a reason to commit secrets.
            </p>

            <CodeBlockServer language="yaml" filename="api.yaml">
              {redactionExample}
            </CodeBlockServer>
          </section>

          {/* Breaking change note */}
          <section>
            <H2 id="upgrading">Upgrading</H2>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Recommended</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <div>• Keep the defaults; opt in only for the run that needs it</div>
                      <div>• Prefer config-file scoping over global environment variables</div>
                      <div>• Store credentials in env vars, not inline in committed YAML</div>
                    </div>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <AlertCircle className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Behavior changes</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <div>
                        • Non-http(s) URLs now need{' '}
                        <code className="font-mono">--allow-protocol</code>
                      </div>
                      <div>
                        • Files outside the working dir need{' '}
                        <code className="font-mono">--allow-path</code>
                      </div>
                      <div>
                        • Disable masking with <code className="font-mono">--no-redact</code> if it
                        gets in the way
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
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
