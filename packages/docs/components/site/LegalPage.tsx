import type { ReactNode } from 'react';
import { Eyebrow } from '@/components/site/Eyebrow';
import { SiteHeader } from '@/components/site/SiteHeader';
import { CONTACT_EMAIL, LEGAL_UPDATED } from '@/lib/legal';

export const LEGAL_LINK =
  'text-fg underline underline-offset-2 hover:text-brand focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand';

export type LegalSection = { id: string; title: string; body: ReactNode };

/** The privacy policy and terms layout: title, table of contents, sections. */
export function LegalPage({
  title,
  intro,
  sections,
}: {
  title: string;
  intro: string;
  sections: LegalSection[];
}) {
  return (
    <>
      <SiteHeader />
      <main id="main-content" className="mx-auto max-w-[1200px] px-6 pt-24 pb-24">
        <Eyebrow>Legal</Eyebrow>
        <h1 className="mt-4 mb-0 text-[clamp(36px,5vw,56px)] leading-[1.05] font-bold tracking-[-0.04em]">
          {title}
        </h1>
        <p className="mt-5 mb-0 max-w-160 text-[18px] leading-[1.6] text-fg-secondary">{intro}</p>
        <p className="mt-3 mb-0 text-sm text-fg-secondary">Last updated {LEGAL_UPDATED}</p>
        <div className="mt-14 grid gap-12 lg:grid-cols-[220px_1fr]">
          <nav aria-label="On this page" className="lg:sticky lg:top-8 lg:self-start">
            <p className="m-0 mb-3 text-sm text-fg">On this page</p>
            <ul className="m-0 flex list-none flex-col gap-2 p-0 text-sm">
              {sections.map((s) => (
                <li key={s.id}>
                  <a href={`#${s.id}`} className="text-fg-secondary hover:text-fg">
                    {s.title}
                  </a>
                </li>
              ))}
            </ul>
          </nav>
          <div className="flex max-w-[68ch] flex-col gap-12 text-[16px] leading-[1.7] text-fg-secondary">
            {sections.map((s) => (
              <section
                key={s.id}
                id={s.id}
                aria-labelledby={`${s.id}-title`}
                className="scroll-mt-8"
              >
                <h2
                  id={`${s.id}-title`}
                  className="m-0 mb-4 text-[24px] font-bold tracking-[-0.02em] text-fg"
                >
                  {s.title}
                </h2>
                <div className="flex flex-col gap-4 [&_li]:mt-1.5 [&_ul]:m-0 [&_ul]:list-disc [&_ul]:pl-5">
                  {s.body}
                </div>
              </section>
            ))}
          </div>
        </div>
      </main>
    </>
  );
}

export function ContactLink() {
  return (
    <a href={`mailto:${CONTACT_EMAIL}`} className={`${LEGAL_LINK} whitespace-nowrap`}>
      {CONTACT_EMAIL}
    </a>
  );
}
