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
    title: 'Getting Started with API Testing',
    description: 'Learn the basics of testing REST APIs using curl-runner with practical examples',
    duration: '15 min',
    level: 'Beginner',
    href: '/docs/tutorials/coming-soon',
    icon: Target,
    topics: ['Basic HTTP requests', 'Response validation', 'Error handling'],
  },
  {
    title: 'Advanced YAML Configuration',
    description: 'Master complex configurations with variables, templates, and dynamic values',
    duration: '20 min',
    level: 'Intermediate',
    href: '/docs/tutorials/coming-soon',
    icon: Book,
    topics: ['Variable interpolation', 'Environment configs', 'Template functions'],
  },
  {
    title: 'CI/CD Pipeline Integration',
    description: 'Integrate curl-runner into GitHub Actions, GitLab CI, and other CI/CD platforms',
    duration: '25 min',
    level: 'Intermediate',
    href: '/docs/tutorials/coming-soon',
    icon: Clock,
    topics: ['GitHub Actions', 'GitLab CI', 'Jenkins integration'],
  },
];

const learningPaths = [
  {
    title: 'API Testing Mastery',
    description: 'Complete path from basic requests to advanced testing scenarios',
    tutorials: ['API Testing Basics', 'Response Validation', 'Load Testing'],
    duration: '2 hours',
  },
  {
    title: 'DevOps Integration',
    description: 'Learn to integrate curl-runner in your development workflow',
    tutorials: ['CI/CD Integration', 'Environment Management', 'Monitoring'],
    duration: '3 hours',
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
