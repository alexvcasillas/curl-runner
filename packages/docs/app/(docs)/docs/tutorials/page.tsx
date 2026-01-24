import { ArrowRight, Book, Clock, Target } from 'lucide-react';
import type { Metadata } from 'next';
import Link from 'next/link';
import { DocsPageHeader } from '@/components/docs-page-header';
import { BreadcrumbSchema, DocumentationArticleSchema } from '@/components/structured-data';
import { Button } from '@/components/ui/button';

export const metadata: Metadata = {
  title: 'Tutorials - Step-by-Step Guides',
  description:
    'Learn curl-runner through comprehensive tutorials covering API testing, CI/CD integration, load testing, and advanced workflows. Perfect for beginners and experienced developers.',
  keywords: [
    'curl-runner tutorials',
    'API testing guide',
    'HTTP testing tutorial',
    'YAML configuration tutorial',
    'CI/CD integration guide',
    'load testing tutorial',
    'REST API testing',
    'automation tutorial',
  ],
  openGraph: {
    title: 'curl-runner Tutorials - Step-by-Step Guides',
    description:
      'Learn curl-runner through comprehensive tutorials covering API testing, CI/CD integration, load testing, and advanced workflows.',
    url: 'https://www.curl-runner.com/docs/tutorials',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/tutorials',
  },
};

const tutorials = [
  {
    title: 'Getting Started: Your First API Test',
    description: 'Learn the basics of curl-runner by creating your first API test with YAML configuration',
    duration: '10 min',
    level: 'Beginner',
    href: '/docs/tutorials/getting-started',
    icon: Target,
    topics: ['Creating YAML files', 'Running requests', 'Basic validation'],
  },
  {
    title: 'Authentication Workflows',
    description: 'Build authentication flows that extract tokens and chain API calls together',
    duration: '15 min',
    level: 'Intermediate',
    href: '/docs/tutorials/authentication-workflows',
    icon: Book,
    topics: ['Response storage', 'Token extraction', 'Chained requests'],
  },
  {
    title: 'CI/CD Integration',
    description: 'Set up curl-runner in GitHub Actions and configure exit codes for your pipeline',
    duration: '20 min',
    level: 'Intermediate',
    href: '/docs/tutorials/ci-cd-integration',
    icon: Clock,
    topics: ['GitHub Actions', 'Exit codes', 'Failure thresholds'],
  },
  {
    title: 'Parallel Execution',
    description: 'Speed up your test suites by running requests in parallel with performance validation',
    duration: '15 min',
    level: 'Intermediate',
    href: '/docs/tutorials/parallel-execution',
    icon: Target,
    topics: ['Parallel vs sequential', 'Performance testing', 'Load patterns'],
  },
  {
    title: 'Advanced Validation Patterns',
    description: 'Master regex patterns, numeric ranges, array selectors, and negative testing',
    duration: '20 min',
    level: 'Advanced',
    href: '/docs/tutorials/advanced-validation',
    icon: Book,
    topics: ['Regex validation', 'Numeric ranges', 'Negative testing'],
  },
];

const learningPaths = [
  {
    title: 'Beginner Path',
    description: 'Start from the basics and build your foundation',
    tutorials: ['Getting Started', 'YAML Structure', 'Variables'],
    duration: '45 min',
  },
  {
    title: 'Intermediate Path',
    description: 'Master authentication flows and advanced validation',
    tutorials: ['Authentication Workflows', 'Advanced Validation', 'CLI Options'],
    duration: '1 hour',
  },
  {
    title: 'DevOps Path',
    description: 'Set up CI/CD and performance testing',
    tutorials: ['CI/CD Integration', 'Parallel Execution', 'Environment Variables'],
    duration: '1 hour',
  },
];

export default function TutorialsPage() {
  return (
    <main className="relative py-6 lg:py-8">
      <DocumentationArticleSchema
        title="curl-runner Tutorials - Step-by-Step Guides"
        description="Learn curl-runner through comprehensive tutorials covering API testing, CI/CD integration, load testing, and advanced workflows. Perfect for beginners and experienced developers."
        url="https://www.curl-runner.com/docs/tutorials"
        section="Tutorials"
      />
      <BreadcrumbSchema
        items={[
          { name: 'Home', url: 'https://www.curl-runner.com' },
          { name: 'Documentation', url: 'https://www.curl-runner.com/docs' },
          { name: 'Tutorials', url: 'https://www.curl-runner.com/docs/tutorials' },
        ]}
      />

      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Tutorials"
          text="Step-by-step guides to master curl-runner from basics to advanced use cases"
        />

        <div className="space-y-12">
          {/* Quick Start Notice */}
          <div className="rounded-lg border bg-blue-50/50 dark:bg-blue-950/20 p-6">
            <div className="flex">
              <div className="flex-shrink-0">
                <Book className="h-5 w-5 text-blue-600 dark:text-blue-400" />
              </div>
              <div className="ml-3">
                <h3 className="text-sm font-medium text-foreground">New to curl-runner?</h3>
                <div className="mt-2 text-sm text-muted-foreground">
                  <p>
                    Start with our{' '}
                    <Link
                      href="/docs/quick-start"
                      className="font-medium text-blue-600 hover:underline"
                    >
                      Quick Start Guide
                    </Link>{' '}
                    to install curl-runner and run your first requests in under 5 minutes.
                  </p>
                </div>
              </div>
            </div>
          </div>

          {/* Available Tutorials */}
          <section>
            <h2 className="text-2xl font-bold mb-6">Available Tutorials</h2>
            <div className="grid gap-6 md:grid-cols-1 lg:grid-cols-2">
              {tutorials.map((tutorial) => {
                const Icon = tutorial.icon;
                return (
                  <div
                    key={tutorial.title}
                    className="rounded-lg border bg-card p-6 hover:shadow-md transition-shadow"
                  >
                    <div className="flex items-start gap-4">
                      <div className="rounded-full bg-primary/10 p-3">
                        <Icon className="h-6 w-6 text-primary" />
                      </div>
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-2">
                          <h3 className="font-semibold text-lg">{tutorial.title}</h3>
                          <span className="text-xs bg-muted px-2 py-1 rounded">
                            {tutorial.level}
                          </span>
                        </div>
                        <p className="text-muted-foreground mb-4">{tutorial.description}</p>
                        <div className="flex items-center gap-4 text-sm text-muted-foreground mb-4">
                          <span className="flex items-center gap-1">
                            <Clock className="h-4 w-4" />
                            {tutorial.duration}
                          </span>
                        </div>
                        <div className="mb-4">
                          <p className="text-sm font-medium mb-2">You'll learn:</p>
                          <ul className="text-sm text-muted-foreground space-y-1">
                            {tutorial.topics.map((topic) => (
                              <li key={topic} className="flex items-center gap-2">
                                <span className="h-1.5 w-1.5 bg-primary rounded-full" />
                                {topic}
                              </li>
                            ))}
                          </ul>
                        </div>
                        <Button asChild className="w-full">
                          <Link href={tutorial.href} className="flex items-center justify-center">
                            Start Tutorial <ArrowRight className="ml-2 h-4 w-4" />
                          </Link>
                        </Button>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </section>

          {/* Learning Paths */}
          <section>
            <h2 className="text-2xl font-bold mb-6">Learning Paths</h2>
            <div className="grid gap-6 md:grid-cols-2">
              {learningPaths.map((path) => (
                <div key={path.title} className="rounded-lg border bg-card p-6">
                  <h3 className="font-semibold text-lg mb-2">{path.title}</h3>
                  <p className="text-muted-foreground mb-4">{path.description}</p>
                  <div className="flex items-center gap-2 text-sm text-muted-foreground mb-4">
                    <Clock className="h-4 w-4" />
                    <span>Total duration: {path.duration}</span>
                  </div>
                  <div className="space-y-2">
                    {path.tutorials.map((tutorial, index) => (
                      <div key={tutorial} className="flex items-center gap-2 text-sm">
                        <span className="flex h-6 w-6 items-center justify-center rounded-full bg-primary text-primary-foreground text-xs">
                          {index + 1}
                        </span>
                        {tutorial}
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* Community */}
          <section>
            <h2 className="text-2xl font-bold mb-6">Need Help?</h2>
            <div className="rounded-lg border bg-card p-6">
              <p className="text-muted-foreground mb-4">
                Stuck on a tutorial or have questions? Join our community for help and discussions.
              </p>
              <div className="flex gap-4">
                <Button asChild variant="outline">
                  <Link
                    href="https://github.com/alexvcasillas/curl-runner/discussions"
                    target="_blank"
                  >
                    GitHub Discussions
                  </Link>
                </Button>
                <Button asChild variant="outline">
                  <Link href="https://github.com/alexvcasillas/curl-runner/issues" target="_blank">
                    Report Issues
                  </Link>
                </Button>
              </div>
            </div>
          </section>
        </div>
      </div>
    </main>
  );
}
