import type { Metadata } from 'next';
import { AccessBadge, SourceBadge } from '@/components/Badge';
import { Breadcrumb } from '@/components/Breadcrumb';
import { ButtonLink } from '@/components/ButtonLink';
import { buttonClasses } from '@/components/button-styles';
import { Card, CardBody, CardTitle } from '@/components/Card';
import { type CollectionTab, CollectionTabs } from '@/components/CollectionTabs';
import { CopyLinkButton } from '@/components/CopyLinkButton';
import { DeleteCollection } from '@/components/DeleteCollection';
import { EmptyState } from '@/components/EmptyState';
import { InputsTable } from '@/components/InputsTable';
import { ClockIcon } from '@/components/icons';
import { PublishedBy } from '@/components/PublishedBy';
import { SourceSwitch } from '@/components/SourceSwitch';
import { type VersionRow, VersionsTable } from '@/components/VersionsTable';
import { commitUrl, shortSha } from '@/lib/collections';
import {
  getCollections,
  getShare,
  getVersion,
  getVersions,
  requireCollection,
  requireTeam,
} from '@/lib/collections-data';
import { ApiError } from '@/lib/errors';
import { routes } from '@/lib/routes';
import { absoluteTime, relativeTime } from '@/lib/time';
import { deleteCollection, switchSource } from './actions';

type Props = {
  params: Promise<{ team: string; slug: string }>;
  searchParams: Promise<{ tab?: string }>;
};

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { team, slug } = await params;
  try {
    const found = (await getCollections(team)).find((c) => c.slug === slug);
    return { title: found?.name ?? 'Collection' };
  } catch (e) {
    // The page itself shows the error. A title is not worth failing over.
    if (e instanceof ApiError) {
      return { title: 'Collection' };
    }
    throw e;
  }
}

function tabOf(value: string | undefined): Exclude<CollectionTab, 'sharing'> {
  return value === 'inputs' || value === 'settings' ? value : 'versions';
}

export default async function CollectionPage({ params, searchParams }: Props) {
  const [{ team, slug }, { tab: tabParam }] = await Promise.all([params, searchParams]);
  const tab = tabOf(tabParam);
  const current = await requireTeam(team);
  const collection = await requireCollection(team, slug);
  const now = new Date();

  const latest = collection.current_version;
  const [versions, share, detail] = await Promise.all([
    getVersions(team, slug),
    getShare(team, slug),
    tab === 'inputs' && latest !== null ? getVersion(team, slug, latest) : null,
  ]);
  const newest = versions[0];

  const rows: VersionRow[] = versions.map((v) => ({
    number: v.number,
    current: v.number === latest,
    when: relativeTime(v.published_at, now),
    abs: absoluteTime(v.published_at),
    by: v.publisher,
    sha: v.git?.commit ? shortSha(v.git.commit) : null,
    shaHref: commitUrl(v.git),
    ref: v.git?.ref ?? null,
    files: v.file_count,
    changesHref: routes.version(team, slug, v.number),
    downloadHref: routes.download(team, slug, v.number),
  }));

  return (
    <>
      <Breadcrumb
        items={[{ label: 'Collections', href: routes.team(team) }, { label: collection.name }]}
      />

      <div className="flex flex-wrap items-start gap-4">
        <div className="mr-auto flex min-w-0 flex-col gap-2">
          <div className="flex flex-wrap items-center gap-2.5">
            <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em] [overflow-wrap:anywhere]">
              {collection.name}
            </h1>
            <SourceBadge source={collection.source} />
            <AccessBadge access={collection.access} />
          </div>
          <p className="m-0 text-[12px] text-fg-subtle">
            {newest ? (
              <PublishedBy
                version={newest}
                lead={<span className="font-mono text-fg">v{newest.number}</span>}
              />
            ) : (
              'Nothing published yet.'
            )}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          {share ? <CopyLinkButton link={share.link} disabled={share.access === 'off'} /> : null}
          <ButtonLink href={routes.sharing(team, slug)} variant="primary">
            Share
          </ButtonLink>
        </div>
      </div>

      <CollectionTabs team={team} slug={slug} current={tab} />

      {tab === 'versions' ? (
        rows.length === 0 ? (
          <EmptyState icon={<ClockIcon size={18} />} title="No versions yet">
            The first sync from the studio, or the first CI publish, creates version 1.
          </EmptyState>
        ) : (
          <VersionsTable
            rows={rows}
            note={
              collection.source === 'git'
                ? 'To roll back, re-run CI on an older commit.'
                : undefined
            }
          />
        )
      ) : null}

      {tab === 'inputs' ? (
        detail ? (
          <InputsTable name={collection.name} inputs={detail.inputs} />
        ) : (
          <EmptyState icon={<ClockIcon size={18} />} title="No inputs yet">
            Inputs come from a published version. Publish one and they appear here.
          </EmptyState>
        )
      ) : null}

      {tab === 'settings' ? (
        <div className="flex flex-col gap-5">
          <SourceSwitch
            name={collection.name}
            source={collection.source}
            version={latest}
            downloadHref={latest === null ? '' : routes.download(team, slug, latest)}
            onSwitch={switchSource.bind(null, team, slug)}
          />
          {latest !== null ? (
            <Card aria-labelledby="download-title">
              <CardBody className="flex-row flex-wrap items-center">
                <div className="flex min-w-[240px] flex-1 flex-col gap-1">
                  <CardTitle id="download-title" className="text-[14px] leading-5">
                    Download the current version
                  </CardTitle>
                  <p className="m-0 text-fg-muted leading-normal">
                    v{latest} as a .zip of its YAML files, ready to commit or run with the CLI.
                  </p>
                </div>
                <a
                  href={routes.download(team, slug, latest)}
                  className={buttonClasses('secondary')}
                >
                  Download v{latest}
                </a>
              </CardBody>
            </Card>
          ) : null}
          <DeleteCollection
            name={collection.name}
            slug={slug}
            versions={versions.length}
            people={share?.entries.length ?? 0}
            source={collection.source}
            owner={current.role === 'owner'}
            onDelete={deleteCollection.bind(null, team, slug)}
          />
        </div>
      ) : null}
    </>
  );
}
