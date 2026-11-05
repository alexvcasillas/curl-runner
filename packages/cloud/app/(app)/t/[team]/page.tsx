import type { Metadata } from 'next';
import { ButtonLink } from '@/components/ButtonLink';
import { CollectionsEmpty } from '@/components/CollectionsEmpty';
import { type CollectionRow, CollectionsView } from '@/components/CollectionsView';
import { PageHeader } from '@/components/PageHeader';
import { getInfo } from '@/lib/app-data';
import { planNote } from '@/lib/collections';
import { getCollections, requireTeam } from '@/lib/collections-data';
import { external, routes } from '@/lib/routes';
import { absoluteTime, relativeTime } from '@/lib/time';

export const metadata: Metadata = { title: 'Collections' };

export default async function TeamPage({ params }: { params: Promise<{ team: string }> }) {
  const { team } = await params;
  const [current, info] = await Promise.all([requireTeam(team), getInfo()]);
  const collections = await getCollections(team);
  const now = new Date();

  const rows: CollectionRow[] = collections.map((c) => ({
    slug: c.slug,
    name: c.name,
    source: c.source,
    access: c.access,
    version: c.current_version === null ? null : `v${c.current_version}`,
    when: c.updated_at ? relativeTime(c.updated_at, now) : null,
    abs: c.updated_at ? absoluteTime(c.updated_at) : null,
    by: c.publisher,
    href: routes.collection(team, c.slug),
  }));

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
