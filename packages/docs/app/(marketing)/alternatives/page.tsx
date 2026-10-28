import type { Metadata } from 'next';
import { ButtonLink } from '@/components/site/ButtonLink';
import { Eyebrow } from '@/components/site/Eyebrow';
import { GITHUB_URL, SiteHeader } from '@/components/site/SiteHeader';
import { COMPARISON_CARDS, COMPARISON_CHECKED, COMPARISON_ROWS } from '@/lib/alternatives';

const DESCRIPTION =
  'How curl-runner compares with Postman, Insomnia, curl and HTTPie, including when each of them is the better choice.';

export const metadata: Metadata = {
  title: 'curl-runner alternatives: Postman, Insomnia, curl, HTTPie',
  description: DESCRIPTION,
  alternates: { canonical: '/alternatives' },
};

const H2 = 'm-0 text-[clamp(30px,4vw,44px)] leading-[1.1] font-bold tracking-[-0.035em] text-fg';
const CELL = 'border-b border-hairline px-5 py-4 align-top';

export default function AlternativesPage() {
  return (
    <>
      <SiteHeader active="alternatives" />

      <section className="mx-auto max-w-[1200px] px-6 pt-24 pb-14">
        <Eyebrow>Alternatives</Eyebrow>
        <h1 className="mt-4 mb-0 max-w-[18ch] text-[clamp(40px,6vw,72px)] leading-[1.02] font-bold tracking-[-0.045em] text-fg">
          The right tool depends on the job.
        </h1>
        <p className="mt-6 mb-0 max-w-2xl text-[19px] leading-[1.6] text-fg-secondary">
          curl-runner is built for API suites that live in your repo and run in CI. Here is how it
          compares with the tools you might already use, including when they are the better choice.
        </p>
      </section>

      <section aria-label="Comparison" className="mx-auto max-w-[1200px] px-6 pb-6">
        <div
          className="overflow-x-auto rounded-[14px] border border-line focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-brand"
          // biome-ignore lint/a11y/noNoninteractiveTabindex: keyboard-scrollable region
          tabIndex={0}
          role="region"
          aria-label="Comparison table"
        >
          <table className="w-full min-w-[880px] border-collapse text-sm">
            <thead>
              <tr className="text-left">
                <th
                  scope="col"
                  className="w-[16%] border-b border-line px-5 py-4 font-medium text-fg-tertiary"
                >
                  <span className="sr-only">Feature</span>
                </th>
                <th
                  scope="col"
                  className="border-b border-line bg-surface px-5 py-4 font-semibold text-fg"
                >
                  curl-runner
                </th>
                {['Postman', 'Insomnia', 'curl', 'HTTPie'].map((t) => (
                  <th
                    key={t}
                    scope="col"
                    className="border-b border-line px-5 py-4 font-semibold text-fg"
                  >
                    {t}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {COMPARISON_ROWS.map((r) => (
                <tr key={r.label}>
                  <th scope="row" className={`${CELL} text-left font-medium text-fg-secondary`}>
                    {r.label}
                  </th>
                  <td className={`${CELL} bg-surface text-fg`}>{r.curlRunner}</td>
                  <td className={`${CELL} text-fg-secondary`}>{r.postman}</td>
                  <td className={`${CELL} text-fg-secondary`}>{r.insomnia}</td>
                  <td className={`${CELL} text-fg-secondary`}>{r.curl}</td>
                  <td className={`${CELL} text-fg-secondary`}>{r.httpie}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="mt-3.5 mb-0 text-[13px] text-fg-tertiary">
          Checked {COMPARISON_CHECKED} against each tool’s official docs and repository. “Not
          documented” means we found no statement either way. Spot something out of date?{' '}
          <a
            href={`${GITHUB_URL}/issues/new`}
            className="text-fg underline underline-offset-2 hover:text-white"
          >
            Open an issue
          </a>
          .
        </p>
      </section>

      <section aria-label="Side by side" className="mx-auto max-w-[1200px] px-6 py-24">
        <h2 className={`${H2} mb-10`}>Side by side</h2>
        <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,480px),1fr))] gap-4">
          {COMPARISON_CARDS.map((c) => (
            <article
              key={c.tool}
              className="flex flex-col gap-[18px] rounded-[14px] border border-line p-7"
            >
              <div>
                <h3 className="m-0 text-xl font-semibold tracking-[-0.015em] text-fg">
                  curl-runner vs {c.tool}
                </h3>
                <p className="mt-1.5 mb-0 text-sm text-fg-tertiary">{c.what}</p>
              </div>
              <div className="grid grid-cols-1 gap-5 text-sm leading-[1.6] sm:grid-cols-2">
                <div>
                  <p className="m-0 mb-2 font-mono text-xs text-brand uppercase">
                    Pick curl-runner for
                  </p>
                  <p className="m-0 text-fg">{c.pickCurlRunner}</p>
                </div>
                <div>
                  <p className="m-0 mb-2 font-mono text-xs text-fg-tertiary uppercase">
                    Pick {c.tool} for
                  </p>
                  <p className="m-0 text-fg-secondary">{c.pickTool}</p>
                </div>
              </div>
            </article>
          ))}
        </div>
      </section>

      <section aria-label="Get started" className="border-t border-hairline">
        <div className="mx-auto flex max-w-[1200px] flex-col items-center gap-6 px-6 py-28 text-center">
          <h2 className="m-0 text-[clamp(34px,5vw,56px)] leading-[1.04] font-bold tracking-[-0.04em] text-fg">
            Try it on one endpoint.
          </h2>
          <p className="m-0 max-w-136 text-lg text-fg-secondary">
            Install, write a five-line YAML file, run it. Keep it if it earns its place.
          </p>
          <div className="flex flex-wrap justify-center gap-3">
            <ButtonLink href="/docs/quick-start" variant="primary" size="lg">
              Quick start
            </ButtonLink>
            <ButtonLink href="/downloads" variant="secondary" size="lg">
              Install
            </ButtonLink>
          </div>
        </div>
      </section>
    </>
  );
}
