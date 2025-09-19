import type { Metadata } from 'next';
import Link from 'next/link';
import { ArrowRight, Check, X } from 'lucide-react';
import { DocsPageHeader } from '@/components/docs-page-header';
import { DocumentationArticleSchema, BreadcrumbSchema } from '@/components/structured-data';
import { Button } from '@/components/ui/button';

export const metadata: Metadata = {
  title: 'curl-runner vs Postman, Insomnia, curl - API Testing Tool Comparison',
  description: 'Compare curl-runner with Postman, Insomnia, curl, and HTTPie. See why developers choose curl-runner for CLI-based API testing, automation, and CI/CD integration.',
  keywords: [
    'curl-runner vs Postman',
    'curl-runner vs Insomnia',
    'curl-runner vs curl',
    'curl-runner vs HTTPie',
    'API testing tool comparison',
    'Postman alternative',
    'curl alternative',
    'Insomnia alternative',
    'CLI API testing',
    'command line HTTP client',
    'YAML API testing',
    'API automation tools',
    'CI/CD API testing',
  ],
  openGraph: {
    title: 'curl-runner vs Postman, Insomnia, curl - API Testing Tool Comparison',
    description: 'Compare curl-runner with popular API testing tools. See why developers choose curl-runner for CLI-based API testing and automation.',
    url: 'https://www.curl-runner.com/alternatives',
    type: 'article',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/alternatives',
  },
};

const comparisons = [
  {
    tool: 'Postman',
    description: 'Popular GUI-based API testing platform',
    curlRunnerAdvantages: [
      'CLI-first design for automation',
      'YAML configuration (version controllable)',
      'Built for CI/CD pipelines',
      'No GUI overhead',
      'Parallel execution out of the box',
    ],
    postmanAdvantages: [
      'Visual interface for beginners',
      'Team collaboration features',
      'Built-in API documentation',
      'Large marketplace of collections',
    ],
    useCase: 'Choose curl-runner for automation, CI/CD, and developers who prefer command-line tools.',
  },
  {
    tool: 'curl',
    description: 'Classic command-line HTTP client',
    curlRunnerAdvantages: [
      'YAML configuration vs complex command-line flags',
      'Response validation and assertions',
      'Variable templating and environments',
      'Parallel execution support',
      'Better error handling and reporting',
    ],
    postmanAdvantages: [
      'Ubiquitous - available everywhere',
      'Single command for simple requests',
      'Very lightweight',
      'HTTP standard compliance',
    ],
    useCase: 'Choose curl-runner for complex workflows, testing suites, and better maintainability.',
  },
  {
    tool: 'Insomnia',
    description: 'Desktop API client and design platform',
    curlRunnerAdvantages: [
      'Terminal-based workflow',
      'Configuration as code (YAML)',
      'Perfect for headless environments',
      'Parallel execution',
      'CI/CD integration',
    ],
    postmanAdvantages: [
      'Beautiful desktop interface',
      'GraphQL support',
      'Plugin ecosystem',
      'Design-first approach',
    ],
    useCase: 'Choose curl-runner for server environments, automation, and configuration management.',
  },
  {
    tool: 'HTTPie',
    description: 'Human-friendly command-line HTTP client',
    curlRunnerAdvantages: [
      'Complex workflows and test suites',
      'Response validation and assertions',
      'YAML configuration management',
      'Parallel execution',
      'Environment variables support',
    ],
    postmanAdvantages: [
      'Simple syntax for quick requests',
      'JSON highlighting',
      'Intuitive command structure',
      'Great for learning HTTP',
    ],
    useCase: 'Choose curl-runner for comprehensive testing workflows vs HTTPie for simple requests.',
  },
];

const featureMatrix = [
  { feature: 'CLI Interface', curlRunner: true, postman: false, curl: true, insomnia: false, httpie: true },
  { feature: 'GUI Interface', curlRunner: false, postman: true, curl: false, insomnia: true, httpie: false },
  { feature: 'YAML Configuration', curlRunner: true, postman: false, curl: false, insomnia: false, httpie: false },
  { feature: 'Parallel Execution', curlRunner: true, postman: false, curl: false, insomnia: false, httpie: false },
  { feature: 'Response Validation', curlRunner: true, postman: true, curl: false, insomnia: true, httpie: false },
  { feature: 'Environment Variables', curlRunner: true, postman: true, curl: false, insomnia: true, httpie: false },
  { feature: 'CI/CD Ready', curlRunner: true, postman: false, curl: true, insomnia: false, httpie: true },
  { feature: 'Free & Open Source', curlRunner: true, postman: false, curl: true, insomnia: false, httpie: true },
];

export default function AlternativesPage() {
  return (
    <main className="relative py-6 lg:py-8">
      <DocumentationArticleSchema
        title="curl-runner vs Postman, Insomnia, curl - API Testing Tool Comparison"
        description="Compare curl-runner with Postman, Insomnia, curl, and HTTPie. See why developers choose curl-runner for CLI-based API testing, automation, and CI/CD integration."
        url="https://www.curl-runner.com/alternatives"
        section="Comparison"
      />
      <BreadcrumbSchema
        items={[
          { name: 'Home', url: 'https://www.curl-runner.com' },
          { name: 'Alternatives', url: 'https://www.curl-runner.com/alternatives' },
        ]}
      />

      <div className="mx-auto w-full min-w-0 max-w-4xl px-4 sm:px-6 lg:px-8">
        <DocsPageHeader
          heading="curl-runner vs Alternatives"
          text="Compare curl-runner with popular API testing tools to see which fits your workflow best"
        />

        {/* Quick Comparison Table */}
        <section className="mb-12">
          <h2 className="text-2xl font-semibold mb-6">Feature Comparison</h2>
          <div className="overflow-x-auto">
            <table className="w-full border-collapse border border-gray-200 dark:border-gray-700">
              <thead>
                <tr className="bg-gray-50 dark:bg-gray-900">
                  <th className="border border-gray-200 dark:border-gray-700 p-3 text-left">Feature</th>
                  <th className="border border-gray-200 dark:border-gray-700 p-3 text-center">curl-runner</th>
                  <th className="border border-gray-200 dark:border-gray-700 p-3 text-center">Postman</th>
                  <th className="border border-gray-200 dark:border-gray-700 p-3 text-center">curl</th>
                  <th className="border border-gray-200 dark:border-gray-700 p-3 text-center">Insomnia</th>
                  <th className="border border-gray-200 dark:border-gray-700 p-3 text-center">HTTPie</th>
                </tr>
              </thead>
              <tbody>
                {featureMatrix.map((row) => (
                  <tr key={row.feature}>
                    <td className="border border-gray-200 dark:border-gray-700 p-3 font-medium">
                      {row.feature}
                    </td>
                    <td className="border border-gray-200 dark:border-gray-700 p-3 text-center">
                      {row.curlRunner ? <Check className="h-5 w-5 text-green-600 mx-auto" /> : <X className="h-5 w-5 text-red-500 mx-auto" />}
                    </td>
                    <td className="border border-gray-200 dark:border-gray-700 p-3 text-center">
                      {row.postman ? <Check className="h-5 w-5 text-green-600 mx-auto" /> : <X className="h-5 w-5 text-red-500 mx-auto" />}
                    </td>
                    <td className="border border-gray-200 dark:border-gray-700 p-3 text-center">
                      {row.curl ? <Check className="h-5 w-5 text-green-600 mx-auto" /> : <X className="h-5 w-5 text-red-500 mx-auto" />}
                    </td>
                    <td className="border border-gray-200 dark:border-gray-700 p-3 text-center">
                      {row.insomnia ? <Check className="h-5 w-5 text-green-600 mx-auto" /> : <X className="h-5 w-5 text-red-500 mx-auto" />}
                    </td>
                    <td className="border border-gray-200 dark:border-gray-700 p-3 text-center">
                      {row.httpie ? <Check className="h-5 w-5 text-green-600 mx-auto" /> : <X className="h-5 w-5 text-red-500 mx-auto" />}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        {/* Detailed Comparisons */}
        <section className="space-y-8">
          <h2 className="text-2xl font-semibold">Detailed Comparisons</h2>
          {comparisons.map((comparison) => (
            <div key={comparison.tool} className="border rounded-lg p-6 bg-card">
              <h3 className="text-xl font-semibold mb-2">curl-runner vs {comparison.tool}</h3>
              <p className="text-muted-foreground mb-4">{comparison.description}</p>

              <div className="grid md:grid-cols-2 gap-6">
                <div>
                  <h4 className="font-medium text-green-700 dark:text-green-400 mb-3">
                    Why choose curl-runner:
                  </h4>
                  <ul className="space-y-2">
                    {comparison.curlRunnerAdvantages.map((advantage) => (
                      <li key={advantage} className="flex items-start gap-2 text-sm">
                        <Check className="h-4 w-4 text-green-600 mt-0.5 flex-shrink-0" />
                        {advantage}
                      </li>
                    ))}
                  </ul>
                </div>

                <div>
                  <h4 className="font-medium text-blue-700 dark:text-blue-400 mb-3">
                    {comparison.tool} strengths:
                  </h4>
                  <ul className="space-y-2">
                    {comparison.postmanAdvantages.map((advantage) => (
                      <li key={advantage} className="flex items-start gap-2 text-sm">
                        <ArrowRight className="h-4 w-4 text-blue-600 mt-0.5 flex-shrink-0" />
                        {advantage}
                      </li>
                    ))}
                  </ul>
                </div>
              </div>

              <div className="mt-4 p-3 bg-blue-50 dark:bg-blue-950 rounded">
                <p className="text-sm"><strong>Recommendation:</strong> {comparison.useCase}</p>
              </div>
            </div>
          ))}
        </section>

        {/* CTA */}
        <section className="mt-12 text-center">
          <h2 className="text-2xl font-semibold mb-4">Ready to try curl-runner?</h2>
          <p className="text-muted-foreground mb-6">
            Get started in under 5 minutes with our quick installation guide.
          </p>
          <div className="flex gap-4 justify-center">
            <Button asChild>
              <Link href="/docs/quick-start">
                Get Started <ArrowRight className="ml-2 h-4 w-4" />
              </Link>
            </Button>
            <Button variant="outline" asChild>
              <Link href="/docs/installation">
                Installation Guide
              </Link>
            </Button>
          </div>
        </section>
      </div>
    </main>
  );
}