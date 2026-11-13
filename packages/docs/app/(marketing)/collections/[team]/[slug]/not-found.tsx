import type { Metadata } from 'next';
import { ButtonLink } from '@/components/site/ButtonLink';
import { SiteHeader } from '@/components/site/SiteHeader';

export const metadata: Metadata = {
  title: 'Collection not listed',
  robots: { index: false },
};

export default function CollectionNotFound() {
  return (
    <>
      <SiteHeader active="collections" />
      <main id="main-content" className="mx-auto w-full max-w-[1200px] px-6 py-24 max-sm:py-14">
        <h1 className="m-0 text-[clamp(32px,5vw,48px)] leading-[1.1] font-bold tracking-[-0.035em]">
          This collection isn’t listed
        </h1>
        <p className="mt-5 mb-0 max-w-xl text-lg text-fg-secondary">
          It may have been unlisted or renamed. If someone sent you its share link, open that
          instead.
        </p>
        <div className="mt-8">
          <ButtonLink href="/collections" variant="primary" size="lg">
            Browse collections
          </ButtonLink>
        </div>
      </main>
    </>
  );
}
