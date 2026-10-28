import type { Metadata } from 'next';
import Link from 'next/link';
import { notFound } from 'next/navigation';
import { ButtonLink } from '@/components/site/ButtonLink';
import { CodePane } from '@/components/site/CodePane';
import { Eyebrow } from '@/components/site/Eyebrow';
import { SiteHeader } from '@/components/site/SiteHeader';
import { Yaml } from '@/components/site/Yaml';
import {
  type FeatureData,
  getAdjacentFeatures,
  getAllFeatures,
  getFeatureBySlug,
} from '@/lib/features-data';

const BASE_URL = 'https://www.curl-runner.com';

interface FeaturePageProps {
  params: Promise<{ slug: string }>;
}

export const dynamicParams = false;

export function generateStaticParams() {
  return getAllFeatures().map((feature) => ({ slug: feature.slug }));
}

export async function generateMetadata({ params }: FeaturePageProps): Promise<Metadata> {
  const { slug } = await params;
  const feature = getFeatureBySlug(slug);
  if (!feature) {
    return {};
  }
  const title = `${feature.seoTitle ?? feature.title} — curl-runner`;
  const url = `${BASE_URL}/features/${feature.slug}`;
  return {
    title: { absolute: title },
    description: feature.description,
    keywords: ['curl-runner', feature.title, ...feature.keywords],
    openGraph: { title, description: feature.description, url, type: 'website' },
    twitter: { card: 'summary_large_image', title, description: feature.headline },
    alternates: { canonical: url },
  };
}

function FeatureNav({ current }: { current: string }) {
  return (
    <nav aria-label="Features" className="order-last md:order-none md:pt-[72px]">
      <p className="m-0 mb-3.5 font-mono text-xs tracking-[0.06em] text-fg-tertiary">FEATURES</p>
      <ul className="m-0 flex list-none flex-col gap-0.5 p-0 text-sm">
        {getAllFeatures().map((f) => {
          const isCurrent = f.slug === current;
          return (
            <li key={f.slug}>
              <Link
                href={`/features/${f.slug}`}
                aria-current={isCurrent ? 'page' : undefined}
                className={`block rounded-lg px-3 py-2.5 transition-colors focus-visible:outline-2 focus-visible:outline-brand md:py-2 ${
                  isCurrent ? 'bg-hover text-fg' : 'text-fg-secondary hover:text-fg'
                }`}
              >
                {f.title}
              </Link>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}

function NeighbourCard({
  feature,
  direction,
}: {
  feature: FeatureData;
  direction: 'prev' | 'next';
}) {
  const isPrev = direction === 'prev';
  return (
    <Link
      href={`/features/${feature.slug}`}
      rel={direction}
      className={`flex flex-[1_1_260px] flex-col gap-1.5 rounded-xl border border-line p-[22px] transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-brand ${
        isPrev ? '' : 'text-right'
      }`}
    >
      <span className="text-[13px] text-fg-tertiary">{isPrev ? 'Previous' : 'Next'}</span>
      <span className="text-[17px] font-semibold text-fg">
        {isPrev ? <span aria-hidden="true">← </span> : null}
        {feature.title}
        {isPrev ? null : <span aria-hidden="true"> →</span>}
      </span>
    </Link>
  );
}

export default async function FeaturePage({ params }: FeaturePageProps) {
  const { slug } = await params;
  const feature = getFeatureBySlug(slug);
  if (!feature) {
    notFound();
  }
  const { prev, next } = getAdjacentFeatures(feature.slug);

  return (
    <>
      <SiteHeader active="features" />
      <main id="main-content">
        <div className="mx-auto grid max-w-[1200px] gap-12 px-6 md:grid-cols-[220px_minmax(0,1fr)]">
          <FeatureNav current={feature.slug} />

          <article className="min-w-0 pt-16 pb-24 md:pt-[72px]">
            <Eyebrow>{feature.title}</Eyebrow>
            <h1 className="mt-4 mb-0 max-w-[18ch] text-[clamp(40px,5.6vw,64px)] leading-[1.03] font-bold tracking-[-0.045em] text-fg">
              {feature.headline}
            </h1>
            <p className="mt-6 mb-0 max-w-160 text-[19px] leading-[1.6] text-fg-secondary">
              {feature.description}
            </p>
            <div className="mt-8 flex flex-wrap gap-3">
              <ButtonLink href={`/docs/features/${feature.slug}`} variant="primary" size="lg">
                Read the docs
              </ButtonLink>
              <ButtonLink href="/downloads" variant="secondary" size="lg">
                Install curl-runner
              </ButtonLink>
            </div>

            <div className="mt-16">
              <CodePane filename={feature.example.filename} copyText={feature.example.yaml}>
                <Yaml source={feature.example.yaml} />
              </CodePane>
            </div>

            <section aria-labelledby="benefits-heading" className="mt-20">
              <h2 id="benefits-heading" className="sr-only">
                Benefits
              </h2>
              <ul className="m-0 grid list-none grid-cols-[repeat(auto-fit,minmax(min(100%,240px),1fr))] border-t border-l border-hairline p-0">
                {feature.benefits.map((benefit) => (
                  <li key={benefit.title} className="border-r border-b border-hairline p-7">
                    <h3 className="m-0 mb-2.5 text-[17px] font-semibold text-fg">
                      {benefit.title}
                    </h3>
                    <p className="m-0 text-[15px] leading-[1.6] text-fg-secondary">
                      {benefit.description}
                    </p>
                  </li>
                ))}
              </ul>
            </section>

            {feature.fromCurl ? (
              <section aria-labelledby="from-curl-heading">
                <h2
                  id="from-curl-heading"
                  className="mt-20 mb-6 text-[28px] font-bold tracking-[-0.025em] text-fg"
                >
                  {feature.fromCurl.heading}
                </h2>
                <p className="m-0 max-w-160 text-base leading-[1.6] text-fg-secondary">
                  {feature.fromCurl.intro}
                </p>
                <div className="mt-6 flex flex-col gap-4">
                  <CodePane filename="curl" copyText={feature.fromCurl.curl}>
                    <pre className="m-0 overflow-x-auto p-5 font-mono text-[13px] leading-[1.7] text-fg">
                      {feature.fromCurl.curl}
                    </pre>
                  </CodePane>
                  <CodePane filename="retry-from-curl.yaml" copyText={feature.fromCurl.yaml}>
                    <Yaml source={feature.fromCurl.yaml} />
                  </CodePane>
                </div>
                <p className="mt-6 mb-0 max-w-160 text-base leading-[1.6] text-fg-secondary">
                  {feature.fromCurl.outro}{' '}
                  <a
                    href={feature.fromCurl.href}
                    className="text-fg underline underline-offset-4 focus-visible:outline-2 focus-visible:outline-brand"
                  >
                    {feature.fromCurl.linkText}
                  </a>
                  .
                </p>
              </section>
            ) : null}

            <section aria-labelledby="good-for-heading">
              <h2
                id="good-for-heading"
                className="mt-20 mb-6 text-[28px] font-bold tracking-[-0.025em] text-fg"
              >
                Good for
              </h2>
              <ol className="m-0 flex list-none flex-col border-t border-hairline p-0">
                {feature.goodFor.map((item, i) => (
                  <li
                    key={item}
                    className="flex gap-4 border-b border-hairline py-[18px] text-base text-fg"
                  >
                    <span aria-hidden="true" className="font-mono text-fg-tertiary">
                      {String(i + 1).padStart(2, '0')}
                    </span>
                    {item}
                  </li>
                ))}
              </ol>
            </section>

            {prev || next ? (
              <nav aria-label="More features" className="mt-20 flex flex-wrap gap-4">
                {prev ? <NeighbourCard feature={prev} direction="prev" /> : null}
                {next ? <NeighbourCard feature={next} direction="next" /> : null}
              </nav>
            ) : null}
          </article>
        </div>
      </main>
    </>
  );
}
