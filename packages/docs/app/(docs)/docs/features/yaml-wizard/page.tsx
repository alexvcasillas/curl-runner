import { CheckCircle, FileEdit, Rocket, Sparkles, Terminal, Wand2 } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';
import {
  advancedOptionsExample,
  editModeExample,
  fullWizardExample,
  quickInitExample,
  runAfterCreateExample,
  templatesExample,
  wizardFlowExample,
  wizardOutputExample,
} from './snippets';

export const metadata: Metadata = {
  title: 'YAML Wizard',
  description:
    'Create curl-runner YAML configuration files interactively with a guided wizard. No need to memorize syntax - just answer prompts.',
  keywords: [
    'curl-runner wizard',
    'yaml generator',
    'configuration wizard',
    'interactive cli',
    'create yaml',
    'init command',
    'edit command',
    'guided setup',
    'templates',
    'quick start',
    'api testing',
    'http requests',
  ],
  openGraph: {
    title: 'YAML Wizard | curl-runner Documentation',
    description: 'Create curl-runner YAML configuration files interactively with a guided wizard.',
    url: 'https://www.curl-runner.com/docs/features/yaml-wizard',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'YAML Wizard | curl-runner Documentation',
    description:
      'Create curl-runner YAML configuration files interactively. No syntax memorization needed.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/yaml-wizard',
  },
};

export default function YamlWizardPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="YAML Wizard"
          text="Create YAML configuration files interactively without memorizing syntax."
        />

        <div className="space-y-8">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground mb-6">
              The YAML Wizard is an interactive CLI that guides you through creating curl-runner
              configuration files. Instead of writing YAML by hand, answer prompts for URL, method,
              headers, body, authentication, and more. The wizard handles all the formatting and
              syntax for you.
            </p>

            <div className="grid gap-4 sm:grid-cols-3">
              <div className="rounded-lg border p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Badge className="bg-fuchsia-500/10 text-fuchsia-600 dark:text-fuchsia-400 border-fuchsia-500/20">
                    Interactive
                  </Badge>
                </div>
                <h4 className="font-medium mb-1">Guided Prompts</h4>
                <p className="text-sm text-muted-foreground">
                  Step-by-step prompts walk you through every option
                </p>
              </div>

              <div className="rounded-lg border p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Badge className="bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20">
                    Templates
                  </Badge>
                </div>
                <h4 className="font-medium mb-1">Pre-built Templates</h4>
                <p className="text-sm text-muted-foreground">
                  Start from templates for common use cases
                </p>
              </div>

              <div className="rounded-lg border p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Badge className="bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20">
                    Edit Mode
                  </Badge>
                </div>
                <h4 className="font-medium mb-1">Modify Existing</h4>
                <p className="text-sm text-muted-foreground">
                  Edit existing YAML files through the wizard
                </p>
              </div>
            </div>
          </section>

          {/* Quick Init */}
          <section>
            <H2 id="quick-init">Quick Init</H2>
            <p className="text-muted-foreground mb-6">
              The fastest way to create a new YAML file. Just run{' '}
              <code className="text-sm bg-muted px-1 py-0.5 rounded">curl-runner init</code> and
              answer a few prompts.
            </p>

            <CodeBlockServer language="bash" filename="Terminal">
              {quickInitExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-muted/50 p-4">
              <h4 className="text-sm font-medium mb-2">Quick init prompts for:</h4>
              <ul className="text-sm text-muted-foreground space-y-1 list-disc list-inside">
                <li>Request URL (or pass as argument)</li>
                <li>HTTP method (GET, POST, PUT, DELETE)</li>
                <li>Output filename</li>
                <li>Whether to run immediately</li>
              </ul>
            </div>
          </section>

          {/* Full Wizard */}
          <section>
            <H2 id="full-wizard">Full Wizard Mode</H2>
            <p className="text-muted-foreground mb-6">
              For more control, use the full wizard with{' '}
              <code className="text-sm bg-muted px-1 py-0.5 rounded">--wizard</code> or{' '}
              <code className="text-sm bg-muted px-1 py-0.5 rounded">-w</code>. This guides you
              through all available options.
            </p>

            <CodeBlockServer language="bash" filename="Terminal">
              {fullWizardExample}
            </CodeBlockServer>

            <div className="mt-6">
              <H3 id="wizard-flow">Wizard Flow</H3>
              <p className="text-muted-foreground mb-4">
                The wizard takes you through these steps:
              </p>

              <CodeBlockServer language="text" filename="Wizard Flow">
                {wizardFlowExample}
              </CodeBlockServer>
            </div>
          </section>

          {/* Templates */}
          <section>
            <H2 id="templates">Templates</H2>
            <p className="text-muted-foreground mb-6">
              Start from a pre-built template to save time. Templates come with sensible defaults
              for common scenarios.
            </p>

            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3 mb-6">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Terminal className="h-4 w-4 text-muted-foreground" />
                  <h4 className="font-medium">Basic GET</h4>
                </div>
                <p className="text-sm text-muted-foreground">
                  Simple GET request with follow redirects enabled
                </p>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Rocket className="h-4 w-4 text-muted-foreground" />
                  <h4 className="font-medium">Basic POST</h4>
                </div>
                <p className="text-sm text-muted-foreground">
                  POST with JSON content-type header preset
                </p>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-center gap-2 mb-2">
                  <CheckCircle className="h-4 w-4 text-muted-foreground" />
                  <h4 className="font-medium">API Test</h4>
                </div>
                <p className="text-sm text-muted-foreground">
                  Includes validation, response time checks, verbose output
                </p>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Sparkles className="h-4 w-4 text-muted-foreground" />
                  <h4 className="font-medium">File Upload</h4>
                </div>
                <p className="text-sm text-muted-foreground">
                  Multipart form data with file attachment support
                </p>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Wand2 className="h-4 w-4 text-muted-foreground" />
                  <h4 className="font-medium">Auth Flow</h4>
                </div>
                <p className="text-sm text-muted-foreground">
                  Bearer token authentication pre-configured
                </p>
              </div>
            </div>

            <CodeBlockServer language="bash" filename="templates-info.txt">
              {templatesExample}
            </CodeBlockServer>
          </section>

          {/* Edit Mode */}
          <section>
            <H2 id="edit-mode">Edit Existing Files</H2>
            <p className="text-muted-foreground mb-6">
              Use the <code className="text-sm bg-muted px-1 py-0.5 rounded">edit</code> command to
              modify existing YAML files through the wizard. The current values are pre-filled so
              you can quickly make changes.
            </p>

            <CodeBlockServer language="bash" filename="Terminal">
              {editModeExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-blue-500/5 dark:bg-blue-500/10 border-blue-500/20 p-4">
              <div className="flex items-start gap-3">
                <FileEdit className="h-5 w-5 text-blue-600 dark:text-blue-400 mt-0.5" />
                <div>
                  <h4 className="font-medium text-blue-600 dark:text-blue-400 mb-1">
                    Edit Mode Features
                  </h4>
                  <ul className="text-sm text-muted-foreground space-y-1 list-disc list-inside">
                    <li>Pre-fills all prompts with existing values</li>
                    <li>Skip through unchanged options quickly</li>
                    <li>Save to original or new file</li>
                    <li>Preview changes before saving</li>
                  </ul>
                </div>
              </div>
            </div>
          </section>

          {/* Advanced Options */}
          <section>
            <H2 id="advanced-options">Advanced Options</H2>
            <p className="text-muted-foreground mb-6">
              The wizard prompts for advanced options when you select "Configure advanced options".
              These include timeout, retry, SSL, and validation settings.
            </p>

            <CodeBlockServer language="text" filename="Advanced Options">
              {advancedOptionsExample}
            </CodeBlockServer>
          </section>

          {/* Preview and Run */}
          <section>
            <H2 id="preview-and-run">Preview and Run</H2>
            <p className="text-muted-foreground mb-6">
              Before saving, the wizard shows a preview of your YAML configuration. You can then
              choose to run the request immediately to verify it works.
            </p>

            <CodeBlockServer language="text" filename="Preview">
              {runAfterCreateExample}
            </CodeBlockServer>
          </section>

          {/* Generated Output */}
          <section>
            <H2 id="output">Generated Output</H2>
            <p className="text-muted-foreground mb-6">
              The wizard generates clean, well-formatted YAML that follows curl-runner conventions.
            </p>

            <CodeBlockServer language="yaml" filename="generated-config.yaml">
              {wizardOutputExample}
            </CodeBlockServer>
          </section>

          {/* CLI Reference */}
          <section>
            <H2 id="cli-reference">CLI Reference</H2>

            <div className="border rounded-lg overflow-hidden">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Command</th>
                    <th className="text-left p-3 font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">curl-runner init</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Quick create with minimal prompts
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">curl-runner init [url]</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Quick create with URL pre-filled
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">curl-runner init --wizard</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">Full interactive wizard</td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">curl-runner init -w -o file.yaml</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Wizard with custom output file
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">curl-runner edit file.yaml</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">Edit existing YAML file</td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">curl-runner edit file.yaml -o new.yaml</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">Edit and save to new file</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Tips */}
          <section>
            <H2 id="tips">Tips</H2>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Use Templates</h4>
                    <p className="text-sm text-muted-foreground">
                      Start from a template when possible. It sets sensible defaults and saves time.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Rocket className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Run After Create</h4>
                    <p className="text-sm text-muted-foreground">
                      Always run after creating to verify your configuration works as expected.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <FileEdit className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Edit Mode for Updates</h4>
                    <p className="text-sm text-muted-foreground">
                      Use edit mode instead of manual YAML editing to avoid syntax errors.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <Terminal className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Quick Init for Simple Cases</h4>
                    <p className="text-sm text-muted-foreground">
                      For simple GET/POST requests, quick init is faster than the full wizard.
                    </p>
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
