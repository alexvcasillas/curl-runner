import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { Breadcrumb } from '@/components/Breadcrumb';
import { buttonClasses } from '@/components/button-styles';
import { CompareSelect } from '@/components/CompareSelect';
import { EmptyState } from '@/components/EmptyState';
import { PublishedBy } from '@/components/PublishedBy';
import { VersionDiff } from '@/components/VersionDiff';
import { getFiles, getVersions, requireCollection, requireTeam } from '@/lib/collections-data';
import { diffVersions } from '@/lib/diff';
import { routes } from '@/lib/routes';

type Props = {
  params: Promise<{ team: string; slug: string; n: string }>;
  searchParams: Promise<{ compare?: string }>;
};

export const metadata: Metadata = { title: 'Changes' };

const count = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'}`;

export default async function VersionPage({ params, searchParams }: Props) {
  const [{ team, slug, n: raw }, { compare }] = await Promise.all([params, searchParams]);
  if (!/^[1-9]\d{0,8}$/.test(raw)) {
    notFound();
  }
  const n = Number(raw);
  await requireTeam(team);
  const collection = await requireCollection(team, slug);
  const versions = await getVersions(team, slug);
  const version = versions.find((v) => v.number === n);
  if (!version) {
    notFound();
  }

  // Older versions, newest first. Against the one before unless asked otherwise.
  const older = versions.map((v) => v.number).filter((m) => m < n);
  const asked = Number(compare);
  const base = older.includes(asked) ? asked : (older[0] ?? null);

  const [next, prev] = await Promise.all([
    getFiles(team, slug, n),
    base === null ? [] : getFiles(team, slug, base),
  ]);
  const diffs = diffVersions(prev, next);
  const added = diffs.filter((d) => d.status === 'added').length;
  const changed = diffs.filter((d) => d.status === 'changed').length;
  const removed = diffs.filter((d) => d.status === 'removed').length;
  const parts = [
    added ? `${added} added` : null,
    changed ? `${changed} changed` : null,
    removed ? `${removed} removed` : null,
  ].filter(Boolean);
  const custom = base !== null && base !== older[0];

  return (
    <>
      <Breadcrumb
        items={[
          { label: 'Collections', href: routes.team(team) },
          { label: collection.name, href: routes.collection(team, slug) },
          { label: `v${n}` },
        ]}
      />

      <div className="flex flex-wrap items-end gap-4">
        <div className="mr-auto flex min-w-0 flex-col gap-1.5">
          <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">
            What changed in v{n}
          </h1>
          <p className="m-0 text-fg-muted">
            <PublishedBy version={version} lead="Published" />.
            {diffs.length > 0
              ? ` ${count(diffs.length, 'file')} changed${custom ? ` against v${base}` : ''}: ${parts.join(', ')}.`
              : ''}
          </p>
        </div>
        {base !== null && older.length > 1 ? (
          <CompareSelect team={team} slug={slug} n={n} options={older} value={base} />
        ) : null}
        <a href={routes.download(team, slug, n)} className={buttonClasses('secondary')}>
          Download v{n}
        </a>
      </div>

      {diffs.length === 0 ? (
        <EmptyState title="No file changes">
          {base === null ? `v${n} has no files.` : `v${n} has the same files as v${base}.`}
        </EmptyState>
      ) : (
        <VersionDiff diffs={diffs} />
      )}
    </>
  );
}
