import { AlertCircle, ArrowLeftRight, CheckCircle, Code, FileText, Terminal } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'Curl ⇄ YAML Conversion',
  description:
    'Convert curl commands to curl-runner YAML and back. Paste a curl command, get clean YAML. Convert YAML specs to canonical curl commands.',
  keywords: [
    'curl to yaml',
    'yaml to curl',
    'curl converter',
    'curl import',
    'postman import',
    'curl-runner convert',
    'curl command parser',
    'shell script conversion',
    'batch conversion',
    'API migration',
  ],
  openGraph: {
    title: 'Curl ⇄ YAML Conversion | curl-runner Documentation',
    description:
      'Convert curl commands to curl-runner YAML and back. Paste a curl command, get clean YAML.',
    url: 'https://www.curl-runner.com/docs/features/convert',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Curl ⇄ YAML Conversion | curl-runner Documentation',
    description: 'Bidirectional conversion between curl commands and curl-runner YAML.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/convert',
  },
};

const curlToYamlExample = `# Convert a curl command to YAML
curl-runner convert curl "curl -X POST https://api.example.com/users \\
  -H 'Authorization: Bearer TOKEN' \\
  -H 'Content-Type: application/json' \\
  -d '{\\"name\\":\\"Alex\\",\\"role\\":\\"admin\\"}'"`;

const curlToYamlOutput = `request:
  method: POST
  url: https://api.example.com/users
  auth:
    type: bearer
    token: TOKEN
  body:
    json:
      name: Alex
      role: admin`;

const yamlToCurlExample = `# Convert a YAML file back to curl
curl-runner convert yaml api-test.yaml`;

const yamlToCurlOutput = `curl \\
  -X POST \\
  -H 'Authorization: Bearer TOKEN' \\
  -d '{"name":"Alex","role":"admin"}' \\
  -H 'Content-Type: application/json' \\
  https://api.example.com/users`;

const batchExample = `# Convert all curl commands from a shell script
curl-runner convert file api-tests.sh`;

const batchScriptExample = `#!/bin/bash
# api-tests.sh

# Health check
curl https://api.example.com/health

# Create user
curl -X POST https://api.example.com/users \\
  -H "Content-Type: application/json" \\
  -d '{"name":"Alex"}'

# Get users
curl -H "Authorization: Bearer TOKEN" \\
  https://api.example.com/users`;

const batchOutput = `requests:
  - name: request_1
    method: GET
    url: https://api.example.com/health

  - name: request_2
    method: POST
    url: https://api.example.com/users
    body:
      json:
        name: Alex

  - name: request_3
    method: GET
    url: https://api.example.com/users
    auth:
      type: bearer
      token: TOKEN`;

const saveExample = `# Save conversion output to a file
curl-runner convert curl "curl https://api.example.com/users" -o test.yaml

# Then run it immediately
curl-runner test.yaml`;

const debugExample = `# Debug mode shows the full conversion pipeline
curl-runner convert curl "curl -u admin:secret https://api.example.com" --debug`;

const debugOutput = `DEBUG:
{
  "tokens": ["curl", "-u", "admin:secret", "https://api.example.com"],
  "ast": {
    "url": "https://api.example.com",
    "headers": [],
    "user": "admin:secret",
    "unsupportedFlags": []
  },
  "ir": {
    "method": "GET",
    "url": "https://api.example.com",
    "headers": {},
    "auth": { "type": "basic", "username": "admin", "password": "secret" },
    "metadata": { "source": "curl", "warnings": [] }
  }
}

request:
  method: GET
  url: https://api.example.com
  auth:
    type: basic
    username: admin
    password: secret`;

const lossExample = `# Unsupported flags produce warnings
curl-runner convert curl "curl --compressed --http2-prior-knowledge https://api.example.com"`;

const lossOutput = `# Warning: Flag --compressed has no YAML equivalent; curl handles decompression natively
# Warning: Unsupported curl flag: --http2-prior-knowledge
request:
  method: GET
  url: https://api.example.com`;

const complexExample = `# Real-world: Stripe API call
curl-runner convert curl "curl -X POST https://api.stripe.com/v1/charges \\
  -u sk_test_key: \\
  -d amount=2000 \\
  -d currency=usd \\
  -d source=tok_visa"`;

const complexOutput = `request:
  method: POST
  url: https://api.stripe.com/v1/charges
  auth:
    type: basic
    username: sk_test_key
    password: ""
  body:
    form: amount=2000&currency=usd&source=tok_visa`;

export default function ConvertPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Curl ⇄ YAML Conversion"
          text="Bidirectional conversion between raw curl commands and curl-runner YAML specs."
        />

        <div className="space-y-12">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground text-lg mb-6">
              The convert command provides a deterministic, loss-aware conversion engine between curl
              commands and curl-runner YAML. Paste a curl command from documentation or browser
              DevTools, get a clean YAML spec. Convert YAML specs back to canonical curl commands.
            </p>

            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-cyan-500/10 p-2">
                    <ArrowLeftRight className="h-4 w-4 text-cyan-600 dark:text-cyan-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Bidirectional</h4>
                    <p className="text-sm text-muted-foreground">
                      curl &rarr; YAML and YAML &rarr; curl
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <FileText className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Batch Scripts</h4>
                    <p className="text-sm text-muted-foreground">
                      Convert entire shell scripts at once
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Code className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-1">Loss-Aware</h4>
                    <p className="text-sm text-muted-foreground">
                      Warns about unsupported features
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Curl to YAML */}
          <section>
            <H2 id="curl-to-yaml">Curl to YAML</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Convert a raw curl command into a clean, human-editable curl-runner YAML spec.
              The converter automatically detects JSON bodies, authentication, query parameters,
              and more.
            </p>

            <div className="space-y-4">
              <CodeBlockServer language="bash" filename="Command">
                {curlToYamlExample}
              </CodeBlockServer>

              <CodeBlockServer language="yaml" filename="Output">
                {curlToYamlOutput}
              </CodeBlockServer>
            </div>
          </section>

          {/* YAML to Curl */}
          <section>
            <H2 id="yaml-to-curl">YAML to Curl</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Convert a curl-runner YAML file back to canonical, shell-safe curl commands.
              Useful for sharing commands with teammates or debugging.
            </p>

            <div className="space-y-4">
              <CodeBlockServer language="bash" filename="Command">
                {yamlToCurlExample}
              </CodeBlockServer>

              <CodeBlockServer language="bash" filename="Output">
                {yamlToCurlOutput}
              </CodeBlockServer>
            </div>
          </section>

          {/* Batch Conversion */}
          <section>
            <H2 id="batch-conversion">Batch Script Conversion</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Extract and convert all curl commands from a shell script into a single YAML collection.
              The parser handles multiline commands, ignores non-curl lines, and strips piped output.
            </p>

            <div className="space-y-4">
              <CodeBlockServer language="bash" filename="api-tests.sh">
                {batchScriptExample}
              </CodeBlockServer>

              <CodeBlockServer language="bash" filename="Command">
                {batchExample}
              </CodeBlockServer>

              <CodeBlockServer language="yaml" filename="Output">
                {batchOutput}
              </CodeBlockServer>
            </div>
          </section>

          {/* Conversion Features */}
          <section>
            <H2 id="detection">Automatic Detection</H2>
            <p className="text-muted-foreground text-lg mb-6">
              The converter automatically normalizes and detects semantic intent from curl flags.
            </p>

            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b">
                    <th className="py-3 px-4 text-left font-medium">Curl Input</th>
                    <th className="py-3 px-4 text-left font-medium">YAML Output</th>
                  </tr>
                </thead>
                <tbody className="divide-y">
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">-d &apos;{`{"key":"val"}`}&apos;</code></td>
                    <td className="py-3 px-4"><code className="font-mono text-xs">body.json</code> + infers POST</td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">-d &apos;key=val&amp;key2=val2&apos;</code></td>
                    <td className="py-3 px-4"><code className="font-mono text-xs">body.form</code> (urlencoded)</td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">-H &apos;Authorization: Bearer T&apos;</code></td>
                    <td className="py-3 px-4"><code className="font-mono text-xs">auth.type: bearer</code></td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">-u user:pass</code></td>
                    <td className="py-3 px-4"><code className="font-mono text-xs">auth.type: basic</code></td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">-F &apos;file=@img.png&apos;</code></td>
                    <td className="py-3 px-4"><code className="font-mono text-xs">formData.file.file: img.png</code></td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">URL?q=test&amp;page=1</code></td>
                    <td className="py-3 px-4"><code className="font-mono text-xs">params: {`{q: test, page: 1}`}</code></td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">--max-time 30</code></td>
                    <td className="py-3 px-4"><code className="font-mono text-xs">timeout: 30000</code> (ms)</td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">-A &apos;MyApp/1.0&apos;</code></td>
                    <td className="py-3 px-4"><code className="font-mono text-xs">headers.User-Agent</code></td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Save to File */}
          <section>
            <H2 id="save-to-file">Save to File</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Write conversion output directly to a file with <code className="font-mono">-o</code>.
              The output is immediately usable by curl-runner.
            </p>

            <CodeBlockServer language="bash" filename="Save and Run">
              {saveExample}
            </CodeBlockServer>
          </section>

          {/* Real-World Example */}
          <section>
            <H2 id="real-world">Real-World Example</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Convert complex real-world curl commands with authentication, form data, and more.
            </p>

            <div className="space-y-4">
              <CodeBlockServer language="bash" filename="Stripe API">
                {complexExample}
              </CodeBlockServer>

              <CodeBlockServer language="yaml" filename="Output">
                {complexOutput}
              </CodeBlockServer>
            </div>
          </section>

          {/* Loss Handling */}
          <section>
            <H2 id="loss-handling">Loss Handling</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Some curl flags don&apos;t have YAML equivalents. The converter emits YAML comments
              and warnings so you know exactly what was lost.
            </p>

            <div className="space-y-4">
              <CodeBlockServer language="bash" filename="Command">
                {lossExample}
              </CodeBlockServer>

              <CodeBlockServer language="yaml" filename="Output with Warnings">
                {lossOutput}
              </CodeBlockServer>
            </div>
          </section>

          {/* Debug Mode */}
          <section>
            <H2 id="debug-mode">Debug Mode</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Use <code className="font-mono">--debug</code> to inspect the full conversion
              pipeline: token stream, AST, and intermediate representation (IR).
            </p>

            <div className="space-y-4">
              <CodeBlockServer language="bash" filename="Command">
                {debugExample}
              </CodeBlockServer>

              <CodeBlockServer language="json" filename="Debug Output">
                {debugOutput}
              </CodeBlockServer>
            </div>
          </section>

          {/* CLI Reference */}
          <section>
            <H2 id="cli-reference">CLI Reference</H2>

            <div className="overflow-x-auto mb-6">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b">
                    <th className="py-3 px-4 text-left font-medium">Command</th>
                    <th className="py-3 px-4 text-left font-medium">Description</th>
                  </tr>
                </thead>
                <tbody className="divide-y">
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">convert curl &quot;&lt;cmd&gt;&quot;</code></td>
                    <td className="py-3 px-4">Convert inline curl command to YAML</td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">convert file &lt;script.sh&gt;</code></td>
                    <td className="py-3 px-4">Convert shell script to YAML (batch)</td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">convert yaml &lt;file.yaml&gt;</code></td>
                    <td className="py-3 px-4">Convert YAML to curl command(s)</td>
                  </tr>
                </tbody>
              </table>
            </div>

            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b">
                    <th className="py-3 px-4 text-left font-medium">Flag</th>
                    <th className="py-3 px-4 text-left font-medium">Description</th>
                  </tr>
                </thead>
                <tbody className="divide-y">
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">-o, --output &lt;file&gt;</code></td>
                    <td className="py-3 px-4">Write output to file</td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">--pretty</code></td>
                    <td className="py-3 px-4">Pretty-print output (default)</td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">--loss-report</code></td>
                    <td className="py-3 px-4">Include warnings for unsupported features (default)</td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4"><code className="font-mono text-xs">--debug</code></td>
                    <td className="py-3 px-4">Show token stream, AST, and IR</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Best Practices */}
          <section>
            <H2 id="best-practices">Best Practices</H2>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Recommended</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <div>
                        &bull; Use <code className="font-mono">-o</code> to save and immediately run converted YAML
                      </div>
                      <div>&bull; Review warnings for any lost curl features</div>
                      <div>&bull; Use batch conversion for migrating shell scripts</div>
                      <div>&bull; Use debug mode to understand complex conversions</div>
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
                    <h4 className="font-medium mb-2">Notes</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <div>&bull; Shell-specific features (pipes, variables) are stripped</div>
                      <div>&bull; Some curl flags have no YAML equivalent (see warnings)</div>
                      <div>&bull; YAML-only features (expect, store, when) can&apos;t map to curl</div>
                      <div>&bull; Round-trip preserves semantics, not exact syntax</div>
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
