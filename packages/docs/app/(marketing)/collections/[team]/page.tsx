import type { Metadata } from 'next';
import Link from 'next/link';
import { notFound } from 'next/navigation';
import { CollectionCard, GalleryDown, LINK, TeamTile } from '@/components/gallery/parts';
import { SiteHeader } from '@/components/site/SiteHeader';
import { fetchCollections, isSlug, plural, teamMetadata } from '@/lib/gallery';

export const revalidate = 300;

interface Props {
  params: Promise<{ team: string }>;
  searchParams: Promise<{ cursor?: string | string[] }>;
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { team } = await params;
  if (!isSlug(team)) {
    return {};
  }
  const res = await fetchCollections({ team });
  if (res.status !== 'ok' || res.data.collections.length === 0) {
    return {};
  }
  return teamMetadata(res.data.collections[0].team, res.data.total);
}

export default async function TeamPage({ params, searchParams }: Props) {
  const { team: slug } = await params;
  if (!isSlug(slug)) {
    notFound();
  }
  const cursorParam = (await searchParams).cursor;
  const cursor = (Array.isArray(cursorParam) ? cursorParam[0] : cursorParam) ?? '';
  const res = await fetchCollections({ team: slug, cursor });

  if (res.status === 'down') {
    return (
      <>
        <SiteHeader active="collections" />
        <main id="main-content" className="mx-auto max-w-[1200px] px-6 py-24 max-sm:py-14">
          <GalleryDown />
        </main>
      </>
    );
  }
  if (res.status === 'not_found' || res.data.collections.length === 0) {
    notFound();
  }

  const { collections, total, next_cursor } = res.data;
  const team = collections[0].team;

  return (
    <>
      <SiteHeader active="collections" />
      <main id="main-content" className="mx-auto max-w-[1200px] px-6 pt-14 pb-24">
        <nav aria-label="Breadcrumb" className="text-sm text-fg-secondary">
          <Link href="/collections" className="inline-flex h-11 items-center hover:text-fg">
            Collections
          </Link>
          <span aria-hidden="true" className="mx-2 text-fg-tertiary">
            /
          </span>
          <span aria-current="page" className="text-fg">
            {team.name}
          </span>
        </nav>
        <div className="mt-4 flex items-center gap-5">
          <TeamTile name={team.name} size={56} />
          <div className="min-w-0">
            <h1 className="m-0 text-[clamp(30px,5vw,44px)] leading-[1.1] font-bold tracking-[-0.035em] break-words">
              {team.name}
            </h1>
            <p className="mt-2 mb-0 text-sm break-all text-fg-tertiary">
              {plural(total, 'public collection')} · curl-runner.com/collections/{team.slug}
            </p>
          </div>
        </div>
        <ul className="m-0 mt-10 grid list-none grid-cols-2 gap-4 p-0 max-sm:grid-cols-1">
          {collections.map((c) => (
            <CollectionCard key={c.slug} collection={c} showTeam={false} />
          ))}
        </ul>
        {next_cursor ? (
          <p className="mt-8 mb-0">
            <Link
              href={`/collections/${team.slug}?cursor=${encodeURIComponent(next_cursor)}`}
              className="inline-flex h-11 items-center rounded-[10px] border border-line px-5 text-[15px] text-fg transition-colors hover:bg-hover"
            >
              Show more
            </Link>
          </p>
        ) : null}
        {cursor ? (
          <p className="mt-4 mb-0">
            <Link
              href={`/collections/${team.slug}`}
              className={`inline-flex h-11 items-center text-fg-secondary ${LINK}`}
            >
              Back to the first collections
            </Link>
          </p>
        ) : null}
      </main>
    </>
  );
}
