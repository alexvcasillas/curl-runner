import type { LucideIcon } from 'lucide-react';
import { BookOpen } from 'lucide-react';

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
      title: 'Documentation',
      href: '/docs',
      icon: BookOpen,
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
          title: 'Retry Mechanism',
          href: '/docs/features/retry-mechanism',
          description: 'Automatic retry with configurable delays',
        },
        {
          title: 'Output Formats',
          href: '/docs/features/output-formats',
          description: 'Control output format and logging',
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
