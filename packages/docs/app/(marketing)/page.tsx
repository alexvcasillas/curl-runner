import { ArrowRight, FileText, Terminal } from 'lucide-react';
import type { Metadata } from 'next';
import Link from 'next/link';
import { ButtonLink } from '@/components/site/ButtonLink';
import { CodePane } from '@/components/site/CodePane';
import { Eyebrow } from '@/components/site/Eyebrow';
import { InstallTabs } from '@/components/site/InstallTabs';
import { GITHUB_URL, SiteHeader } from '@/components/site/SiteHeader';
import { Yaml } from '@/components/site/Yaml';
import { HOME_FEATURES } from '@/lib/home-features';

const DESCRIPTION =
  'Describe requests in plain YAML, validate every response, and run whole suites in parallel — on your laptop or in CI. One native binary, no runtime.';

export const metadata: Metadata = {
  title: 'curl-runner — API tests as YAML, run anywhere',
  description: DESCRIPTION,
};

const API_YAML = `
global:
  variables:
    BASE: https://api.example.com
  execution: parallel

requests:
  - name: List users
    url: \${BASE}/users
    expect:
      status: 200
      responseTime: "< 500"

  - name: Create user
    url: \${BASE}/users
    method: POST
    body: { name: Ada }
    retry: { count: 3 }
    expect:
      status: 201
`;

const CI_YAML = `
steps:
  - uses: actions/checkout@v7
  - run: curl -fsSL https://www.curl-runner.com/install.sh | bash
  - run: curl-runner tests/ --strict-exit --output results.json
`;

const INSTALL_STRIP = ['brew', 'npm', 'bun', 'pnpm', 'cargo', 'install.sh', 'PowerShell'];

const TILES = [
  { value: '1', label: 'static binary per platform' },
  { value: '0', label: 'runtime dependencies' },
  { value: '< 7 ms', label: 'startup' },
  { value: '13.5 MB', label: 'peak memory, 100 requests' },
];

const H2 = 'm-0 text-[clamp(32px,4.4vw,48px)] leading-[1.08] font-bold tracking-[-0.035em] text-fg';

function PaneHeader({ icon, children }: { icon: 'file' | 'terminal'; children: string }) {
  const Icon = icon === 'file' ? FileText : Terminal;
  return (
    <div className="flex h-11 items-center gap-2 border-b border-hairline px-4 font-mono text-[13px] text-fg-secondary">
      <Icon aria-hidden="true" className="size-3.5" />
      {children}
    </div>
  );
}

export default function Home() {
  return (
    <>
      <SiteHeader />

      <section className="mx-auto flex max-w-[1200px] flex-col items-center gap-7 px-6 pt-24 pb-16 text-center">
        <Link
          href="/changelog#2.0.0"
          className="inline-flex h-8 items-center gap-2 rounded-full border border-line px-3.5 text-[13px] text-fg-secondary transition-colors hover:text-fg"
        >
          <span aria-hidden="true" className="size-1.5 rounded-full bg-brand" />
          curl-runner 2.0 — rewritten in Rust on libcurl
          <ArrowRight aria-hidden="true" className="size-3.5" />
        </Link>
        <h1 className="m-0 max-w-[16ch] text-[clamp(44px,7vw,84px)] leading-[1.02] font-bold tracking-[-0.045em]">
          API tests as YAML.
          <br />
          Run them anywhere.
        </h1>
        <p className="m-0 max-w-160 text-xl leading-[1.55] text-fg-secondary">{DESCRIPTION}</p>
        <div className="flex flex-wrap justify-center gap-3">
          <ButtonLink href="/docs/quick-start" variant="primary" size="lg">
            Get started
          </ButtonLink>
          <ButtonLink href="/docs" variant="secondary" size="lg">
            Read the docs
          </ButtonLink>
        </div>
        <div className="w-full max-w-160">
          <InstallTabs
            methods={['curl', 'brew', 'npm', 'cargo']}
            defaultId="curl"
            showNote={false}
          />
        </div>
      </section>

      <section aria-label="Example" className="mx-auto max-w-[1200px] px-6 pb-24">
        <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,460px),1fr))] overflow-hidden rounded-[14px] border border-line bg-surface">
          <div className="min-w-0 border-hairline md:border-r">
            <PaneHeader icon="file">api.yaml</PaneHeader>
            <Yaml source={API_YAML} />
          </div>
          <div className="min-w-0 bg-ground">
            <PaneHeader icon="terminal">terminal</PaneHeader>
            <pre className="m-0 overflow-x-auto px-[22px] py-5 font-mono text-[13.5px] leading-[1.75] text-fg-secondary focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-brand">
              <code className="font-mono">
                <span className="text-fg">$ curl-runner api.yaml</span>
                {'\n'}
                <span className="text-brand">ℹ</span> Found 1 YAML file(s){'\n\n'}
                <span className="text-success">✓</span> <span className="text-fg">List users</span>{' '}
                [api]{'\n'}
                {'   ├─ GET: https://api.example.com/users\n'}
                {'   ├─ ✓ Status: '}
                <span className="text-success">200</span>
                {'\n   └─ Duration: 84ms | 2.1 KB\n\n'}
                <span className="text-success">✓</span> <span className="text-fg">Create user</span>{' '}
                [api]{'\n'}
                {'   ├─ POST: https://api.example.com/users\n'}
                {'   ├─ ✓ Status: '}
                <span className="text-success">201</span>
                {'\n   └─ Duration: 112ms | 96.00 B\n\n'}
                Summary: <span className="text-success">2 requests completed successfully</span>
              </code>
            </pre>
          </div>
        </div>
        <p className="mt-3.5 mb-0 text-center text-[13px] text-fg-tertiary">
          Example request timings are illustrative.
        </p>
      </section>

      <section aria-label="Install anywhere" className="border-y border-hairline">
        <div className="mx-auto flex max-w-[1200px] flex-wrap items-center justify-center gap-x-10 gap-y-4 px-6 py-7 font-mono text-sm text-fg-secondary">
          <span className="font-sans text-fg-tertiary">Install with</span>
          {INSTALL_STRIP.map((name) => (
            <span key={name}>{name}</span>
          ))}
        </div>
      </section>

      <section id="features" className="mx-auto max-w-[1200px] px-6 pt-28 pb-12">
        <div className="mb-3">
          <Eyebrow>Features</Eyebrow>
        </div>
        <h2 className={`${H2} max-w-[22ch]`}>Everything an API suite needs. Nothing it doesn’t.</h2>
        <ul className="m-0 mt-14 grid list-none grid-cols-[repeat(auto-fit,minmax(min(100%,280px),1fr))] border-t border-l border-hairline p-0">
          {HOME_FEATURES.map((f) => (
            <li key={f.key} className="flex flex-col gap-2.5 border-r border-b border-hairline p-7">
              <p className="m-0 font-mono text-[13px] text-fg-tertiary">{f.key}</p>
              <h3 className="m-0 text-lg font-semibold tracking-[-0.01em]">{f.title}</h3>
              <p className="m-0 text-[15px] leading-[1.6] text-fg-secondary">{f.body}</p>
            </li>
          ))}
        </ul>
      </section>

      <section aria-label="Performance" className="mx-auto max-w-[1200px] px-6 py-24">
        <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,420px),1fr))] items-center gap-12">
          <div className="flex flex-col gap-5">
            <Eyebrow>Native</Eyebrow>
            <h2 className={H2}>
              libcurl inside.
              <br />
              Rust around it.
            </h2>
            <p className="m-0 max-w-136 text-[17px] leading-[1.65] text-fg-secondary">
              The same battle-tested transfer engine as curl, statically linked into a single
              binary. HTTP/2, the operating system’s trust store, and nothing to install first — not
              Node, not Bun, not even curl.
            </p>
          </div>
          <dl className="m-0 grid grid-cols-2 overflow-hidden rounded-[14px] border border-line">
            {TILES.map((t, i) => (
              <div
                key={t.value}
                className={`p-7 ${i % 2 === 0 ? 'border-r border-hairline' : ''} ${i < 2 ? 'border-b border-hairline' : ''}`}
              >
                <dt className="sr-only">{t.label}</dt>
                <dd className="m-0 text-[34px] font-semibold tracking-[-0.03em]">{t.value}</dd>
                <p aria-hidden="true" className="m-0 mt-1.5 text-sm text-fg-secondary">
                  {t.label}
                </p>
              </div>
            ))}
          </dl>
        </div>
        <p className="mt-4 mb-0 text-[13px] text-fg-tertiary">
          Median of repeated runs on an Apple M2 Max against a local server. Reproduce with{' '}
          <code className="font-mono text-fg-secondary">scripts/bench.sh</code>.
        </p>
      </section>

      <section aria-label="CI" className="border-t border-hairline">
        <div className="mx-auto grid max-w-[1200px] grid-cols-[repeat(auto-fit,minmax(min(100%,420px),1fr))] items-center gap-12 px-6 py-28">
          <CodePane filename=".github/workflows/api.yml">
            <Yaml source={CI_YAML} />
          </CodePane>
          <div className="flex flex-col gap-5">
            <Eyebrow>CI-ready</Eyebrow>
            <h2 className={H2}>Smoke tests that fail the build — for the right reasons.</h2>
            <p className="m-0 max-w-136 text-[17px] leading-[1.65] text-fg-secondary">
              Deterministic exit codes, failure thresholds, retries for flaky networks, and one JSON
              report for every file in the run. Drop it into any pipeline.
            </p>
          </div>
        </div>
      </section>

      <section aria-label="Get started" className="border-t border-hairline">
        <div className="mx-auto flex max-w-[1200px] flex-col items-center gap-6 px-6 py-30 text-center">
          <h2 className="m-0 text-[clamp(36px,5vw,60px)] leading-[1.04] font-bold tracking-[-0.04em]">
            Write your first suite in a minute.
          </h2>
          <div className="flex flex-wrap justify-center gap-3">
            <ButtonLink href="/docs/quick-start" variant="primary" size="lg">
              Get started
            </ButtonLink>
            <ButtonLink href={GITHUB_URL} variant="secondary" size="lg">
              View on GitHub
            </ButtonLink>
          </div>
        </div>
      </section>
    </>
  );
}
