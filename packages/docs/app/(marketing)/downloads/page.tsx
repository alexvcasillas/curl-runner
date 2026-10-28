import { Download } from 'lucide-react';
import type { Metadata } from 'next';
import { Eyebrow } from '@/components/site/Eyebrow';
import { InstallTabs } from '@/components/site/InstallTabs';
import { SiteHeader } from '@/components/site/SiteHeader';
import { formatFileSize, getLatestRelease, getPlatformLabel } from '@/lib/github-release';
import { pageMetadata } from '@/lib/site';

export const revalidate = 3600;

const RELEASES_URL = 'https://github.com/alexvcasillas/curl-runner/releases';

export const metadata: Metadata = pageMetadata({
  title: 'Download',
  description:
    'Install curl-runner with a one-line script, Homebrew, npm or cargo, or download a pre-built static binary for macOS, Linux and Windows.',
  path: '/downloads',
  ownImage: true,
});

function formatDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return '';
  }
  return date
    .toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      timeZone: 'UTC',
    })
    .toUpperCase();
}

const LINK =
  'underline underline-offset-2 hover:text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand';

export default async function DownloadsPage() {
  const release = await getLatestRelease();
  const date = release ? formatDate(release.publishedAt) : '';
  const eyebrow = release
    ? ['DOWNLOAD', `v${release.version}`, date].filter(Boolean).join(' · ')
    : 'DOWNLOAD';
  const binaries = release?.binaries ?? [];
  const showTable = release !== null && binaries.length > 0;

  return (
    <>
      <SiteHeader active="downloads" />
      <main id="main-content">
        <section className="mx-auto max-w-[1200px] px-6 pt-24 pb-12">
          <Eyebrow>{eyebrow}</Eyebrow>
          <h1 className="mt-4 mb-0 text-[clamp(40px,6vw,72px)] leading-[1.02] font-bold tracking-[-0.045em]">
            One binary. Every platform.
          </h1>
          <p className="mt-6 mb-0 max-w-160 text-[19px] leading-[1.6] text-fg-secondary">
            A single static executable with libcurl built in. Nothing else to install — no Node, no
            Bun, no curl.
          </p>
        </section>

        <section aria-label="Install" className="mx-auto max-w-[1200px] px-6 pb-18">
          <InstallTabs
            methods={['curl', 'powershell', 'brew', 'npm', 'cargo']}
            defaultId="curl"
            showNote
          />
        </section>

        <section aria-labelledby="binaries-heading" className="mx-auto max-w-[1200px] px-6 pb-24">
          <div className="mb-5 flex flex-wrap items-baseline justify-between gap-4">
            <h2
              id="binaries-heading"
              className="m-0 text-[28px] font-bold tracking-[-0.025em] text-fg"
            >
              Pre-built binaries
            </h2>
            {release?.checksumUrl ? (
              <a href={release.checksumUrl} className={`text-sm text-fg-secondary ${LINK}`}>
                SHA256SUMS.txt
              </a>
            ) : null}
          </div>

          {showTable ? (
            <>
              <ul className="m-0 list-none overflow-hidden rounded-[14px] border border-line p-0">
                {binaries.map((b) => {
                  const label = getPlatformLabel(b.os, b.arch);
                  return (
                    <li
                      key={b.platform}
                      className="flex flex-wrap items-center gap-x-5 gap-y-2 border-b border-hairline px-6 py-5 last:border-b-0"
                    >
                      <span className="min-w-50 flex-[1_1_200px] text-base font-semibold">
                        {label}
                      </span>
                      <span className="min-w-0 flex-[1_1_260px] font-mono text-[13px] break-all text-fg-tertiary">
                        {b.fileName}
                      </span>
                      <span className="w-16 text-right font-mono text-[13px] text-fg-secondary">
                        {formatFileSize(b.size)}
                      </span>
                      <a
                        href={b.downloadUrl}
                        aria-label={`Download ${label} (${b.fileName})`}
                        className="relative inline-flex h-9 items-center gap-2 rounded-lg border border-line px-3.5 text-sm transition-colors before:absolute before:inset-x-0 before:-inset-y-1 before:content-[''] hover:bg-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand"
                      >
                        <Download aria-hidden="true" className="size-3.5" />
                        Download
                      </a>
                    </li>
                  );
                })}
              </ul>
              <p className="mt-4 mb-0 text-sm text-fg-tertiary">
                Verify with{' '}
                <code className="font-mono text-[#d4d4d4]">shasum -a 256 -c SHA256SUMS.txt</code>.
                Older versions and release notes on{' '}
                <a href={RELEASES_URL} className={`text-fg ${LINK}`}>
                  GitHub Releases
                </a>
                .
              </p>
            </>
          ) : (
            <div className="rounded-[14px] border border-line p-6">
              <p className="m-0 text-fg-secondary">
                The release list is unavailable right now. Binaries for every platform are on
                GitHub.
              </p>
              <p className="mt-4 mb-0">
                <a href={RELEASES_URL} className={`text-fg ${LINK}`}>
                  Download from GitHub Releases
                </a>
              </p>
            </div>
          )}
        </section>
      </main>
    </>
  );
}
