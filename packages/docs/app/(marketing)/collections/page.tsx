import type { Metadata } from 'next';
import Link from 'next/link';
import {
  CollectionCard,
  GalleryDown,
  GalleryNotice,
  LINK,
  ListYourOwn,
} from '@/components/gallery/parts';
import { Eyebrow } from '@/components/site/Eyebrow';
import { SiteHeader } from '@/components/site/SiteHeader';
import { CLOUD_DOCS_PATH, fetchCollections, listMetadata, plural } from '@/lib/gallery';

export const revalidate = 300;

export const metadata: Metadata = listMetadata();

interface Props {
  searchParams: Promise<{ q?: string | string[]; cursor?: string | string[] }>;
}

function first(value: string | string[] | undefined): string {
  return (Array.isArray(value) ? value[0] : value)?.trim() ?? '';
}

export default async function CollectionsPage({ searchParams }: Props) {
  const params = await searchParams;
  const q = first(params.q).slice(0, 100);
  const cursor = first(params.cursor);
  const res = await fetchCollections({ q, cursor });
  const list = res.status === 'ok' ? res.data : null;
  const searching = q !== '';

  const more =
    list?.next_cursor != null
      ? `/collections?${new URLSearchParams({ ...(searching && { q }), cursor: list.next_cursor })}`
      : null;

  return (
    <>
      <SiteHeader active="collections" />
      <main id="main-content">
        <section className="mx-auto max-w-[1200px] px-6 pt-24 pb-10 max-sm:pt-14">
          <Eyebrow>COLLECTIONS</Eyebrow>
          <h1 className="mt-4 mb-0 text-[clamp(40px,6vw,72px)] leading-[1.02] font-bold tracking-[-0.045em]">
            APIs, ready to run.
          </h1>
          <p className="mt-6 mb-0 max-w-160 text-[19px] leading-[1.6] text-fg-secondary max-sm:text-[17px]">
            Request collections that teams publish with curl-runner. Read the YAML here, open one in
            the studio, or pull it with the CLI. You fill in your own keys; nothing is shared back.
          </p>
          <form
            action="/collections"
            method="get"
            role="search"
            className="mt-8 flex max-w-xl gap-2"
          >
            <input
              type="search"
              name="q"
              defaultValue={q}
              placeholder="Search by name, description or team"
              aria-label="Search collections"
              maxLength={100}
              className="h-11 min-w-0 grow rounded-[10px] border border-line bg-surface px-4 text-[15px] text-fg placeholder:text-fg-tertiary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand"
            />
            <button
              type="submit"
              className="h-11 shrink-0 cursor-pointer rounded-[10px] border border-line px-4 text-[15px] text-fg transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand"
            >
              Search
            </button>
          </form>
          {list && list.collections.length > 0 ? (
            <p className="mt-4 mb-0 text-sm text-fg-tertiary">
              {searching
                ? plural(list.total, 'collection')
                : `${plural(list.total, 'collection')} from ${plural(list.teams, 'team')}`}
            </p>
          ) : null}
        </section>

        <section aria-label="Collections" className="mx-auto max-w-[1200px] px-6 pb-14">
          {list === null ? (
            <GalleryDown />
          ) : list.collections.length > 0 ? (
            <>
              <ul className="m-0 grid list-none grid-cols-[repeat(auto-fill,minmax(min(100%,340px),1fr))] gap-4 p-0">
                {list.collections.map((c) => (
                  <CollectionCard key={`${c.team.slug}/${c.slug}`} collection={c} />
                ))}
              </ul>
              {more || cursor ? (
                <p className="mt-8 mb-0 flex flex-wrap gap-x-6">
                  {more ? (
                    <Link
                      href={more}
                      className="inline-flex h-11 items-center rounded-[10px] border border-line px-5 text-[15px] text-fg transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand"
                    >
                      Show more
                    </Link>
                  ) : null}
                  {cursor ? (
                    <Link
                      href={searching ? `/collections?q=${encodeURIComponent(q)}` : '/collections'}
                      className={`inline-flex h-11 items-center text-fg-secondary ${LINK}`}
                    >
                      Back to the first collections
                    </Link>
                  ) : null}
                </p>
              ) : null}
            </>
          ) : searching ? (
            <GalleryNotice title={`No collections match “${q}”`}>
              Search looks at names, descriptions and teams. Try a shorter word, or{' '}
              <Link href="/collections" className={`text-fg ${LINK}`}>
                see every collection
              </Link>
              .
            </GalleryNotice>
          ) : (
            <GalleryNotice title="No collections yet">
              Teams list their public collections here from curl-runner Cloud.{' '}
              <Link href={CLOUD_DOCS_PATH} className={`text-fg ${LINK}`}>
                Publish yours
              </Link>
              .
            </GalleryNotice>
          )}
        </section>

        <div className="mx-auto max-w-[1200px] px-6 pb-24">
          <ListYourOwn />
        </div>
      </main>
    </>
  );
}
