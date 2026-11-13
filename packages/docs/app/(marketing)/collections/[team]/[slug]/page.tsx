import type { Metadata } from 'next';
import Link from 'next/link';
import { notFound } from 'next/navigation';
import { CopyCommand } from '@/components/gallery/CopyCommand';
import { GalleryDown, LINK, metaLine, TeamTile } from '@/components/gallery/parts';
import { SiteHeader } from '@/components/site/SiteHeader';
import { Yaml } from '@/components/site/Yaml';
import {
  collectionHref,
  detailMetadata,
  fetchCollection,
  folderCount,
  groupRequests,
  methodColor,
  plural,
  pullCommand,
  type RequestGroup,
  requestHref,
  safeShareLink,
  selectRequest,
  shareToken,
  studioLink,
} from '@/lib/gallery';

export const revalidate = 300;

interface Props {
  params: Promise<{ team: string; slug: string }>;
  searchParams: Promise<{ file?: string | string[]; req?: string | string[] }>;
}

function first(value: string | string[] | undefined): string | undefined {
  return Array.isArray(value) ? value[0] : value;
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { team, slug } = await params;
  const res = await fetchCollection(team, slug);
  return res.status === 'ok' ? detailMetadata(res.data) : {};
}

function Method({ method }: { method: string }) {
  return (
    <span
      className="w-14 shrink-0 font-mono text-[11.5px] font-semibold"
      style={{ color: methodColor(method) }}
    >
      {method.toUpperCase()}
    </span>
  );
}

function RequestList({
  groups,
  base,
  current,
}: {
  groups: RequestGroup[];
  base: string;
  current: { file: string; index: number } | null;
}) {
  return (
    <div className="flex flex-col gap-4">
      {groups.map((group) => (
        <div key={group.folder ?? '(root)'}>
          {group.folder ? (
            <p className="m-0 mb-1 px-3 font-mono text-[11.5px] tracking-[0.06em] text-fg-tertiary uppercase">
              {group.folder}
            </p>
          ) : null}
          <ul className="m-0 flex list-none flex-col p-0">
            {group.rows.map((row) => {
              const active = current?.file === row.file && current.index === row.index;
              return (
                <li key={`${row.file}#${row.index}`}>
                  <Link
                    href={requestHref(base, row)}
                    aria-current={active ? 'true' : undefined}
                    className={`flex min-h-11 items-center gap-2 rounded-lg px-3 py-2 text-sm transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-brand ${
                      active ? 'bg-selected text-fg' : 'text-fg-secondary'
                    }`}
                  >
                    <Method method={row.method} />
                    <span className="min-w-0 break-words">{row.name}</span>
                  </Link>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </div>
  );
}

export default async function CollectionPage({ params, searchParams }: Props) {
  const { team: teamSlug, slug } = await params;
  const query = await searchParams;
  const res = await fetchCollection(teamSlug, slug);

  if (res.status === 'not_found') {
    notFound();
  }
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

  const c = res.data;
  const base = collectionHref(c.team.slug, c.slug);
  const groups = groupRequests(c.files);
  const requestCount = groups.reduce((n, g) => n + g.rows.length, 0);
  const { row, file } = selectRequest(c.files, groups, first(query.file), first(query.req));
  const current = row ? { file: row.file, index: row.index } : null;
  const link = safeShareLink(c.link);
  const folders = folderCount(groups);
  const meta = metaLine({ version: c.version, updated_at: c.updated_at });
  const summary = `${plural(requestCount, 'request')}${folders > 0 ? ` in ${plural(folders, 'folder')}` : ''}`;

  return (
    <>
      <SiteHeader active="collections" />
      <main id="main-content" className="mx-auto max-w-[1200px] px-6 pt-10 pb-24">
        <div className="grid grid-cols-[minmax(0,1fr)_360px] gap-x-12 gap-y-8 max-lg:grid-cols-1">
          <div className="min-w-0">
            <nav
              aria-label="Breadcrumb"
              className="flex flex-wrap items-center text-sm text-fg-secondary"
            >
              <Link href="/collections" className="inline-flex h-11 items-center hover:text-fg">
                Collections
              </Link>
              <span aria-hidden="true" className="mx-2 text-fg-tertiary">
                /
              </span>
              <Link
                href={`/collections/${c.team.slug}`}
                className="inline-flex h-11 items-center hover:text-fg"
              >
                {c.team.name}
              </Link>
              <span aria-hidden="true" className="mx-2 text-fg-tertiary">
                /
              </span>
              <span aria-current="page" className="text-fg">
                {c.name}
              </span>
            </nav>
            <Link
              href={`/collections/${c.team.slug}`}
              className="mt-3 inline-flex min-h-11 items-center gap-2 text-sm text-fg-secondary hover:text-fg"
            >
              <TeamTile name={c.team.name} />
              {c.team.name}
            </Link>
            <h1 className="mt-2 mb-0 text-[clamp(34px,5vw,56px)] leading-[1.05] font-bold tracking-[-0.04em] break-words">
              {c.name}
            </h1>
            {c.description ? (
              <p className="mt-5 mb-0 max-w-160 text-[17px] leading-[1.6] break-words text-fg-secondary">
                {c.description}
              </p>
            ) : null}
            <p className="mt-5 mb-0 font-mono text-[13px] text-fg-tertiary">
              {[summary, meta].filter(Boolean).join(' · ')}
            </p>
          </div>

          <aside
            aria-label="Open this collection"
            className="flex min-w-0 flex-col gap-4 self-start rounded-[14px] border border-line p-6 lg:mt-11"
          >
            {link ? (
              <>
                <a
                  href={studioLink(c.team.slug, c.slug, shareToken(link))}
                  className="inline-flex h-11 w-full items-center justify-center rounded-[10px] bg-fg px-5 text-[15px] font-medium text-ground transition-colors hover:bg-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand"
                >
                  Open in curl-runner studio
                </a>
                <p className="m-0 text-[13px] text-fg-tertiary">
                  macOS. No studio yet?{' '}
                  <Link href="/downloads" className={`text-fg-secondary ${LINK}`}>
                    Download it
                  </Link>
                  .
                </p>
                <hr className="m-0 border-0 border-t border-hairline" />
                <p className="m-0 text-sm text-fg-secondary">Or with the CLI, on any OS:</p>
                <CopyCommand command={pullCommand(link)} />
              </>
            ) : null}
            <p className="m-0 text-[13px] leading-[1.5] text-fg-tertiary">
              You fill in your own values. Nothing you enter is sent back to {c.team.name}.
            </p>
          </aside>
        </div>

        {c.inputs.length > 0 ? (
          <section aria-labelledby="inputs-heading" className="mt-14">
            <h2
              id="inputs-heading"
              className="m-0 mb-4 text-[22px] font-bold tracking-[-0.02em] text-fg"
            >
              You’ll fill in
            </h2>
            <ul className="m-0 flex list-none flex-wrap gap-3 p-0">
              {c.inputs.map((input) => (
                <li
                  key={input.name}
                  className="min-w-0 max-w-full rounded-[10px] border border-line px-4 py-3"
                >
                  <code className="font-mono text-[13px] break-all text-fg">{input.name}</code>
                  {input.description ? (
                    <span className="mt-1 block text-[13px] text-fg-tertiary">
                      {input.description}
                    </span>
                  ) : null}
                  {input.secret ? (
                    <span className="mt-1 block text-[12px] text-fg-tertiary">
                      Secret · stays on your machine
                    </span>
                  ) : null}
                </li>
              ))}
            </ul>
          </section>
        ) : null}

        <section aria-labelledby="requests-heading" className="mt-14">
          <h2
            id="requests-heading"
            className="m-0 mb-4 text-[22px] font-bold tracking-[-0.02em] text-fg"
          >
            Requests
          </h2>
          {row && file ? (
            <div className="grid grid-cols-[280px_minmax(0,1fr)] overflow-hidden rounded-[14px] border border-line max-md:grid-cols-1">
              <nav
                aria-label="Requests"
                className="border-r border-line p-3 max-md:hidden md:max-h-[640px] md:overflow-y-auto"
              >
                <RequestList groups={groups} base={base} current={current} />
              </nav>
              <details className="group border-b border-line md:hidden">
                <summary className="flex min-h-11 cursor-pointer list-none items-center gap-2 px-4 py-2 text-sm text-fg marker:hidden [&::-webkit-details-marker]:hidden">
                  <Method method={row.method} />
                  <span className="min-w-0 grow break-words">{row.name}</span>
                  <span aria-hidden="true" className="text-fg-tertiary group-open:rotate-180">
                    ▾
                  </span>
                </summary>
                <div className="max-h-72 overflow-y-auto border-t border-hairline p-2">
                  <RequestList groups={groups} base={base} current={current} />
                </div>
              </details>
              <div className="min-w-0">
                <div className="flex items-center justify-between gap-3 border-b border-line px-4 py-2 min-h-11">
                  <span className="min-w-0 font-mono text-[13px] break-all text-fg-secondary">
                    {file.path}
                  </span>
                  <span className="shrink-0 text-[12px] text-fg-tertiary">Read only</span>
                </div>
                <div className="max-h-[640px] overflow-auto max-md:max-h-[420px]">
                  <Yaml source={file.text} />
                </div>
              </div>
            </div>
          ) : (
            <p className="m-0 rounded-[14px] border border-line p-6 text-fg-secondary">
              This collection has no requests yet.
            </p>
          )}
        </section>

        <p className="mt-8 mb-0 text-sm text-fg-tertiary">
          Published by {c.team.name} with curl-runner. Version {c.version}.
        </p>
      </main>
    </>
  );
}
