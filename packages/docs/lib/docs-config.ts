import type { LucideIcon } from 'lucide-react';
import { BookOpen, Download, Zap } from 'lucide-react';

export interface NavItem {
  title: string;
  href?: string;
  description?: string;
  icon?: LucideIcon;
  items?: NavItem[];
}

export interface SidebarConfig {
  mainNav: NavItem[];
  sidebarNav: NavItem[];
}

export const docsConfig: SidebarConfig = {
  mainNav: [
    {
      title: 'Features',
      href: '/#features',
      icon: Zap,
    },
    {
      title: 'Documentation',
      href: '/docs',
      icon: BookOpen,
    },
    {
      title: 'Downloads',
      href: '/downloads',
      icon: Download,
    },
  ],
  sidebarNav: [
    {
      title: 'Getting Started',
      items: [
        {
          title: 'Introduction',
          href: '/docs',
          description: 'Learn about curl-runner and its capabilities',
        },
        {
          title: 'Installation',
          href: '/docs/installation',
          description: 'How to install and set up curl-runner',
        },
        {
          title: 'Quick Start',
          href: '/docs/quick-start',
          description: 'Get up and running in minutes',
        },
        {
          title: 'Use Cases',
          href: '/docs/use-cases',
          description: 'Real-world applications and examples',
        },
        {
          title: 'Tutorials',
          href: '/docs/tutorials',
          description: 'Step-by-step guides and walkthroughs',
        },
      ],
    },
    {
      title: 'Configuration',
      items: [
        {
          title: 'YAML Structure',
          href: '/docs/yaml-structure',
          description: 'Understanding YAML configuration files',
        },
        {
          title: 'Variables',
          href: '/docs/variables',
          description: 'Using variables and interpolation',
        },
        {
          title: 'Global Settings',
          href: '/docs/global-settings',
          description: 'Configure global execution settings',
        },
      ],
    },
    {
      title: 'Features',
      items: [
        {
          title: 'Response Storage',
          href: '/docs/features/response-storage',
          description: 'Store and reuse response values across requests',
        },
        {
          title: 'Conditional Execution',
          href: '/docs/features/conditional-execution',
          description: 'Skip or run requests based on previous results',
        },
        {
          title: 'File Uploads',
          href: '/docs/features/file-uploads',
          description: 'Upload files using multipart/form-data requests',
        },
        {
          title: 'Parallel Execution',
          href: '/docs/features/parallel-execution',
          description: 'Execute multiple requests simultaneously',
        },
        {
          title: 'Response Validation',
          href: '/docs/features/response-validation',
          description: 'Validate API responses',
        },
        {
          title: 'Snapshot Testing',
          href: '/docs/features/snapshots',
          description: 'Save and compare response snapshots for regression testing',
        },
        {
          title: 'Response Diffing',
          href: '/docs/features/response-diffing',
          description: 'Compare responses between environments to detect API drift',
        },
        {
          title: 'Retry Mechanism',
          href: '/docs/features/retry-mechanism',
          description: 'Automatic retry with exponential backoff',
        },
        {
          title: 'Watch Mode',
          href: '/docs/features/watch-mode',
          description: 'Auto re-run requests on file changes',
        },
        {
          title: 'Dry Run Mode',
          href: '/docs/features/dry-run',
          description: 'Preview curl commands without executing',
        },
        {
          title: 'HTTP/2 Support',
          href: '/docs/features/http2',
          description: 'Enable HTTP/2 protocol with multiplexing',
        },
        {
          title: 'Connection Pooling',
          href: '/docs/features/connection-pooling',
          description: 'Reuse TCP connections with HTTP/2 multiplexing',
        },
        {
          title: 'Output Formats',
          href: '/docs/features/output-formats',
          description: 'Control output format and logging',
        },
        {
          title: 'CI/CD Integration',
          href: '/docs/features/ci-integration',
          description: 'CI-friendly exit codes for pipeline integration',
        },
        {
          title: 'Performance Profiling',
          href: '/docs/features/performance-profiling',
          description: 'Run requests N times for p50/p95/p99 latency stats',
        },
        {
          title: 'YAML Wizard',
          href: '/docs/features/yaml-wizard',
          description: 'Interactive CLI for creating YAML configuration files',
        },
        {
          title: 'YAML Validation',
          href: '/docs/features/validate',
          description: 'Validate YAML configs with auto-fix and security checks',
        },
        {
          title: 'Environment Files',
          href: '/docs/features/env-files',
          description: 'Load .env files with automatic secret redaction',
        },
      ],
    },
    {
      title: 'CLI Reference',
      items: [
        {
          title: 'Commands',
          href: '/docs/cli-commands',
          description: 'Complete CLI command reference',
        },
        {
          title: 'Options',
          href: '/docs/cli-options',
          description: 'All available command-line options',
        },
        {
          title: 'Environment Variables',
          href: '/docs/environment-variables',
          description: 'Configure behavior with CURL_RUNNER_* variables',
        },
        {
          title: 'Upgrade',
          href: '/docs/features/upgrade',
          description: 'Upgrade curl-runner to the latest version',
        },
      ],
    },
    {
      title: 'Examples',
      items: [
        {
          title: 'Basic Usage',
          href: '/docs/examples/basic',
          description: 'Simple request examples',
        },
        {
          title: 'Collection',
          href: '/docs/examples/collection',
          description: 'Managing request collections',
        },
        {
          title: 'Advanced',
          href: '/docs/examples/advanced',
          description: 'Complex scenarios and workflows',
        },
      ],
    },
    {
      title: 'API Reference',
      items: [
        {
          title: 'Request Object',
          href: '/docs/api-reference/request-object',
          description: 'RequestConfig interface documentation',
        },
        {
          title: 'Response Object',
          href: '/docs/api-reference/response-object',
          description: 'ExecutionResult and ExecutionSummary',
        },
        {
          title: 'Global Settings',
          href: '/docs/api-reference/global-settings',
          description: 'GlobalConfig interface documentation',
        },
        {
          title: 'Validation Rules',
          href: '/docs/api-reference/validation-rules',
          description: 'ExpectConfig and validation patterns',
        },
      ],
    },
  ],
};

export const exampleCategories = [
  {
    title: 'Basic Examples',
    description: 'Simple HTTP request configurations',
    examples: [
      { title: 'Simple GET Request', slug: 'simple-get' },
      { title: 'POST with Body', slug: 'post-with-body' },
      { title: 'Headers and Authentication', slug: 'headers-auth' },
    ],
  },
  {
    title: 'Collections',
    description: 'Managing multiple requests',
    examples: [
      { title: 'Request Collection', slug: 'collection' },
      { title: 'Parallel Execution', slug: 'parallel' },
      { title: 'Variables Usage', slug: 'variables' },
    ],
  },
  {
    title: 'Advanced',
    description: 'Complex scenarios and configurations',
    examples: [
      { title: 'Retry Logic', slug: 'retry-logic' },
      { title: 'Response Validation', slug: 'response-validation' },
      { title: 'Environment Variables', slug: 'env-variables' },
    ],
  },
];
