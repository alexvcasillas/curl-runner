import type { Metadata } from 'next';

export const SITE_NAME = 'curl-runner';

/** The homepage hero sentence, reused for the site-wide description. */
export const SITE_DESCRIPTION =
  'Describe requests in plain YAML, validate every response, and run whole suites in parallel — on your laptop or in CI. One native binary, no runtime.';

export const SITE_URL = 'https://www.curl-runner.com';

export interface PageMetaInput {
  /** Page name without the site suffix, e.g. "Downloads". */
  title: string;
  description: string;
  /** Absolute path starting with "/". */
  path: string;
  /** Set when the route has its own opengraph-image file; otherwise the site card is used. */
  ownImage?: boolean;
}

/** Title, Open Graph, Twitter and canonical metadata shared by every marketing page. */
export function pageMetadata({ title, description, path, ownImage }: PageMetaInput): Metadata {
  const full = `${title} — ${SITE_NAME}`;
  const url = `${SITE_URL}${path}`;
  const images = ownImage ? undefined : ['/opengraph-image'];
  return {
    title: { absolute: full },
    description,
    alternates: { canonical: url },
    openGraph: { type: 'website', siteName: SITE_NAME, title: full, description, url, images },
    twitter: { card: 'summary_large_image', title: full, description, images },
  };
}
