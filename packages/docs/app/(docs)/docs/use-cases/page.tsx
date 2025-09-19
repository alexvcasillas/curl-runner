import type { Metadata } from 'next';
import Link from 'next/link';
import {
  ArrowRight,
  CheckCircle,
  Cloud,
  Code2,
  Database,
  GitBranch,
  Globe,
  Monitor,
  Server,
  Shield,
  Zap,
} from 'lucide-react';
import { DocsPageHeader } from '@/components/docs-page-header';
import { LogoCloud } from '@/components/logo-cloud';
import {
  DocumentationArticleSchema,
  BreadcrumbSchema,
  FAQSchema,
} from '@/components/structured-data';
import { Button } from '@/components/ui/button';

export const metadata: Metadata = {
  title: 'Use Cases - Real-World Applications',
  description:
    'Discover how to use curl-runner for API testing, CI/CD automation, load testing, microservices monitoring, and development workflows. Real-world examples and best practices.',
  keywords: [
    'curl-runner use cases',
    'API testing examples',
    'CI/CD automation',
    'load testing scenarios',
    'microservices testing',
    'API monitoring',
    'development workflow',
    'REST API validation',
    'HTTP testing patterns',
    'automation examples',
  ],
  openGraph: {
    title: 'curl-runner Use Cases - Real-World Applications',
    description:
      'Discover how to use curl-runner for API testing, CI/CD automation, load testing, microservices monitoring, and development workflows.',
    url: 'https://www.curl-runner.com/docs/use-cases',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/use-cases',
  },
};

const useCases = [
  {
    title: 'API Testing & Validation',
    description: 'Comprehensive testing of REST APIs, GraphQL endpoints, and microservices',
    icon: CheckCircle,
    examples: [
      'End-to-end API testing suites',
      'Response schema validation',
      'Authentication flow testing',
      'Error handling verification',
    ],
    industries: ['SaaS', 'E-commerce', 'Fintech', 'Healthcare'],
    link: '/docs/features/response-validation',
  },
  {
    title: 'CI/CD Pipeline Integration',
    description: 'Automated testing and validation in continuous integration workflows',
    icon: GitBranch,
    examples: [
      'Pre-deployment API health checks',
      'Integration test automation',
      'Environment smoke testing',
      'Release validation pipelines',
    ],
    industries: ['DevOps', 'Software Development', 'Cloud Services'],
    link: '/docs/examples/advanced',
  },
  {
    title: 'Load Testing & Performance',
    description: 'Stress testing APIs and measuring performance under various loads',
    icon: Zap,
    examples: [
      'Concurrent request load testing',
      'Performance benchmarking',
      'Stress testing scenarios',
      'Capacity planning validation',
    ],
    industries: ['E-commerce', 'Gaming', 'Media Streaming'],
    link: '/docs/features/parallel-execution',
  },
  {
    title: 'Microservices Monitoring',
    description: 'Health checks and monitoring for distributed microservices architectures',
    icon: Server,
    examples: [
      'Service health monitoring',
      'Inter-service communication testing',
      'Circuit breaker validation',
      'Dependency health checks',
    ],
    industries: ['Enterprise', 'Cloud Platforms', 'Distributed Systems'],
    link: '/docs/examples/collection',
  },
  {
    title: 'Development Workflows',
    description: 'Streamline development processes with automated request execution',
    icon: Code2,
    examples: [
      'Local development testing',
      'API documentation validation',
      'Mock server interaction',
      'Development environment setup',
    ],
    industries: ['Software Development', 'API Development'],
    link: '/docs/examples/basic',
  },
  {
    title: 'Security Testing',
    description: 'Validate authentication, authorization, and security protocols',
    icon: Shield,
    examples: [
      'OAuth flow validation',
      'JWT token testing',
      'Rate limiting verification',
      'Security header validation',
    ],
    industries: ['Security', 'Banking', 'Healthcare', 'Government'],
    link: '/docs/variables',
  },
];

const benefits = [
  {
    title: 'Faster Development Cycles',
    description: 'Automated testing reduces manual effort and speeds up delivery',
    icon: Zap,
  },
  {
    title: 'Better API Quality',
    description: 'Comprehensive validation ensures robust and reliable APIs',
    icon: CheckCircle,
  },
  {
    title: 'Reduced Downtime',
    description: 'Early detection of issues prevents production failures',
    icon: Monitor,
  },
  {
    title: 'Cost Savings',
    description: 'Prevent expensive post-deployment fixes and rollbacks',
    icon: Database,
  },
];

const faqs = [
  {
    question: 'Can curl-runner replace Postman for API testing?',
    answer:
      "Yes, curl-runner is designed for automated, configuration-driven API testing that's perfect for CI/CD pipelines. While Postman excels at manual testing and exploration, curl-runner excels at automated, repeatable testing workflows.",
  },
  {
    question: 'How does curl-runner handle authentication in real-world scenarios?',
    answer:
      'curl-runner supports various authentication methods including Basic Auth, Bearer tokens, OAuth flows, and custom headers. You can use environment variables and templates to manage credentials securely across different environments.',
  },
  {
    question: 'Is curl-runner suitable for load testing production systems?',
    answer:
      "curl-runner can perform load testing with parallel execution, but it's primarily designed for functional testing and moderate load scenarios. For extensive load testing, consider specialized tools like k6 or Artillery alongside curl-runner for validation.",
  },
  {
    question: 'How do I migrate existing API tests to curl-runner?',
    answer:
      'Most existing curl commands and Postman collections can be converted to YAML configuration. Start with simple requests and gradually add validation rules, variables, and parallel execution as needed.',
  },
];

export default function UseCasesPage() {
  return (
    <main className="relative py-6 lg:py-8">
      <DocumentationArticleSchema
        title="curl-runner Use Cases - Real-World Applications"
        description="Discover how to use curl-runner for API testing, CI/CD automation, load testing, microservices monitoring, and development workflows. Real-world examples and best practices."
        url="https://www.curl-runner.com/docs/use-cases"
        section="Use Cases"
      />
      <BreadcrumbSchema
        items={[
          { name: 'Home', url: 'https://www.curl-runner.com' },
          { name: 'Documentation', url: 'https://www.curl-runner.com/docs' },
          { name: 'Use Cases', url: 'https://www.curl-runner.com/docs/use-cases' },
        ]}
      />
      <FAQSchema faqs={faqs} />

      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="Real-World Use Cases"
          text="Discover how organizations use curl-runner to solve API testing, automation, and monitoring challenges"
        />

        <div className="space-y-12">
          {/* Overview - Bento Grid */}
          <section>
            <h2 className="text-2xl font-bold mb-6">Why Teams Choose curl-runner</h2>
            <p className="text-muted-foreground mb-8 max-w-3xl">
              From startups to enterprise organizations, teams rely on curl-runner for reliable,
              automated API testing and validation. Here's how they're using it to improve their
              workflows.
            </p>

            {/* Bento Grid Layout */}
            <div className="mt-8 grid gap-4 lg:grid-cols-3 lg:grid-rows-2">
              {/* Faster Development Cycles - Large Left Card */}
              <div className="relative lg:row-span-2">
                <div className="absolute inset-px rounded-lg bg-gradient-to-br from-green-50/50 to-emerald-100/80 dark:from-green-950/20 dark:to-emerald-950/40 lg:rounded-l-2xl" />
                <div className="relative flex h-full flex-col overflow-hidden rounded-lg lg:rounded-l-2xl border">
                  <div className="px-6 pt-6 pb-3 sm:px-8 sm:pt-8 sm:pb-0">
                    <div className="flex items-center gap-3 mb-4">
                      <div className="rounded-full bg-green-500/10 p-2">
                        <Zap className="h-5 w-5 text-green-600 dark:text-green-400" />
                      </div>
                    </div>
                    <h4 className="text-lg font-semibold mb-3 text-green-900 dark:text-green-100">
                      {benefits[0].title}
                    </h4>
                    <p className="text-sm text-green-800 dark:text-green-200 mb-4">
                      {benefits[0].description}
                    </p>
                  </div>
                  <div className="flex-1 px-6 pb-6 sm:px-8 sm:pb-8">
                    <div className="mt-4 space-y-3">
                      <div className="flex items-start gap-3">
                        <div className="w-2 h-2 rounded-full bg-green-500 mt-2 flex-shrink-0" />
                        <div className="text-sm text-green-800 dark:text-green-200">
                          Reduce manual testing effort
                        </div>
                      </div>
                      <div className="flex items-start gap-3">
                        <div className="w-2 h-2 rounded-full bg-green-500 mt-2 flex-shrink-0" />
                        <div className="text-sm text-green-800 dark:text-green-200">
                          Accelerate deployment cycles
                        </div>
                      </div>
                      <div className="flex items-start gap-3">
                        <div className="w-2 h-2 rounded-full bg-green-500 mt-2 flex-shrink-0" />
                        <div className="text-sm text-green-800 dark:text-green-200">
                          Streamline development workflows
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              {/* Better API Quality - Top Middle Card */}
              <div className="relative max-lg:row-start-1">
                <div className="absolute inset-px rounded-lg bg-gradient-to-br from-blue-50/50 to-cyan-100/80 dark:from-blue-950/20 dark:to-cyan-950/40 max-lg:rounded-t-2xl" />
                <div className="relative flex h-full flex-col overflow-hidden rounded-lg max-lg:rounded-t-2xl border">
                  <div className="px-6 pt-6 pb-6 sm:px-8 sm:pt-8 sm:pb-8">
                    <div className="flex items-center gap-3 mb-4">
                      <div className="rounded-full bg-blue-500/10 p-2">
                        <CheckCircle className="h-5 w-5 text-blue-600 dark:text-blue-400" />
                      </div>
                    </div>
                    <h4 className="text-lg font-semibold mb-3 text-blue-900 dark:text-blue-100">
                      {benefits[1].title}
                    </h4>
                    <p className="text-sm text-blue-800 dark:text-blue-200">
                      {benefits[1].description}
                    </p>
                  </div>
                </div>
              </div>

              {/* Reduced Downtime - Bottom Middle Card */}
              <div className="relative max-lg:row-start-3 lg:col-start-2 lg:row-start-2">
                <div className="absolute inset-px rounded-lg bg-gradient-to-br from-purple-50/50 to-violet-100/80 dark:from-purple-950/20 dark:to-violet-950/40" />
                <div className="relative flex h-full flex-col overflow-hidden rounded-lg border">
                  <div className="px-6 pt-6 pb-6 sm:px-8 sm:pt-8 sm:pb-8">
                    <div className="flex items-center gap-3 mb-4">
                      <div className="rounded-full bg-purple-500/10 p-2">
                        <Monitor className="h-5 w-5 text-purple-600 dark:text-purple-400" />
                      </div>
                    </div>
                    <h4 className="text-lg font-semibold mb-3 text-purple-900 dark:text-purple-100">
                      {benefits[2].title}
                    </h4>
                    <p className="text-sm text-purple-800 dark:text-purple-200">
                      {benefits[2].description}
                    </p>
                  </div>
                </div>
              </div>

              {/* Cost Savings - Large Right Card */}
              <div className="relative lg:row-span-2">
                <div className="absolute inset-px rounded-lg bg-gradient-to-br from-orange-50/50 to-amber-100/80 dark:from-orange-950/20 dark:to-amber-950/40 max-lg:rounded-b-2xl lg:rounded-r-2xl" />
                <div className="relative flex h-full flex-col overflow-hidden rounded-lg max-lg:rounded-b-2xl lg:rounded-r-2xl border">
                  <div className="px-6 pt-6 pb-3 sm:px-8 sm:pt-8 sm:pb-0">
                    <div className="flex items-center gap-3 mb-4">
                      <div className="rounded-full bg-orange-500/10 p-2">
                        <Database className="h-5 w-5 text-orange-600 dark:text-orange-400" />
                      </div>
                    </div>
                    <h4 className="text-lg font-semibold mb-3 text-orange-900 dark:text-orange-100">
                      {benefits[3].title}
                    </h4>
                    <p className="text-sm text-orange-800 dark:text-orange-200 mb-4">
                      {benefits[3].description}
                    </p>
                  </div>
                  <div className="flex-1 px-6 pb-6 sm:px-8 sm:pb-8">
                    <div className="mt-4 space-y-4">
                      <div className="flex items-center gap-3">
                        <div className="h-8 w-8 rounded-lg bg-orange-500/10 flex items-center justify-center">
                          <Database className="h-4 w-4 text-orange-500" />
                        </div>
                        <div className="flex-1">
                          <div className="text-sm font-medium text-orange-900 dark:text-orange-100">
                            Avoid Rollbacks
                          </div>
                          <div className="text-xs text-orange-800 dark:text-orange-300">
                            Catch issues before deployment
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-3">
                        <div className="h-8 w-8 rounded-lg bg-red-500/10 flex items-center justify-center">
                          <Zap className="h-4 w-4 text-red-500" />
                        </div>
                        <div className="flex-1">
                          <div className="text-sm font-medium text-orange-900 dark:text-orange-100">
                            Reduce Hotfixes
                          </div>
                          <div className="text-xs text-orange-800 dark:text-orange-300">
                            Less emergency patches needed
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          <LogoCloud className="py-12 sm:py-16" />

          {/* Use Cases Grid */}
          <section>
            <h2 className="text-2xl font-bold mb-8">Popular Use Cases</h2>
            <div className="grid gap-8 lg:grid-cols-2">
              {useCases.map((useCase) => {
                const Icon = useCase.icon;
                return (
                  <div
                    key={useCase.title}
                    className="rounded-lg border bg-card p-6 hover:shadow-md transition-shadow"
                  >
                    <div className="flex items-start gap-4 mb-4">
                      <div className="rounded-full bg-primary/10 p-3">
                        <Icon className="h-6 w-6 text-primary" />
                      </div>
                      <div className="flex-1">
                        <h3 className="font-semibold text-lg mb-2">{useCase.title}</h3>
                        <p className="text-muted-foreground">{useCase.description}</p>
                      </div>
                    </div>

                    <div className="space-y-4">
                      <div>
                        <h4 className="font-medium text-sm mb-2">Common Applications:</h4>
                        <ul className="space-y-1">
                          {useCase.examples.map((example) => (
                            <li
                              key={example}
                              className="flex items-center gap-2 text-sm text-muted-foreground"
                            >
                              <span className="h-1.5 w-1.5 bg-primary rounded-full" />
                              {example}
                            </li>
                          ))}
                        </ul>
                      </div>

                      <div>
                        <h4 className="font-medium text-sm mb-2">Popular Industries:</h4>
                        <div className="flex flex-wrap gap-2">
                          {useCase.industries.map((industry) => (
                            <span key={industry} className="text-xs bg-muted px-2 py-1 rounded">
                              {industry}
                            </span>
                          ))}
                        </div>
                      </div>

                      <Button asChild className="w-full" variant="outline">
                        <Link href={useCase.link} className="flex items-center justify-center">
                          Learn More <ArrowRight className="ml-2 h-4 w-4" />
                        </Link>
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>
          </section>

          {/* Industry Examples */}
          <section>
            <h2 className="text-2xl font-bold mb-6">Industry Examples</h2>
            <div className="grid gap-6 md:grid-cols-3">
              <div className="rounded-lg border bg-card p-6">
                <div className="flex items-center gap-3 mb-4">
                  <Globe className="h-6 w-6 text-blue-600" />
                  <h3 className="font-semibold">E-commerce Platform</h3>
                </div>
                <p className="text-sm text-muted-foreground mb-4">
                  "We use curl-runner to validate our checkout API, payment processing, and
                  inventory updates across 50+ microservices. Reduced deployment issues by 80%."
                </p>
                <div className="text-xs text-muted-foreground">
                  <strong>Key Features:</strong> Parallel execution, response validation, CI/CD
                  integration
                </div>
              </div>

              <div className="rounded-lg border bg-card p-6">
                <div className="flex items-center gap-3 mb-4">
                  <Cloud className="h-6 w-6 text-green-600" />
                  <h3 className="font-semibold">Cloud Platform</h3>
                </div>
                <p className="text-sm text-muted-foreground mb-4">
                  "Our infrastructure team runs curl-runner health checks every 5 minutes across
                  200+ API endpoints to ensure service availability and performance."
                </p>
                <div className="text-xs text-muted-foreground">
                  <strong>Key Features:</strong> Monitoring automation, environment variables, error
                  reporting
                </div>
              </div>

              <div className="rounded-lg border bg-card p-6">
                <div className="flex items-center gap-3 mb-4">
                  <Database className="h-6 w-6 text-purple-600" />
                  <h3 className="font-semibold">Fintech Startup</h3>
                </div>
                <p className="text-sm text-muted-foreground mb-4">
                  "Security and compliance are critical. curl-runner helps us validate OAuth flows,
                  rate limiting, and API security policies automatically."
                </p>
                <div className="text-xs text-muted-foreground">
                  <strong>Key Features:</strong> Security testing, authentication validation,
                  compliance checks
                </div>
              </div>
            </div>
          </section>

          {/* FAQ */}
          <section>
            <h2 className="text-2xl font-bold mb-6">Frequently Asked Questions</h2>
            <div className="space-y-6">
              {faqs.map((faq, index) => (
                <div key={index} className="rounded-lg border bg-card p-6">
                  <h3 className="font-semibold mb-3">{faq.question}</h3>
                  <p className="text-muted-foreground text-sm">{faq.answer}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Get Started */}
          <section className="text-center">
            <div className="rounded-lg border bg-card p-8">
              <h2 className="text-2xl font-bold mb-4">Ready to Get Started?</h2>
              <p className="text-muted-foreground mb-6 max-w-2xl mx-auto">
                Join thousands of developers and teams who trust curl-runner for their API testing
                and automation needs. Get up and running in minutes.
              </p>
              <div className="flex gap-4 justify-center">
                <Button asChild size="lg">
                  <Link href="/docs/installation">Install curl-runner</Link>
                </Button>
                <Button asChild variant="outline" size="lg">
                  <Link href="/docs/quick-start">Quick Start Guide</Link>
                </Button>
              </div>
            </div>
          </section>
        </div>
      </div>
    </main>
  );
}
