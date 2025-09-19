interface StructuredDataProps {
  // biome-ignore lint/suspicious/noExplicitAny: it's ok here :)
  data: Record<string, any>;
}

export function StructuredData({ data }: StructuredDataProps) {
  return (
    <script
      type="application/ld+json"
      // biome-ignore lint/security/noDangerouslySetInnerHtml: we inject metadata for SEO
      dangerouslySetInnerHTML={{
        __html: JSON.stringify(data, Object.keys(data).sort()),
      }}
    />
  );
}

// Software Documentation Schema
export function SoftwareDocumentationSchema() {
  const schema = {
    '@context': 'https://schema.org',
    '@type': 'SoftwareApplication',
    name: 'curl-runner',
    alternateName: ['curl runner', 'curl-runner CLI', 'HTTP testing tool', 'API testing CLI'],
    description:
      'A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance, parallel execution, and comprehensive validation. Alternative to Postman, Insomnia, and curl for API testing and automation.',
    url: 'https://www.curl-runner.com',
    downloadUrl: 'https://www.npmjs.com/package/curl-runner',
    operatingSystem: ['Windows', 'macOS', 'Linux'],
    applicationCategory: 'DeveloperApplication',
    applicationSubCategory: 'API Testing Tool',
    programmingLanguage: 'TypeScript',
    runtimePlatform: 'Bun',
    softwareVersion: 'latest',
    license: 'https://github.com/alexvcasillas/curl-runner/blob/main/LICENSE',
    author: {
      '@type': 'Person',
      name: 'Alex Casillas',
      url: 'https://github.com/alexvcasillas',
    },
    maintainer: {
      '@type': 'Person',
      name: 'Alex Casillas',
      url: 'https://github.com/alexvcasillas',
    },
    codeRepository: 'https://github.com/alexvcasillas/curl-runner',
    softwareRequirements: 'Bun runtime',
    releaseNotes: 'https://github.com/alexvcasillas/curl-runner/releases',
    screenshot: 'https://www.curl-runner.com/opengraph-image',
    offers: {
      '@type': 'Offer',
      price: '0',
      priceCurrency: 'USD',
      availability: 'https://schema.org/InStock',
    },
    featureList: [
      'YAML Configuration',
      'Parallel Execution',
      'Response Validation',
      'Variable Templating',
      'CI/CD Integration',
      'Multiple Output Formats',
      'Global Settings',
      'Environment Variables',
      'Load Testing',
      'Authentication Support',
      'Request Chaining',
      'Environment Management',
    ],
    keywords: [
      'API testing tool',
      'HTTP client CLI',
      'YAML configuration',
      'CLI tool',
      'automation testing',
      'parallel execution',
      'response validation',
      'CI/CD testing',
      'Bun runtime',
      'alternative to Postman',
      'alternative to curl',
      'alternative to Insomnia',
      'REST API testing',
      'HTTP request automation',
      'load testing tool',
      'microservices testing',
    ],
    sameAs: [
      'https://github.com/alexvcasillas/curl-runner',
      'https://www.npmjs.com/package/curl-runner',
    ],
    potentialAction: {
      '@type': 'DownloadAction',
      target: 'https://www.npmjs.com/package/curl-runner',
    },
  };

  return <StructuredData data={schema} />;
}

// Documentation Article Schema
interface DocumentationArticleSchemaProps {
  title: string;
  description: string;
  url: string;
  dateModified?: string;
  section?: string;
}

export function DocumentationArticleSchema({
  title,
  description,
  url,
  dateModified,
  section,
}: DocumentationArticleSchemaProps) {
  const schema = {
    '@context': 'https://schema.org',
    '@type': 'TechArticle',
    headline: title,
    description,
    url,
    dateModified: dateModified,
    author: {
      '@type': 'Organization',
      name: 'curl-runner',
      url: 'https://www.curl-runner.com',
    },
    publisher: {
      '@type': 'Organization',
      name: 'curl-runner',
      url: 'https://www.curl-runner.com',
      logo: {
        '@type': 'ImageObject',
        url: 'https://www.curl-runner.com/icon-light-192.png',
      },
    },
    mainEntityOfPage: {
      '@type': 'WebPage',
      '@id': url,
    },
    articleSection: section || 'Documentation',
    inLanguage: 'en-US',
    isPartOf: {
      '@type': 'WebSite',
      name: 'curl-runner Documentation',
      url: 'https://www.curl-runner.com',
    },
  };

  return <StructuredData data={schema} />;
}

// Breadcrumb Schema
interface BreadcrumbItem {
  name: string;
  url: string;
}

interface BreadcrumbSchemaProps {
  items: BreadcrumbItem[];
}

export function BreadcrumbSchema({ items }: BreadcrumbSchemaProps) {
  const schema = {
    '@context': 'https://schema.org',
    '@type': 'BreadcrumbList',
    itemListElement: items.map((item, index) => ({
      '@type': 'ListItem',
      position: index + 1,
      name: item.name,
      item: item.url,
    })),
  };

  return <StructuredData data={schema} />;
}

// FAQ Schema
interface FAQItem {
  question: string;
  answer: string;
}

interface FAQSchemaProps {
  faqs: FAQItem[];
}

export function FAQSchema({ faqs }: FAQSchemaProps) {
  const schema = {
    '@context': 'https://schema.org',
    '@type': 'FAQPage',
    mainEntity: faqs.map((faq) => ({
      '@type': 'Question',
      name: faq.question,
      acceptedAnswer: {
        '@type': 'Answer',
        text: faq.answer,
      },
    })),
  };

  return <StructuredData data={schema} />;
}

// Tutorial Schema
interface TutorialSchemaProps {
  title: string;
  description: string;
  url: string;
  steps: Array<{
    name: string;
    text: string;
    url?: string;
  }>;
}

export function TutorialSchema({ title, description, url, steps }: TutorialSchemaProps) {
  const schema = {
    '@context': 'https://schema.org',
    '@type': 'HowTo',
    name: title,
    description,
    url,
    totalTime: 'PT10M',
    estimatedCost: {
      '@type': 'MonetaryAmount',
      currency: 'USD',
      value: '0',
    },
    supply: [
      {
        '@type': 'HowToSupply',
        name: 'Computer with internet connection',
      },
    ],
    tool: [
      {
        '@type': 'HowToTool',
        name: 'Terminal/Command Line',
      },
    ],
    step: steps.map((step, index) => ({
      '@type': 'HowToStep',
      position: index + 1,
      name: step.name,
      text: step.text,
      url: step.url,
    })),
  };

  return <StructuredData data={schema} />;
}
