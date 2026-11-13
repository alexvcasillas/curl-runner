import type { Metadata } from 'next';
import { Breadcrumb } from '@/components/Breadcrumb';
import { ButtonLink } from '@/components/ButtonLink';
import { CollectionTabs } from '@/components/CollectionTabs';
import { EmptyState } from '@/components/EmptyState';
import { ShareAside } from '@/components/ShareAside';
import { SharePanel } from '@/components/SharePanel';
import { getInfo } from '@/lib/app-data';
import { getCollections, getShare, requireCollection, requireTeam } from '@/lib/collections-data';
import { ApiError } from '@/lib/errors';
import { routes } from '@/lib/routes';
import { addEntry, patchShare, removeEntry } from '../share-actions';

type Props = { params: Promise<{ team: string; slug: string }> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { team, slug } = await params;
  try {
    const found = (await getCollections(team)).find((c) => c.slug === slug);
    return { title: found ? `Sharing · ${found.name}` : 'Sharing' };
  } catch (e) {
    if (e instanceof ApiError) {
      return { title: 'Sharing' };
    }
    throw e;
  }
}

export default async function SharingPage({ params }: Props) {
  const { team, slug } = await params;
  const current = await requireTeam(team);
  const collection = await requireCollection(team, slug);
  const [share, info] = await Promise.all([getShare(team, slug), getInfo()]);

  return (
    <>
      <Breadcrumb
        items={[{ label: 'Collections', href: routes.team(team) }, { label: collection.name }]}
      />
      <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em] [overflow-wrap:anywhere]">
        {collection.name}
      </h1>
      <CollectionTabs team={team} slug={slug} current="sharing" />

      {share ? (
        <div className="flex flex-wrap items-start gap-5">
          <div className="min-w-0 flex-[999_1_560px]">
            <SharePanel
              team={team}
              slug={slug}
              teamName={current.name}
              name={collection.name}
              plan={current.plan}
              billing={info.billing}
              share={share}
              now={new Date().toISOString()}
              onPatch={patchShare.bind(null, team, slug)}
              onAdd={addEntry.bind(null, team, slug)}
              onRemove={removeEntry.bind(null, team, slug)}
            />
          </div>
          <ShareAside teamName={current.name} link={share.link} off={share.access === 'off'} />
        </div>
      ) : (
        <EmptyState
          title="Sharing isn’t available right now"
          actions={<ButtonLink href={routes.sharing(team, slug)}>Reload</ButtonLink>}
        >
          Cloud couldn’t load the share settings. Reload the page in a minute.
        </EmptyState>
      )}
    </>
  );
}
