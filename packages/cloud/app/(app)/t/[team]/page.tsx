import type { Metadata } from 'next';
import { ButtonLink } from '@/components/ButtonLink';
import { CollectionsEmpty } from '@/components/CollectionsEmpty';
import { type CollectionRow, CollectionsView } from '@/components/CollectionsView';
import { PageHeader } from '@/components/PageHeader';
import { getInfo } from '@/lib/app-data';
import { planNote } from '@/lib/collections';
import { getCollections, getVersion, requireTeam } from '@/lib/collections-data';
import { ApiError } from '@/lib/errors';
import { external, routes } from '@/lib/routes';
import { absoluteTime, relativeTime } from '@/lib/time';
import type { CollectionSummary } from '@/lib/types';

export const metadata: Metadata = { title: 'Collections' };

/** Who published the current version. A row still shows if this can't be read. */
async function publisherOf(team: string, c: CollectionSummary): Promise<string | null> {
  if (c.current_version === null) {
    return null;
  }
  try {
    return (await getVersion(team, c.slug, c.current_version)).publisher;
  } catch (e) {
    if (e instanceof ApiError) {
      return null;
    }
    throw e;
  }
}

export default async function TeamPage({ params }: { params: Promise<{ team: string }> }) {
  const { team } = await params;
  const [current, info] = await Promise.all([requireTeam(team), getInfo()]);
  const collections = await getCollections(team);
  const now = new Date();

  const rows: CollectionRow[] = await Promise.all(
    collections.map(async (c) => ({
      slug: c.slug,
      name: c.name,
      source: c.source,
      access: c.access,
      version: c.current_version === null ? null : `v${c.current_version}`,
      when: c.updated_at ? relativeTime(c.updated_at, now) : null,
      abs: c.updated_at ? absoluteTime(c.updated_at) : null,
      by: await publisherOf(team, c),
      href: routes.collection(team, c.slug),
    })),
  );

  return (
    <>
      <PageHeader
        title="Collections"
        actions={
          <ButtonLink href={external.cloudDocs} external>
            Publishing guide
          </ButtonLink>
        }
      >
        Everything CI or the studio publishes to {current.name}. Each publish makes a new version.
      </PageHeader>
      {rows.length === 0 ? (
        <CollectionsEmpty team={team} />
      ) : (
        <CollectionsView rows={rows} note={planNote(current.plan, info.billing)} />
      )}
    </>
  );
}
