import {
  ArrowRight,
  Book,
  FileText,
  Github,
  Grid3x3,
  Linkedin,
  Settings,
  Terminal,
  Twitter,
  Users,
  Zap,
} from 'lucide-react';
import Link from 'next/link';
import { CodeBlockServer } from '@/components/code-block-server';
import { DocsPageHeader } from '@/components/docs-page-header';
import { H2, H3 } from '@/components/mdx-heading';
import { TableOfContents } from '@/components/toc';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { basicUsageExample, quickInstallExample } from './snippets';

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
            <H2>Features</H2>
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

          {/* Quick Start */}
          <section>
            <H2>Quick Start</H2>
            <p className="text-muted-foreground text-lg mb-8">
              Get up and running with <code className="font-mono">curl-runner</code> in just a few
              minutes.
            </p>

            <div className="space-y-8">
              <div>
                <H3>1. Create a YAML file</H3>
                <CodeBlockServer language="yaml" filename="simple.yaml">
                  {basicUsageExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3>
                  2. Run <code className="font-mono">curl-runner</code>
                </H3>
                <CodeBlockServer language="bash">{quickInstallExample}</CodeBlockServer>
              </div>
            </div>
          </section>

          {/* Next Steps */}
          <section>
            <H2>Next Steps</H2>
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
