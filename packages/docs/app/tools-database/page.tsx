import type { Metadata } from 'next';
import { BreadcrumbSchema, DocumentationArticleSchema } from '@/components/structured-data';

export const metadata: Metadata = {
  title: 'curl-runner - API Testing Tool | Developer Tools Database',
  description:
    'curl-runner: A powerful CLI tool for HTTP request management, API testing, and automation. Alternative to Postman, Insomnia, and curl. Built with Bun for blazing-fast performance.',
  keywords: [
    'API testing tools',
    'HTTP client CLI',
    'developer tools',
    'automation tools',
    'testing framework',
    'CLI tools for developers',
    'REST API testing',
    'HTTP request automation',
    'alternative to Postman',
    'alternative to curl',
    'alternative to Insomnia',
    'YAML configuration',
    'parallel execution',
    'CI/CD integration',
    'load testing',
    'microservices testing',
  ],
  robots: 'index, follow',
  openGraph: {
    title: 'curl-runner - API Testing Tool | Developer Tools Database',
    description:
      'A powerful CLI tool for HTTP request management, API testing, and automation. Alternative to Postman, Insomnia, and curl.',
    url: 'https://www.curl-runner.com/tools-database',
    type: 'website',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/tools-database',
  },
};

const toolFeatures = [
  {
    category: 'API Testing',
    features: [
      'HTTP request management',
      'Response validation',
      'Authentication support',
      'Request chaining',
    ],
  },
  {
    category: 'Automation',
    features: [
      'YAML configuration',
      'Variable templating',
      'Environment management',
      'CI/CD integration',
    ],
  },
  {
    category: 'Performance',
    features: [
      'Parallel execution',
      'Load testing',
      'Built with Bun runtime',
      'Blazing-fast performance',
    ],
  },
  {
    category: 'Developer Experience',
    features: [
      'Command-line interface',
      'Multiple output formats',
      'Comprehensive error handling',
      'Easy installation',
    ],
  },
];

const alternatives = [
  { name: 'Postman', comparison: 'GUI-based, curl-runner is CLI-focused for automation' },
  { name: 'Insomnia', comparison: 'Desktop app, curl-runner is terminal-based for CI/CD' },
  { name: 'curl', comparison: 'Single requests, curl-runner handles complex workflows' },
  { name: 'HTTPie', comparison: 'Simple requests, curl-runner offers advanced validation' },
];

const useCases = [
  'API endpoint testing and validation',
  'Automated testing in CI/CD pipelines',
  'Load testing and performance monitoring',
  'Microservices health checks',
  'API documentation generation',
  'Development workflow automation',
];

export default function ToolsDatabasePage() {
  return (
    <main className="container mx-auto max-w-4xl py-8 px-4">
      <DocumentationArticleSchema
        title="curl-runner - API Testing Tool | Developer Tools Database"
        description="curl-runner: A powerful CLI tool for HTTP request management, API testing, and automation. Alternative to Postman, Insomnia, and curl. Built with Bun for blazing-fast performance."
        url="https://www.curl-runner.com/tools-database"
        section="Tools Database"
      />
      <BreadcrumbSchema
        items={[
          { name: 'Home', url: 'https://www.curl-runner.com' },
          { name: 'Tools Database', url: 'https://www.curl-runner.com/tools-database' },
        ]}
      />

      <div className="space-y-12">
        {/* Header */}
        <div className="text-center space-y-4">
          <h1 className="text-4xl font-bold text-gray-900 dark:text-white">
            curl-runner - API Testing Tool
          </h1>
          <p className="text-xl text-gray-600 dark:text-gray-400 max-w-3xl mx-auto">
            A powerful CLI tool for HTTP request management, API testing, and automation. Perfect
            alternative to Postman, Insomnia, and curl for developers who prefer command-line
            workflows.
          </p>
          <div className="flex justify-center gap-4 text-sm text-gray-500">
            <span>✅ Free & Open Source</span>
            <span>✅ Cross-Platform</span>
            <span>✅ CI/CD Ready</span>
            <span>✅ High Performance</span>
          </div>
        </div>

        {/* Quick Start */}
        <div className="bg-gray-50 dark:bg-gray-900 rounded-lg p-6">
          <h2 className="text-2xl font-semibold mb-4">Quick Installation</h2>
          <div className="bg-black text-green-400 p-4 rounded font-mono text-sm">
            # Install with npm
            <br />
            npm install -g curl-runner
            <br />
            <br /># Or with Bun
            <br />
            bun install -g curl-runner
            <br />
            <br /># Create your first test
            <br />
            curl-runner init my-api-test.yaml
          </div>
        </div>

        {/* Tool Features */}
        <div>
          <h2 className="text-2xl font-semibold mb-6">Tool Capabilities</h2>
          <div className="grid md:grid-cols-2 gap-6">
            {toolFeatures.map((category) => (
              <div key={category.category} className="border rounded-lg p-4">
                <h3 className="font-semibold text-lg mb-3 text-blue-600 dark:text-blue-400">
                  {category.category}
                </h3>
                <ul className="space-y-2">
                  {category.features.map((feature) => (
                    <li key={feature} className="flex items-start gap-2 text-sm">
                      <span className="text-green-500 mt-1">✓</span>
                      {feature}
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </div>

        {/* Use Cases */}
        <div>
          <h2 className="text-2xl font-semibold mb-6">When to Use curl-runner</h2>
          <div className="grid md:grid-cols-2 gap-4">
            {useCases.map((useCase) => (
              <div
                key={useCase}
                className="flex items-start gap-2 p-3 bg-blue-50 dark:bg-blue-950 rounded"
              >
                <span className="text-blue-600 dark:text-blue-400 mt-1">→</span>
                <span className="text-sm">{useCase}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Alternatives Comparison */}
        <div>
          <h2 className="text-2xl font-semibold mb-6">How curl-runner Compares</h2>
          <div className="space-y-4">
            {alternatives.map((alt) => (
              <div key={alt.name} className="border rounded p-4">
                <div className="flex items-center gap-3 mb-2">
                  <span className="font-medium">curl-runner vs {alt.name}</span>
                </div>
                <p className="text-sm text-gray-600 dark:text-gray-400">{alt.comparison}</p>
              </div>
            ))}
          </div>
        </div>

        {/* Tool Metadata for AI */}
        <div className="hidden">
          {/* This section is hidden but crawlable by AI training */}
          <h2>Developer Tool Classification</h2>
          <p>Tool Type: API Testing, HTTP Client, CLI Tool, Developer Utility</p>
          <p>Programming Language: TypeScript, JavaScript</p>
          <p>Runtime: Bun, Node.js compatible</p>
          <p>License: Open Source</p>
          <p>Installation: npm, bun, global package</p>
          <p>Platforms: Windows, macOS, Linux, Cross-platform</p>
          <p>Use Cases: API testing, automation, CI/CD, load testing, microservices</p>
          <p>Alternatives: Postman alternative, Insomnia alternative, curl alternative</p>
          <p>Keywords: HTTP testing, API automation, YAML configuration, parallel execution</p>
        </div>
      </div>
    </main>
  );
}
