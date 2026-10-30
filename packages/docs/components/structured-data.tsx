import { getAllFeatures } from '@/lib/features-data';
import { SITE_DESCRIPTION } from '@/lib/site';
import { SITE_URL } from '@/lib/source';

const REPO_URL = 'https://github.com/alexvcasillas/curl-runner';

interface StructuredDataProps {
  data: Record<string, unknown>;
}

export function StructuredData({ data }: StructuredDataProps) {
  return (
    <script
      type="application/ld+json"
      // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON-LD for SEO, built from static site data
      dangerouslySetInnerHTML={{ __html: JSON.stringify(data).replace(/</g, '\\u003c') }}
    />
  );
}

/** SoftwareApplication JSON-LD for the homepage. Only facts the site itself states. */
export function SoftwareApplicationSchema() {
  const schema = {
    '@context': 'https://schema.org',
    '@type': 'SoftwareApplication',
    name: 'curl-runner',
    description: SITE_DESCRIPTION,
    url: SITE_URL,
    downloadUrl: `${SITE_URL}/downloads`,
    applicationCategory: 'DeveloperApplication',
    operatingSystem: ['macOS', 'Linux', 'Windows'],
    license: `${REPO_URL}/blob/main/LICENSE.md`,
    codeRepository: REPO_URL,
    author: {
      '@type': 'Person',
      name: 'Alex Casillas',
      url: 'https://github.com/alexvcasillas',
    },
    offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
    // The CLI's features: a preview (the macOS-only studio) isn't part of this application yet.
    featureList: getAllFeatures()
      .filter((feature) => !feature.preview)
      .map((feature) => feature.title),
    sameAs: [REPO_URL],
  };

  return <StructuredData data={schema} />;
}
