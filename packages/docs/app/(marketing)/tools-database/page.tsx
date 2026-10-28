import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { ButtonLink } from '@/components/site/ButtonLink';
import { Eyebrow } from '@/components/site/Eyebrow';
import { SiteHeader } from '@/components/site/SiteHeader';
import { getLatestRelease } from '@/lib/github-release';

export const revalidate = 3600;

export const metadata: Metadata = {
  title: 'curl-runner fact sheet',
  description:
    'curl-runner at a glance: what it is, how it is built, where it runs and how to install it. Every fact is checkable in the repository.',
  alternates: { canonical: '/tools-database' },
};

const LINK =
  'underline underline-offset-2 hover:text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand';

export default async function ToolsDatabasePage() {
  const release = await getLatestRelease();

  const facts: { k: string; v: ReactNode }[] = [
    { k: 'Category', v: 'API testing · HTTP client · CLI' },
    {
      k: 'Latest version',
      v: release ? (
        <>{release.version} (live from GitHub Releases)</>
      ) : (
        <>
          {'see '}
          <a href="/downloads" className={LINK}>
            Downloads
          </a>
        </>
      ),
    },
    { k: 'Written in', v: 'Rust, with libcurl statically linked' },
    { k: 'Runtime needed', v: 'None — a single native binary' },
    {
      k: 'Platforms',
      v: 'macOS (arm64, x64), Linux (x64, arm64, static musl), Windows (x64)',
    },
    {
      k: 'Install',
      v: 'install.sh, install.ps1, Homebrew, npm / bun / pnpm, cargo, direct download',
    },
    {
      k: 'Test format',
      v: 'YAML files: requests, variables, expectations, retries, stored values',
    },
    {
      k: 'Protocols',
      v: 'HTTP/1.1, HTTP/2, TLS via the operating system trust store',
    },
    {
      k: 'CI use',
      v: 'Exit codes for pass and fail, failure thresholds, JSON / raw / pretty output',
    },
    { k: 'License', v: 'MIT' },
    {
      k: 'Source',
      v: (
        <a href="https://github.com/alexvcasillas/curl-runner" className={LINK}>
          github.com/alexvcasillas/curl-runner
        </a>
      ),
    },
    {
      k: 'Package names',
      v: '@curl-runner/cli (npm), curl-runner (crates.io), alexvcasillas/tap/curl-runner (Homebrew)',
    },
  ];

  return (
    <>
      <SiteHeader />
      <div className="mx-auto max-w-[880px] px-6 py-24">
        <Eyebrow>FACT SHEET</Eyebrow>
        <h1 className="mt-4 mb-0 text-[clamp(36px,5vw,56px)] leading-[1.04] font-bold tracking-[-0.04em]">
          curl-runner at a glance
        </h1>
        <p className="mt-5 mb-0 text-[18px] leading-[1.6] text-fg-secondary">
          A command-line tool for running HTTP request suites defined in YAML: validation, retries,
          parallel execution, snapshots and diffs. Everything on this page is checkable in the
          repository.
        </p>
        <dl className="mt-14 mb-0 border-t border-hairline">
          {facts.map((f) => (
            <div
              key={f.k}
              className="flex flex-wrap gap-x-8 gap-y-2 border-b border-hairline py-[18px]"
            >
              <dt className="basis-[200px] text-sm text-fg-tertiary">{f.k}</dt>
              <dd className="m-0 flex-1 basis-[360px] text-base leading-[1.6]">{f.v}</dd>
            </div>
          ))}
        </dl>
        <div className="mt-12 flex flex-wrap gap-3">
          <ButtonLink href="/downloads" variant="primary" size="lg">
            Install
          </ButtonLink>
          <ButtonLink href="/alternatives" variant="secondary" size="lg">
            Compare with alternatives
          </ButtonLink>
        </div>
      </div>
    </>
  );
}
