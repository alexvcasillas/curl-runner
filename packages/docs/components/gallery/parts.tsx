import Link from 'next/link';
import type { ReactNode } from 'react';
import {
  CLOUD_DOCS_PATH,
  collectionHref,
  type GalleryCollection,
  initial,
  plural,
  relativeTime,
} from '@/lib/gallery';

export const LINK =
  'underline underline-offset-2 hover:text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand';

export function TeamTile({ name, size = 24 }: { name: string; size?: 24 | 56 }) {
  return (
    <span
      aria-hidden="true"
      className={`inline-flex shrink-0 items-center justify-center bg-selected font-semibold text-fg-secondary ${
        size === 56 ? 'size-14 rounded-[14px] text-2xl' : 'size-6 rounded-md text-xs'
      }`}
    >
      {initial(name)}
    </span>
  );
}

export function metaLine(c: { requests?: number; version: number; updated_at: string }): string {
  const updated = relativeTime(c.updated_at);
  return [
    c.requests === undefined ? null : plural(c.requests, 'request'),
    `v${c.version}`,
    updated ? `Updated ${updated}` : null,
  ]
    .filter(Boolean)
    .join(' · ');
}

export function CollectionCard({
  collection,
  showTeam = true,
}: {
  collection: GalleryCollection;
  showTeam?: boolean;
}) {
  return (
    <li className="m-0 min-w-0 list-none">
      <Link
        href={collectionHref(collection.team.slug, collection.slug)}
        className="flex h-full min-h-11 flex-col gap-3 rounded-[14px] border border-line p-5 transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand"
      >
        {showTeam ? (
          <span className="flex items-center gap-2 text-sm text-fg-secondary">
            <TeamTile name={collection.team.name} />
            <span className="min-w-0 truncate">{collection.team.name}</span>
          </span>
        ) : null}
        <span className="text-xl leading-[1.3] font-semibold tracking-[-0.01em] break-words text-fg">
          {collection.name}
        </span>
        <span className="line-clamp-2 text-[15px] leading-[1.5] break-words text-fg-secondary">
          {collection.description}
        </span>
        <span className="mt-auto pt-1 font-mono text-[12.5px] text-fg-tertiary">
          {metaLine(collection)}
        </span>
      </Link>
    </li>
  );
}

export function GalleryNotice({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div role="status" className="rounded-[14px] border border-line p-8 max-sm:p-6">
      <h2 className="m-0 text-xl font-semibold tracking-[-0.01em] text-fg">{title}</h2>
      <p className="mt-3 mb-0 max-w-xl text-fg-secondary">{children}</p>
    </div>
  );
}

export function GalleryDown() {
  return (
    <GalleryNotice title="The gallery can’t be reached right now">
      Try again in a minute. Collections you’ve already added keep working in the studio and the
      CLI.
    </GalleryNotice>
  );
}

export function ListYourOwn() {
  return (
    <section
      aria-labelledby="list-own-heading"
      className="rounded-[14px] border border-line p-8 max-sm:p-6"
    >
      <h2 id="list-own-heading" className="m-0 text-xl font-semibold tracking-[-0.01em] text-fg">
        List your own
      </h2>
      <p className="mt-3 mb-0 max-w-2xl text-fg-secondary">
        Publish a collection to curl-runner Cloud, make it Public, give it a description, and turn
        on “List in the gallery”.{' '}
        <Link href={CLOUD_DOCS_PATH} className={`text-fg ${LINK}`}>
          How to publish
        </Link>
      </p>
    </section>
  );
}
