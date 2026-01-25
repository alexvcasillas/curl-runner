import { AlertTriangle, CheckCircle, GitBranch, Sparkles, Workflow, Zap } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';
import {
  allConditionsExample,
  anyConditionsExample,
  authFlowExample,
  basicWhenExample,
  caseSensitiveExample,
  errorHandlingExample,
  operatorsReference,
  stringShorthandExample,
  workflowExample,
} from './snippets';

export const metadata: Metadata = {
  title: 'Conditional Execution',
  description:
    'Skip or run requests based on previous results. Build dynamic API workflows with conditional request chaining.',
  keywords: [
    'curl-runner conditional execution',
    'conditional requests',
    'request chaining',
    'skip requests',
    'API workflow branching',
    'when condition',
    'dynamic API testing',
    'conditional API calls',
    'request conditions',
    'API test branching',
  ],
  openGraph: {
    title: 'Conditional Execution | curl-runner Documentation',
    description:
      'Skip or run requests based on previous results. Build dynamic API workflows with conditional request chaining.',
    url: 'https://www.curl-runner.com/docs/features/conditional-execution',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Conditional Execution | curl-runner Documentation',
    description:
      'Learn how to build dynamic API workflows with conditional request chaining in curl-runner.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/conditional-execution',
  },
};

export default function ConditionalExecutionPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Conditional Execution"
          text="Skip or run requests based on previous results."
        />

        <div className="space-y-8">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground mb-6">
              Conditional execution allows you to control which requests run based on the results of
              previous requests. Use the <code className="text-sm bg-muted px-1 py-0.5 rounded">when</code> field
              to define conditions that must be met for a request to execute.
            </p>

            <div className="grid gap-4 sm:grid-cols-2">
              <div className="rounded-lg border p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Badge className="bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20">
                    Flexible
                  </Badge>
                </div>
                <h4 className="font-medium mb-1">Multiple Operators</h4>
                <p className="text-sm text-muted-foreground">
                  Compare values with ==, !=, &gt;, &lt;, contains, matches, exists, and more
                </p>
              </div>

              <div className="rounded-lg border p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Badge className="bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20">
                    Powerful
                  </Badge>
                </div>
                <h4 className="font-medium mb-1">Compound Conditions</h4>
                <p className="text-sm text-muted-foreground">
                  Combine conditions with AND (all) and OR (any) logic
                </p>
              </div>
            </div>
          </section>

          {/* Basic Usage */}
          <section>
            <H2 id="basic-usage">Basic Usage</H2>
            <p className="text-muted-foreground mb-6">
              Add a <code className="text-sm bg-muted px-1 py-0.5 rounded">when</code> field to any
              request to make it conditional. The request will only execute if the condition
              evaluates to true.
            </p>

            <CodeBlockServer language="yaml" filename="basic-when.yaml">
              {basicWhenExample}
            </CodeBlockServer>

            <div className="mt-6 rounded-lg border bg-muted/50 p-4">
              <h4 className="text-sm font-medium mb-2">How it works:</h4>
              <ol className="text-sm text-muted-foreground space-y-1 list-decimal list-inside">
                <li>First request executes and stores values</li>
                <li>Before each subsequent request, the <code>when</code> condition is evaluated</li>
                <li>If condition is true, request executes normally</li>
                <li>If condition is false, request is skipped (marked as skipped, not failed)</li>
              </ol>
            </div>
          </section>

          {/* String Shorthand */}
          <section>
            <H2 id="string-shorthand">String Shorthand Syntax</H2>
            <p className="text-muted-foreground mb-6">
              For simple conditions, use the string shorthand syntax instead of the full object
              format. This makes your YAML more readable.
            </p>

            <CodeBlockServer language="yaml" filename="shorthand.yaml">
              {stringShorthandExample}
            </CodeBlockServer>

            <div className="mt-6 border rounded-lg overflow-hidden">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Shorthand</th>
                    <th className="text-left p-3 font-medium">Equivalent Object</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">"store.status == 200"</code></td>
                    <td className="p-3 text-sm text-muted-foreground">
                      <code>left: store.status, operator: "==", right: 200</code>
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">"store.userId exists"</code></td>
                    <td className="p-3 text-sm text-muted-foreground">
                      <code>left: store.userId, operator: exists</code>
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">"store.count &gt;= 10"</code></td>
                    <td className="p-3 text-sm text-muted-foreground">
                      <code>left: store.count, operator: "&gt;=", right: 10</code>
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3"><code className="text-sm">"store.name contains admin"</code></td>
                    <td className="p-3 text-sm text-muted-foreground">
                      <code>left: store.name, operator: contains, right: "admin"</code>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Operators */}
          <section>
            <H2 id="operators">Operators</H2>
            <p className="text-muted-foreground mb-6">
              All available operators for building conditions.
            </p>

            <div className="border rounded-lg overflow-hidden mb-6">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Operator</th>
                    <th className="text-left p-3 font-medium">Description</th>
                    <th className="text-left p-3 font-medium">Example</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">==</code></td>
                    <td className="p-3 text-sm text-muted-foreground">Equal to</td>
                    <td className="p-3 text-sm"><code>store.status == 200</code></td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">!=</code></td>
                    <td className="p-3 text-sm text-muted-foreground">Not equal to</td>
                    <td className="p-3 text-sm"><code>store.status != 404</code></td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">&gt;</code></td>
                    <td className="p-3 text-sm text-muted-foreground">Greater than</td>
                    <td className="p-3 text-sm"><code>store.count &gt; 0</code></td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">&lt;</code></td>
                    <td className="p-3 text-sm text-muted-foreground">Less than</td>
                    <td className="p-3 text-sm"><code>store.count &lt; 100</code></td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">&gt;=</code></td>
                    <td className="p-3 text-sm text-muted-foreground">Greater than or equal</td>
                    <td className="p-3 text-sm"><code>store.version &gt;= 2</code></td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">&lt;=</code></td>
                    <td className="p-3 text-sm text-muted-foreground">Less than or equal</td>
                    <td className="p-3 text-sm"><code>store.count &lt;= 50</code></td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">contains</code></td>
                    <td className="p-3 text-sm text-muted-foreground">String contains substring</td>
                    <td className="p-3 text-sm"><code>store.name contains "admin"</code></td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">matches</code></td>
                    <td className="p-3 text-sm text-muted-foreground">Regex pattern match</td>
                    <td className="p-3 text-sm"><code>store.email matches "^.+@.+"</code></td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3"><code className="text-sm">exists</code></td>
                    <td className="p-3 text-sm text-muted-foreground">Value exists and not empty</td>
                    <td className="p-3 text-sm"><code>store.token exists</code></td>
                  </tr>
                  <tr>
                    <td className="p-3"><code className="text-sm">not-exists</code></td>
                    <td className="p-3 text-sm text-muted-foreground">Value missing or empty</td>
                    <td className="p-3 text-sm"><code>store.error not-exists</code></td>
                  </tr>
                </tbody>
              </table>
            </div>

            <CodeBlockServer language="yaml" filename="operators.yaml">
              {operatorsReference}
            </CodeBlockServer>
          </section>

          {/* Compound Conditions */}
          <section>
            <H2 id="compound-conditions">Compound Conditions</H2>

            <div className="space-y-6">
              <div>
                <H3 id="all-conditions">AND Logic (all)</H3>
                <p className="text-muted-foreground mb-4">
                  Use <code className="text-sm bg-muted px-1 py-0.5 rounded">all</code> when every
                  condition must be true. Evaluation stops on the first false condition (short-circuit).
                </p>
                <CodeBlockServer language="yaml" filename="all-conditions.yaml">
                  {allConditionsExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3 id="any-conditions">OR Logic (any)</H3>
                <p className="text-muted-foreground mb-4">
                  Use <code className="text-sm bg-muted px-1 py-0.5 rounded">any</code> when at least
                  one condition must be true. Evaluation stops on the first true condition (short-circuit).
                </p>
                <CodeBlockServer language="yaml" filename="any-conditions.yaml">
                  {anyConditionsExample}
                </CodeBlockServer>
              </div>
            </div>
          </section>

          {/* Case Sensitivity */}
          <section>
            <H2 id="case-sensitivity">Case Sensitivity</H2>
            <p className="text-muted-foreground mb-6">
              String comparisons are case-insensitive by default. Use{' '}
              <code className="text-sm bg-muted px-1 py-0.5 rounded">caseSensitive: true</code> for
              exact matching.
            </p>

            <CodeBlockServer language="yaml" filename="case-sensitive.yaml">
              {caseSensitiveExample}
            </CodeBlockServer>
          </section>

          {/* Use Cases */}
          <section>
            <H2 id="use-cases">Common Use Cases</H2>

            <div className="space-y-6">
              <div>
                <H3 id="auth-flow">Authentication Flow</H3>
                <p className="text-muted-foreground mb-4">
                  Execute authenticated requests only when login succeeds.
                </p>
                <CodeBlockServer language="yaml" filename="auth-flow.yaml">
                  {authFlowExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3 id="error-handling">Error Handling</H3>
                <p className="text-muted-foreground mb-4">
                  Branch execution based on success or error responses.
                </p>
                <CodeBlockServer language="yaml" filename="error-handling.yaml">
                  {errorHandlingExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3 id="workflow-branching">Workflow Branching</H3>
                <p className="text-muted-foreground mb-4">
                  Build dynamic workflows with feature flags and rollout percentages.
                </p>
                <CodeBlockServer language="yaml" filename="workflow.yaml">
                  {workflowExample}
                </CodeBlockServer>
              </div>
            </div>
          </section>

          {/* Important Notes */}
          <section>
            <H2 id="important-notes">Important Notes</H2>

            <div className="space-y-4">
              <div className="rounded-lg border bg-yellow-500/5 dark:bg-yellow-500/10 border-yellow-500/20 p-4">
                <div className="flex items-start gap-3">
                  <AlertTriangle className="h-5 w-5 text-yellow-600 dark:text-yellow-400 mt-0.5" />
                  <div>
                    <h4 className="font-medium text-yellow-600 dark:text-yellow-400 mb-1">
                      Sequential Execution Required
                    </h4>
                    <p className="text-sm text-muted-foreground">
                      Conditional execution only works in sequential mode (the default). In parallel
                      mode, request order is not guaranteed, so conditions based on previous
                      responses won't work as expected.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-muted/50 p-4">
                <div className="flex items-start gap-3">
                  <CheckCircle className="h-5 w-5 text-green-600 dark:text-green-400 mt-0.5" />
                  <div>
                    <h4 className="font-medium mb-1">Skipped Requests Don't Fail</h4>
                    <p className="text-sm text-muted-foreground">
                      When a condition evaluates to false, the request is skipped and marked as
                      successful. Skipped requests don't count toward failure thresholds in CI mode.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-muted/50 p-4">
                <div className="flex items-start gap-3">
                  <Zap className="h-5 w-5 text-blue-600 dark:text-blue-400 mt-0.5" />
                  <div>
                    <h4 className="font-medium mb-1">Short-Circuit Evaluation</h4>
                    <p className="text-sm text-muted-foreground">
                      For <code>all</code> conditions, evaluation stops at the first false. For{' '}
                      <code>any</code> conditions, evaluation stops at the first true. This improves
                      performance for complex conditions.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-muted/50 p-4">
                <div className="flex items-start gap-3">
                  <Sparkles className="h-5 w-5 text-purple-600 dark:text-purple-400 mt-0.5" />
                  <div>
                    <h4 className="font-medium mb-1">Variable Interpolation</h4>
                    <p className="text-sm text-muted-foreground">
                      Use <code>{'${VAR}'}</code> syntax in condition values. Variables are resolved
                      before condition evaluation.
                    </p>
                  </div>
                </div>
              </div>
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
                  <div>
                    <h4 className="font-medium mb-2">Use String Shorthand</h4>
                    <p className="text-sm text-muted-foreground">
                      For simple conditions, prefer <code>"store.x == y"</code> over the verbose
                      object syntax. It's more readable.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <GitBranch className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Check Existence First</h4>
                    <p className="text-sm text-muted-foreground">
                      Before comparing values, ensure they exist. Use <code>exists</code> in{' '}
                      <code>all</code> conditions to guard against missing data.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Workflow className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Plan Your Flow</h4>
                    <p className="text-sm text-muted-foreground">
                      Design your workflow before adding conditions. Ensure data-producing
                      requests come before conditional requests.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <AlertTriangle className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Use continueOnError</h4>
                    <p className="text-sm text-muted-foreground">
                      When building error-handling flows, enable <code>continueOnError</code> to
                      allow conditional error handlers to execute.
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
