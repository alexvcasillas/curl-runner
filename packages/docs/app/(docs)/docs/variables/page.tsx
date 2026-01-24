import { AlertTriangle, CheckCircle, Info } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';

export const metadata: Metadata = {
  title: 'Variables & Templating',
  description:
    'Master variable definition and templating in curl-runner. Learn about environment variables, computed values, dynamic content generation, and template expressions.',
  keywords: [
    'curl-runner variables',
    'YAML variables',
    'template expressions',
    'environment variables',
    'dynamic values',
    'computed variables',
    'variable interpolation',
    'templating system',
    'configuration variables',
    'request templating',
  ],
  openGraph: {
    title: 'Variables & Templating | curl-runner Documentation',
    description:
      'Master variable definition and templating in curl-runner. Learn about environment variables, computed values, dynamic content generation, and template expressions.',
    url: 'https://www.curl-runner.com/docs/variables',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Variables & Templating | curl-runner Documentation',
    description:
      'Master variable definition and templating in curl-runner. Learn about environment variables and dynamic content generation.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/variables',
  },
};

import {
  basicVariablesExample,
  computedVariablesExample,
  conditionalVariablesExample,
  dynamicVariablesExample,
  environmentVariablesExample,
  variablePrecedenceExample,
} from './snippets';

export default function VariablesPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Variables"
          text="Use variables and templating to create reusable, dynamic HTTP request configurations."
        />

        <div className="space-y-12">
          {/* Introduction */}
          <section>
            <p className="text-lg text-muted-foreground mb-6">
              Variables in curl-runner allow you to create flexible, reusable configurations. You
              can define variables at different scopes and use them throughout your request
              definitions with template interpolation.
            </p>

            <div className="rounded-lg border bg-card p-4">
              <div className="flex items-start gap-3">
                <div className="rounded-full bg-blue-500/10 p-2">
                  <Info className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                </div>
                <div className="flex-1 pt-1">
                  <h4 className="font-medium mb-2">Variable Interpolation Syntax</h4>
                  <p className="text-sm text-muted-foreground">
                    Variables are interpolated using the{' '}
                    <code className="bg-primary/20 text-primary px-2 py-1 rounded text-sm font-mono">
                      ${`{VARIABLE_NAME}`}
                    </code>{' '}
                    syntax. You can also use default values with{' '}
                    <code className="bg-primary/20 text-primary px-2 py-1 rounded text-sm font-mono">
                      ${`{VAR:default}`}
                    </code>
                    .
                  </p>
                </div>
              </div>
            </div>
          </section>

          {/* Variable Scopes */}
          <section>
            <H2 id="variable-scopes">Variable Scopes</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Variables can be defined at different levels, each with its own scope and precedence
              rules.
            </p>

            <div className="space-y-8">
              <div>
                <H3 id="global-variables">Global Variables</H3>
                <p className="text-muted-foreground mb-4">
                  Global variables are available to all requests in the file and have the lowest
                  precedence.
                </p>
                <CodeBlockServer language="yaml" filename="global-variables.yaml">
                  {basicVariablesExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3 id="collection-variables">Collection Variables</H3>
                <p className="text-muted-foreground mb-4">
                  Collection variables are scoped to a specific collection and override global
                  variables.
                </p>
                <CodeBlockServer language="yaml" filename="collection-variables.yaml">
                  {variablePrecedenceExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3 id="environment-variables">Environment Variables</H3>
                <p className="text-muted-foreground mb-4">
                  Access system environment variables using the <code>ENV</code> object.
                </p>
                <CodeBlockServer language="yaml" filename="env-variables.yaml">
                  {environmentVariablesExample}
                </CodeBlockServer>
              </div>
            </div>
          </section>

          {/* Variable Precedence */}
          <section>
            <H2 id="variable-precedence">Variable Precedence</H2>
            <p className="text-muted-foreground text-lg mb-6">
              When variables are defined at multiple levels, curl-runner follows a specific
              precedence order.
            </p>

            <div className="rounded-lg border bg-card p-4 mb-6">
              <div className="flex items-start gap-3">
                <div className="rounded-full bg-purple-500/10 p-2">
                  <AlertTriangle className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                </div>
                <div className="flex-1 pt-1">
                  <h4 className="font-medium mb-4">Precedence Order (Highest to Lowest)</h4>
                  <div className="space-y-3">
                    <div className="flex items-center space-x-3">
                      <Badge variant="default" className="bg-primary">
                        1
                      </Badge>
                      <span className="font-medium">Environment Variables</span>
                      <code className="text-sm bg-primary/20 text-primary px-2 py-1 rounded font-mono">
                        ${`{ENV.VAR_NAME}`}
                      </code>
                    </div>
                    <div className="flex items-center space-x-3">
                      <Badge variant="secondary">2</Badge>
                      <span className="font-medium">Collection Variables</span>
                      <code className="text-sm bg-muted px-2 py-1 rounded font-mono">
                        collection.variables
                      </code>
                    </div>
                    <div className="flex items-center space-x-3">
                      <Badge variant="outline">3</Badge>
                      <span className="font-medium">Global Variables</span>
                      <code className="text-sm bg-muted px-2 py-1 rounded font-mono">
                        global.variables
                      </code>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <CodeBlockServer language="yaml" filename="variable-precedence.yaml">
              {variablePrecedenceExample}
            </CodeBlockServer>
          </section>

          {/* Dynamic Variables */}
          <section>
            <H2 id="dynamic-variables">Dynamic Variables</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Create dynamic values using built-in functions.
            </p>

            <div className="space-y-6">
              <H3 id="built-in-functions">Built-in Functions</H3>
              <div className="grid gap-4 sm:grid-cols-2 mb-6">
                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-orange-500/10 p-2">
                      <Info className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                    </div>
                    <div className="flex-1 pt-1">
                      <h4 className="font-medium mb-3">Date & Time</h4>
                      <div className="space-y-2 text-sm">
                        <div>
                          <code className="bg-primary/10 text-primary px-2 py-1 rounded font-mono">
                            ${`{DATE:YYYY-MM-DD}`}
                          </code>{' '}
                          - Formatted date
                        </div>
                        <div>
                          <code className="bg-primary/10 text-primary px-2 py-1 rounded font-mono">
                            ${`{TIME:HH:mm:ss}`}
                          </code>{' '}
                          - Formatted time
                        </div>
                        <div>
                          <code className="bg-primary/10 text-primary px-2 py-1 rounded font-mono">
                            ${`{TIMESTAMP}`}
                          </code>{' '}
                          - Unix timestamp
                        </div>
                      </div>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <div className="rounded-full bg-cyan-500/10 p-2">
                      <Info className="h-4 w-4 text-cyan-600 dark:text-cyan-400" />
                    </div>
                    <div className="flex-1 pt-1">
                      <h4 className="font-medium mb-3">Random Values</h4>
                      <div className="space-y-2 text-sm">
                        <div>
                          <code className="bg-primary/10 text-primary px-2 py-1 rounded font-mono">
                            ${`{UUID}`}
                          </code>{' '}
                          - UUID v4
                        </div>
                        <div>
                          <code className="bg-primary/10 text-primary px-2 py-1 rounded font-mono">
                            ${`{UUID:short}`}
                          </code>{' '}
                          - Short UUID
                        </div>
                        <div>
                          <code className="bg-primary/10 text-primary px-2 py-1 rounded font-mono">
                            ${`{RANDOM:1-1000}`}
                          </code>{' '}
                          - Random integer
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              <CodeBlockServer language="yaml" filename="dynamic-variables.yaml">
                {dynamicVariablesExample}
              </CodeBlockServer>
            </div>
          </section>

          {/* Default Values */}
          <section>
            <H2 id="default-values">Default Values</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Use default value syntax to provide fallback values when variables are not set.
            </p>

            <CodeBlockServer language="yaml" filename="default-values.yaml">
              {conditionalVariablesExample}
            </CodeBlockServer>

            <p className="text-muted-foreground mt-4">
              For environment-specific configurations, use separate YAML files per environment or set
              environment variables directly.
            </p>
          </section>

          {/* Complex Interpolation */}
          <section>
            <H2 id="complex-interpolation">Complex Interpolation</H2>
            <p className="text-muted-foreground text-lg mb-6">
              Combine multiple variables and expressions to create complex, computed values.
            </p>

            <CodeBlockServer language="yaml" filename="complex-interpolation.yaml">
              {computedVariablesExample}
            </CodeBlockServer>
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
                  <div className="flex-1 pt-1">
                    <h4 className="font-medium mb-3">Best Practices</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <p>• Use descriptive variable names</p>
                      <p>• Define common values as variables</p>
                      <p>• Use environment variables for secrets</p>
                      <p>• Group related variables logically</p>
                      <p>• Document complex expressions</p>
                    </div>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-amber-500/10 p-2">
                    <AlertTriangle className="h-4 w-4 text-amber-600 dark:text-amber-400" />
                  </div>
                  <div className="flex-1 pt-1">
                    <h4 className="font-medium mb-3">Avoid These Mistakes</h4>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <p>• Hard-coding sensitive information</p>
                      <p>• Using overly complex expressions</p>
                      <p>• Creating circular references</p>
                      <p>• Overusing dynamic variables</p>
                      <p>• Ignoring variable naming conventions</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Common Patterns */}
          <section>
            <H2 id="common-patterns">Common Patterns</H2>

            <div className="space-y-6">
              <H3 id="api-authentication">API Authentication</H3>
              <CodeBlockServer language="yaml" title="API Authentication Pattern">
                {`global:
  variables:
    # Use environment variables for configuration
    BASE_URL: "\${API_BASE_URL:https://api-staging.example.com}"
    API_KEY: "\${API_KEY}"

collection:
  requests:
    - name: "Authenticated request"
      url: "\${BASE_URL}/users"
      headers:
        Authorization: "Bearer \${API_KEY}"`}
              </CodeBlockServer>

              <H3 id="request-correlation-ids">Request Correlation IDs</H3>
              <CodeBlockServer language="yaml" title="Request Correlation IDs">
                {`global:
  variables:
    CORRELATION_ID: "\${UUID}"

  defaults:
    headers:
      X-Correlation-ID: "\${CORRELATION_ID}"
      X-Request-Time: "\${TIMESTAMP}"`}
              </CodeBlockServer>
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
