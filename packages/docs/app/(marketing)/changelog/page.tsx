import type { Metadata } from 'next';
import { Eyebrow } from '@/components/site/Eyebrow';
import { SiteHeader } from '@/components/site/SiteHeader';
import { type Bump, getReleaseDates, getReleases, REPO_URL } from '@/lib/changelog';
import { renderEntry } from '@/lib/changelog-markdown';
import { pageMetadata } from '@/lib/site';

// Prerendered from packages/cli/CHANGELOG.md; release dates refresh hourly (ISR). The
// changelog is traced into the function by next.config.ts for those regenerations.
export const dynamic = 'force-static';

const SHOWN_RELEASES = 10;
const RELEASES_URL = `${REPO_URL}/releases`;

const changelogMeta = pageMetadata({
  title: 'Changelog',
  description:
    'Release notes for every curl-runner version, generated from the CLI changelog: new features, fixes and migration notes.',
  path: '/changelog',
});

export const metadata: Metadata = {
  ...changelogMeta,
  alternates: { ...changelogMeta.alternates, types: { 'application/rss+xml': '/rss.xml' } },
};

const BUMP_LABEL: Record<Bump, string> = { major: 'Major', minor: 'Minor', patch: 'Patch' };

const LINK =
  'text-fg-secondary underline-offset-2 hover:text-fg hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand';

const FOCUS = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand';

function formatDate(iso: string): string | null {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return null;
  }
  return date.toLocaleDateString('en-US', {
    month: 'long',
    day: 'numeric',
    year: 'numeric',
    timeZone: 'UTC',
  });
}

function BumpBadge({ bump }: { bump: Bump }) {
  const style =
    bump === 'major' ? 'bg-brand font-semibold text-ground' : 'border border-line text-[#d4d4d4]';
  return (
    <span
      className={`inline-flex h-6 items-center self-start rounded-full px-2.5 text-xs ${style}`}
    >
      {BUMP_LABEL[bump]}
    </span>
  );
}

function Notes({ html }: { html: string }) {
  return (
    <div
      className="release-notes"
      // biome-ignore lint/security/noDangerouslySetInnerHtml: Markdown from the repo's own CHANGELOG.md, rendered at build time with raw HTML escaped
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}

export default async function ChangelogPage() {
  const releases = getReleases().slice(0, SHOWN_RELEASES);
  const dates = await getReleaseDates();
  const rendered = await Promise.all(
    releases.map(async (release) => ({
      ...release,
      date: dates[release.version] ? formatDate(dates[release.version]) : null,
      iso: dates[release.version],
      entries: await Promise.all(
        release.entries.map(async (entry) => ({ entry, html: await renderEntry(entry) })),
      ),
    })),
  );

  return (
    <>
      <SiteHeader active="changelog" />
      <main id="main-content">
        <section className="mx-auto flex max-w-[1200px] flex-wrap items-end justify-between gap-6 px-6 pt-24 pb-14">
          <div>
            <Eyebrow>Changelog</Eyebrow>
            <h1 className="mt-4 mb-0 text-[clamp(40px,6vw,72px)] leading-[1.02] font-bold tracking-[-0.045em] text-fg">
              What’s new in curl-runner.
            </h1>
            <p className="mt-5 mb-0 max-w-[38rem] text-lg leading-[1.6] text-fg-secondary">
              Every release, straight from the project’s changelog. Upgrade any install with{' '}
              <code className="font-mono text-fg">curl-runner upgrade</code>.
            </p>
          </div>
          <a
            href="/rss.xml"
            className={`inline-flex h-11 items-center gap-2 rounded-[10px] border border-line px-4 text-sm text-fg transition-colors hover:bg-hover ${FOCUS}`}
          >
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
              className="size-4"
            >
              <path d="M4 11a9 9 0 0 1 9 9" />
              <path d="M4 4a16 16 0 0 1 16 16" />
              <circle cx="5" cy="19" r="1" />
            </svg>
            RSS
            <span className="sr-only"> feed of releases</span>
          </a>
        </section>

        <section aria-label="Releases" className="mx-auto max-w-[1200px] px-6 pb-24">
          {rendered.map((release) => (
            <article
              key={release.version}
              id={release.version}
              aria-labelledby={`v-${release.version}`}
              className="flex scroll-mt-6 flex-wrap gap-x-12 gap-y-4 border-t border-hairline py-12 last-of-type:border-b"
            >
              <div className="flex flex-[0_0_220px] flex-col gap-2.5">
                <h2 id={`v-${release.version}`} className="m-0 font-mono text-[22px] font-medium">
                  <a href={`#${release.version}`} className={`text-fg hover:text-white ${FOCUS}`}>
                    {release.version}
                  </a>
                </h2>
                {release.date && release.iso ? (
                  <time dateTime={release.iso} className="text-sm text-fg-tertiary">
                    {release.date}
                  </time>
                ) : null}
                <BumpBadge bump={release.bump} />
              </div>

              <div className="flex min-w-0 flex-[999_1_560px] flex-col gap-10">
                {release.entries.map(({ entry, html }, index) => (
                  <div
                    key={`${entry.commit ?? ''}-${index}`}
                    className="flex min-w-0 flex-col gap-4"
                  >
                    {html.headlineHtml ? (
                      <h3
                        className="m-0 text-[26px] leading-tight font-bold tracking-[-0.02em] text-fg [&_code]:font-mono"
                        // biome-ignore lint/security/noDangerouslySetInnerHtml: inline Markdown from the repo's CHANGELOG.md, raw HTML escaped
                        dangerouslySetInnerHTML={{ __html: html.headlineHtml }}
                      />
                    ) : null}
                    <Notes html={html.previewHtml} />
                    {html.restHtml ? (
                      <details className="group">
                        <summary
                          className={`inline-flex h-10 cursor-pointer list-none items-center gap-2 self-start rounded-[10px] border border-line px-4 text-sm text-fg transition-colors hover:bg-hover [&::-webkit-details-marker]:hidden ${FOCUS}`}
                        >
                          <span className="group-open:hidden">{`Show all ${release.version} changes`}</span>
                          <span className="hidden group-open:inline">Show fewer changes</span>
                        </summary>
                        <div className="mt-4">
                          <Notes html={html.restHtml} />
                        </div>
                      </details>
                    ) : null}
                    {entry.pr !== null || entry.commit !== null ? (
                      <p className="m-0 font-mono text-xs text-fg-tertiary">
                        {entry.pr !== null ? (
                          <a
                            href={`${REPO_URL}/pull/${entry.pr}`}
                            aria-label={`Pull request #${entry.pr}`}
                            className={LINK}
                          >
                            #{entry.pr}
                          </a>
                        ) : null}
                        {entry.pr !== null && entry.commit !== null ? ' · ' : null}
                        {entry.commit !== null ? (
                          <a
                            href={`${REPO_URL}/commit/${entry.commit}`}
                            aria-label={`Commit ${entry.commit}`}
                            className={LINK}
                          >
                            {entry.commit}
                          </a>
                        ) : null}
                      </p>
                    ) : null}
                  </div>
                ))}
              </div>
            </article>
          ))}

          <div className="flex justify-center pt-10">
            <a
              href={RELEASES_URL}
              className={`inline-flex h-11 items-center rounded-[10px] border border-line px-5 text-[15px] text-fg transition-colors hover:bg-hover ${FOCUS}`}
            >
              Older releases
            </a>
          </div>
        </section>
      </main>
    </>
  );
}
