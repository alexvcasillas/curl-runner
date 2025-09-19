import { ArrowRight, Book, FileText, Grid3x3, Settings, Terminal, Zap } from 'lucide-react';
import type { Metadata } from 'next';
import Link from 'next/link';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Button } from '@/components/ui/button';
import { basicUsageExample, quickInstallExample } from './snippets';

export const metadata: Metadata = {
  title: 'curl-runner Documentation',
  description:
    'A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance, parallel execution, and comprehensive validation.',
  keywords: [
    'curl-runner documentation',
    'HTTP request management',
    'YAML configuration',
    'API testing tool',
    'Bun CLI tool',
    'parallel HTTP requests',
    'response validation',
    'API automation',
    'HTTP client',
    'REST API testing',
    'request templating',
    'CI/CD integration',
  ],
  openGraph: {
    title: 'curl-runner Documentation',
    description:
      'A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance, parallel execution, and comprehensive validation.',
    url: 'https://www.curl-runner.com/docs',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'curl-runner Documentation',
    description:
      'A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs',
  },
};

const features = [
  {
    icon: FileText,
    title: 'YAML Configuration',
    description:
      'Define HTTP requests using simple, readable YAML files with support for variables and templates.',
    color: 'blue',
  },
  {
    icon: Zap,
    title: 'Fast Execution',
    description:
      'Built with Bun for blazing-fast performance. Execute requests sequentially or in parallel.',
    color: 'yellow',
  },
  {
    icon: Terminal,
    title: 'Powerful CLI',
    description:
      'Comprehensive command-line interface with beautiful output, progress indicators, and error handling.',
    color: 'green',
  },
  {
    icon: Settings,
    title: 'Flexible Configuration',
    description:
      'Global settings, variable interpolation, response validation, and customizable output formats.',
    color: 'purple',
  },
];

export default function DocsPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="curl-runner Documentation"
          text="A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance."
        />

        <div className="space-y-12">
          {/* Features Grid */}
          <section>
            <H2 id="features">Features</H2>
            <div className="grid gap-6 md:grid-cols-2 mt-6">
              {features.map((feature) => {
                const Icon = feature.icon;
                const colorClasses = {
                  blue: 'bg-blue-500/10 text-blue-600 dark:text-blue-400',
                  yellow: 'bg-yellow-500/10 text-yellow-600 dark:text-yellow-500',
                  green: 'bg-green-500/10 text-green-600 dark:text-green-400',
                  purple: 'bg-purple-500/10 text-purple-600 dark:text-purple-400',
                } as const;

                return (
                  <div key={feature.title} className="rounded-lg border bg-card p-6">
                    <div className="flex items-start gap-4">
                      <div
                        className={`rounded-full p-3 ${colorClasses[feature.color as keyof typeof colorClasses]?.split(' ')[0]}`}
                      >
                        <Icon
                          className={`h-5 w-5 ${colorClasses[feature.color as keyof typeof colorClasses]?.split(' ').slice(1).join(' ')}`}
                        />
                      </div>
                      <div className="flex-1">
                        <h3 className="font-semibold text-lg mb-2">{feature.title}</h3>
                        <p className="text-muted-foreground">{feature.description}</p>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </section>

          {/* What is curl-runner */}
          <section>
            <H2 id="what-is-curl-runner">What is curl-runner?</H2>
            <p className="text-muted-foreground text-lg mb-6">
              <code className="font-mono">curl-runner</code> is a modern CLI tool that transforms
              how you manage and execute HTTP requests. Instead of writing complex curl commands or
              maintaining scattered shell scripts, you define your API requests in clean, readable
              YAML files with powerful features like parallel execution, response validation, and
              variable templating.
            </p>
            <p className="text-muted-foreground text-lg">
              Built with Bun for exceptional performance, curl-runner is perfect for API testing,
              automation workflows, CI/CD pipelines, and development environments where speed and
              reliability matter.
            </p>
          </section>

          {/* Quick Start */}
          <section>
            <H2 id="quick-start">Quick Start</H2>
            <p className="text-muted-foreground text-lg mb-8">
              Get up and running with <code className="font-mono">curl-runner</code> in just a few
              minutes.
            </p>

            {/* Prerequisites Notice */}
            <div className="rounded-lg border bg-blue-50/50 dark:bg-blue-950/20 p-4 mb-8">
              <div className="flex">
                <div className="flex-shrink-0">
                  <Zap className="h-5 w-5 text-blue-600 dark:text-blue-400" />
                </div>
                <div className="ml-3">
                  <h3 className="text-sm font-medium text-foreground">Prerequisites</h3>
                  <div className="mt-2 text-sm text-muted-foreground space-y-2">
                    <p>
                      <strong>For npm/yarn/pnpm installation:</strong> Bun runtime must be installed
                      on your system.
                    </p>
                    <p>
                      Install Bun:{' '}
                      <code className="bg-muted px-1 rounded">
                        curl -fsSL https://bun.sh/install | bash
                      </code>
                    </p>
                    <p className="mt-3">
                      <strong>For standalone builds:</strong> No prerequisites needed! The
                      standalone executable includes Bun bundled within it.
                    </p>
                  </div>
                </div>
              </div>
            </div>

            <div className="space-y-8">
              <div>
                <H3 id="1-install-curl-runner">1. Install curl-runner</H3>
                <CodeBlockServer language="bash">{quickInstallExample}</CodeBlockServer>
              </div>

              <div>
                <H3 id="2-create-a-yaml-file">2. Create a YAML file</H3>
                <CodeBlockServer language="yaml" filename="api-test.yaml">
                  {basicUsageExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3 id="3-run-your-requests">3. Run your requests</H3>
                <CodeBlockServer language="bash">curl-runner api-test.yaml</CodeBlockServer>
              </div>
            </div>
          </section>

          {/* Next Steps */}
          <section>
            <H2 id="next-steps">Next Steps</H2>
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <Settings className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Installation Guide</h4>
                    <p className="text-sm text-muted-foreground mb-4">
                      Detailed installation instructions for all platforms
                    </p>
                    <Button asChild variant="ghost" className="p-0">
                      <Link href="/docs/installation" className="flex items-center">
                        Get Started <ArrowRight className="ml-1 h-4 w-4" />
                      </Link>
                    </Button>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <Book className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">YAML Structure</h4>
                    <p className="text-sm text-muted-foreground mb-4">
                      Learn the YAML configuration format and options
                    </p>
                    <Button asChild variant="ghost" className="p-0">
                      <Link href="/docs/yaml-structure" className="flex items-center">
                        Learn More <ArrowRight className="ml-1 h-4 w-4" />
                      </Link>
                    </Button>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Grid3x3 className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-medium mb-2">Examples</h4>
                    <p className="text-sm text-muted-foreground mb-4">
                      Browse real-world examples and use cases
                    </p>
                    <Button asChild variant="ghost" className="p-0">
                      <Link href="/docs/examples/basic" className="flex items-center">
                        Explore <ArrowRight className="ml-1 h-4 w-4" />
                      </Link>
                    </Button>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Key Concepts Section */}
          <section className="mb-16">
            <div className="mb-8">
              <h2 className="text-2xl font-bold mb-4">Key Concepts</h2>
              <p className="text-muted-foreground">
                Master these essential concepts to get the most out of curl-runner
              </p>
            </div>

            <div className="grid gap-6 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-6">
                <h3 className="font-semibold mb-2">
                  <Link href="/docs/variables" className="hover:text-primary">
                    Variables & Templating
                  </Link>
                </h3>
                <p className="text-sm text-muted-foreground mb-3">
                  Create dynamic requests with variables, environment values, and computed expressions
                </p>
                <Link href="/docs/variables" className="text-sm font-medium text-primary hover:underline">
                  Learn about variables →
                </Link>
              </div>

              <div className="rounded-lg border bg-card p-6">
                <h3 className="font-semibold mb-2">
                  <Link href="/docs/global-settings" className="hover:text-primary">
                    Global Settings
                  </Link>
                </h3>
                <p className="text-sm text-muted-foreground mb-3">
                  Configure execution behavior, timeouts, and default values for all requests
                </p>
                <Link href="/docs/global-settings" className="text-sm font-medium text-primary hover:underline">
                  Configure globally →
                </Link>
              </div>

              <div className="rounded-lg border bg-card p-6">
                <h3 className="font-semibold mb-2">
                  <Link href="/docs/features/response-validation" className="hover:text-primary">
                    Response Validation
                  </Link>
                </h3>
                <p className="text-sm text-muted-foreground mb-3">
                  Validate status codes, headers, and response body content automatically
                </p>
                <Link href="/docs/features/response-validation" className="text-sm font-medium text-primary hover:underline">
                  Validate responses →
                </Link>
              </div>

              <div className="rounded-lg border bg-card p-6">
                <h3 className="font-semibold mb-2">
                  <Link href="/docs/features/parallel-execution" className="hover:text-primary">
                    Parallel Execution
                  </Link>
                </h3>
                <p className="text-sm text-muted-foreground mb-3">
                  Execute multiple requests concurrently for faster performance and load testing
                </p>
                <Link href="/docs/features/parallel-execution" className="text-sm font-medium text-primary hover:underline">
                  Run in parallel →
                </Link>
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
