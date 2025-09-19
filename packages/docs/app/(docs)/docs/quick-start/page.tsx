import {
  ArrowRight,
  Book,
  CheckCircle,
  FileText,
  FolderOpen,
  Grid3x3,
  Play,
  Shield,
  Terminal,
  Zap,
} from 'lucide-react';
import type { Metadata } from 'next';
import Link from 'next/link';
import { CodeBlockServer } from '@/components/code-block-server';
import { DocsPageHeader } from '@/components/docs-page-header';
import { BreadcrumbSchema, DocumentationArticleSchema } from '@/components/structured-data';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

export const metadata: Metadata = {
  title: 'Quick Start',
  description:
    'Get started with curl-runner in minutes. Learn how to create your first YAML configuration, run HTTP requests, and validate responses with this step-by-step guide.',
  keywords: [
    'curl-runner quick start',
    'getting started curl-runner',
    'curl-runner tutorial',
    'HTTP request tutorial',
    'YAML configuration tutorial',
    'API testing quick start',
    'curl-runner beginner guide',
    'first HTTP request',
    'CLI tool tutorial',
  ],
  openGraph: {
    title: 'Quick Start | curl-runner Documentation',
    description:
      'Get started with curl-runner in minutes. Learn how to create your first YAML configuration, run HTTP requests, and validate responses with this step-by-step guide.',
    url: 'https://www.curl-runner.com/docs/quick-start',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Quick Start | curl-runner Documentation',
    description:
      'Get started with curl-runner in minutes. Learn how to create your first YAML configuration and run HTTP requests.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/quick-start',
  },
};

const simpleExample = `# simple.yaml
request:
  name: Get JSONPlaceholder Post
  url: https://jsonplaceholder.typicode.com/posts/1
  method: GET`;

const collectionExample = `# collection.yaml
global:
  variables:
    BASE_URL: https://jsonplaceholder.typicode.com
  execution: sequential

requests:
  - name: Get All Posts
    url: \${BASE_URL}/posts
    method: GET
    
  - name: Get Specific Post
    url: \${BASE_URL}/posts/1
    method: GET
    
  - name: Get Post Comments
    url: \${BASE_URL}/posts/1/comments
    method: GET`;

const parallelExample = `# parallel.yaml
global:
  execution: parallel
  continueOnError: true

requests:
  - name: Check API Health
    url: https://jsonplaceholder.typicode.com/posts/1
    method: GET
    
  - name: Check Users Endpoint
    url: https://jsonplaceholder.typicode.com/users/1
    method: GET
    
  - name: Check Albums Endpoint
    url: https://jsonplaceholder.typicode.com/albums/1
    method: GET`;

const steps = [
  {
    step: 1,
    title: 'Install curl-runner',
    description: 'Install using your preferred package manager',
    code: 'bun install -g @curl-runner/cli',
  },
  {
    step: 2,
    title: 'Create your first YAML file',
    description: 'Define your HTTP request configuration',
    code: simpleExample,
    filename: 'simple.yaml',
  },
  {
    step: 3,
    title: 'Run the request',
    description: 'Execute your configuration file',
    code: 'curl-runner simple.yaml',
  },
  {
    step: 4,
    title: 'View the results',
    description: 'See the formatted output in your terminal',
    code: `ℹ Found 1 YAML file(s)
ℹ Processing: simple.yaml

✓ Get JSONPlaceholder Post [simple]
   ├─ GET: https://jsonplaceholder.typicode.com/posts/1
   ├─ ✓ Status: 200
   └─ Duration: 245ms | 292.00 B

Summary: 1 request completed successfully`,
  },
];

export default function QuickStartPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <DocumentationArticleSchema
        title="curl-runner Quick Start Guide"
        description="Get started with curl-runner in minutes. Learn how to create your first YAML configuration, run HTTP requests, and validate responses with this step-by-step guide."
        url="https://www.curl-runner.com/docs/quick-start"
        section="Quick Start"
      />
      <BreadcrumbSchema
        items={[
          { name: 'Home', url: 'https://www.curl-runner.com' },
          { name: 'Documentation', url: 'https://www.curl-runner.com/docs' },
          { name: 'Quick Start', url: 'https://www.curl-runner.com/docs/quick-start' },
        ]}
      />
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Quick Start"
          text="Get up and running with curl-runner in just a few minutes. Follow these simple steps to make your first HTTP request."
        />

        <div className="space-y-8">
          {/* Step by Step Guide */}
          <section>
            <div className="space-y-8">
              {steps.map((step, _index) => (
                <div key={step.step} className="space-y-3">
                  <div className="flex items-start space-x-4">
                    <div className="flex h-8 w-8 items-center justify-center rounded-full bg-primary text-primary-foreground text-sm font-medium">
                      {step.step}
                    </div>
                    <div className="space-y-1">
                      <h3 className="text-xl font-semibold">{step.title}</h3>
                      <p className="text-muted-foreground">{step.description}</p>
                    </div>
                  </div>
                  <CodeBlockServer
                    language={step.step === 4 ? 'text' : step.filename ? 'yaml' : 'bash'}
                    filename={step.filename}
                  >
                    {step.code}
                  </CodeBlockServer>
                </div>
              ))}
            </div>
          </section>

          {/* More Examples */}
          <section>
            <h2 className="text-2xl font-semibold tracking-tight mb-4">More Examples</h2>
            <p className="text-muted-foreground mb-6">
              Try these more advanced examples to explore{' '}
              <code className="font-mono">curl-runner</code>'s capabilities.
            </p>

            <div className="space-y-8">
              {/* Collection Example */}
              <div className="space-y-3">
                <div className="flex items-center space-x-2">
                  <h3 className="text-xl font-semibold">Request Collection</h3>
                  <Badge variant="secondary">Sequential</Badge>
                </div>
                <p className="text-muted-foreground">
                  Execute multiple requests in sequence with shared variables.
                </p>
                <div className="space-y-4">
                  <CodeBlockServer language="yaml" filename="collection.yaml">
                    {collectionExample}
                  </CodeBlockServer>
                  <CodeBlockServer language="bash">curl-runner collection.yaml</CodeBlockServer>
                </div>
              </div>

              {/* Parallel Example */}
              <div className="space-y-3">
                <div className="flex items-center space-x-2">
                  <h3 className="text-xl font-semibold">Parallel Execution</h3>
                  <Badge variant="default">Parallel</Badge>
                </div>
                <p className="text-muted-foreground">
                  Run multiple requests simultaneously for faster execution.
                </p>
                <div className="space-y-4">
                  <CodeBlockServer language="yaml" filename="parallel.yaml">
                    {parallelExample}
                  </CodeBlockServer>
                  <CodeBlockServer language="bash">curl-runner parallel.yaml -v</CodeBlockServer>
                </div>
              </div>
            </div>
          </section>

          {/* Tips and Tricks */}
          <section>
            <h2 className="text-2xl font-semibold tracking-tight mb-4">Tips & Tricks</h2>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Play className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-3">CLI Options</h4>
                    <div className="space-y-3 text-sm">
                      <div>
                        <code className="bg-muted p-1.5 rounded-lg">-v</code> —Enable verbose output
                      </div>
                      <div>
                        <code className="bg-muted p-1.5 rounded-lg">-p</code> —Run requests in
                        parallel
                      </div>
                      <div>
                        <code className="bg-muted p-1.5 rounded-lg">-c</code> —Continue on errors
                      </div>
                      <div>
                        <code className="bg-muted p-1.5 rounded-lg">--output results.json</code>{' '}
                        —Save results
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-3">File Patterns</h4>
                    <div className="space-y-3 text-sm">
                      <div>
                        <code className="bg-muted p-1.5 rounded-lg">*.yaml</code> —Run all YAML
                        files
                      </div>
                      <div>
                        <code className="bg-muted p-1.5 rounded-lg">tests/</code> —Run all files in
                        directory
                      </div>
                      <div>
                        <code className="bg-muted p-1.5 rounded-lg">--all</code> —Search recursively
                      </div>
                      <div>
                        <code className="bg-muted p-1.5 rounded-lg">api-*.yaml</code> —Pattern
                        matching
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Next Steps */}
          <section>
            <h2 className="text-2xl font-semibold tracking-tight mb-4">What's Next?</h2>
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Book className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Learn YAML Structure</h4>
                    <p className="text-sm text-muted-foreground mb-4">
                      Understand the full configuration format
                    </p>
                    <Button asChild variant="ghost" className="p-0">
                      <Link href="/docs/yaml-structure" className="flex items-center">
                        Read Guide <ArrowRight className="ml-1 h-4 w-4" />
                      </Link>
                    </Button>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <FileText className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Explore Variables</h4>
                    <p className="text-sm text-muted-foreground mb-4">
                      Use variables and templating
                    </p>
                    <Button asChild variant="ghost" className="p-0">
                      <Link href="/docs/variables" className="flex items-center">
                        Learn More <ArrowRight className="ml-1 h-4 w-4" />
                      </Link>
                    </Button>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Grid3x3 className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Browse Examples</h4>
                    <p className="text-sm text-muted-foreground mb-4">
                      Real-world use cases and patterns
                    </p>
                    <Button asChild variant="ghost" className="p-0">
                      <Link href="/docs/examples/basic" className="flex items-center">
                        View Examples <ArrowRight className="ml-1 h-4 w-4" />
                      </Link>
                    </Button>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Continue Learning Section */}
          <section className="mb-16">
            <div className="mb-8">
              <h2 className="text-2xl font-bold mb-4">Continue Learning</h2>
              <p className="text-muted-foreground">
                Now that you've made your first request, explore these advanced features
              </p>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Shield className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h3 className="font-medium mb-2">
                      <Link
                        href="/docs/features/response-validation"
                        className="hover:text-primary"
                      >
                        Response Validation
                      </Link>
                    </h3>
                    <p className="text-sm text-muted-foreground mb-3">
                      Automatically validate API responses to ensure they meet your expectations
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <FolderOpen className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h3 className="font-medium mb-2">
                      <Link href="/docs/examples/collection" className="hover:text-primary">
                        Request Collections
                      </Link>
                    </h3>
                    <p className="text-sm text-muted-foreground mb-3">
                      Organize multiple related requests into collections for better workflow
                      management
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <Terminal className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div className="flex-1">
                    <h3 className="font-medium mb-2">
                      <Link href="/docs/cli-commands" className="hover:text-primary">
                        CLI Commands
                      </Link>
                    </h3>
                    <p className="text-sm text-muted-foreground mb-3">
                      Master all command-line options and flags for advanced usage
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Zap className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <h3 className="font-medium mb-2">
                      <Link href="/docs/features/parallel-execution" className="hover:text-primary">
                        Parallel Execution
                      </Link>
                    </h3>
                    <p className="text-sm text-muted-foreground mb-3">
                      Execute multiple requests concurrently for performance testing and faster
                      workflows
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
          <div className="space-y-2">
            <p className="font-medium">On This Page</p>
            <ul className="m-0 list-none">
              <li className="mt-0 pt-2">
                <a
                  href="#step-by-step"
                  className="inline-block no-underline transition-colors hover:text-foreground text-muted-foreground"
                >
                  Step by Step
                </a>
              </li>
              <li className="mt-0 pt-2">
                <a
                  href="#more-examples"
                  className="inline-block no-underline transition-colors hover:text-foreground text-muted-foreground"
                >
                  More Examples
                </a>
              </li>
              <li className="mt-0 pt-2">
                <a
                  href="#tips-tricks"
                  className="inline-block no-underline transition-colors hover:text-foreground text-muted-foreground"
                >
                  Tips & Tricks
                </a>
              </li>
              <li className="mt-0 pt-2">
                <a
                  href="#whats-next"
                  className="inline-block no-underline transition-colors hover:text-foreground text-muted-foreground"
                >
                  What's Next?
                </a>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </main>
  );
}
