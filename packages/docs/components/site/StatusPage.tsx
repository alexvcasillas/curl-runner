import Link from 'next/link';
import { ButtonLink } from '@/components/site/ButtonLink';
import { SiteFooter } from '@/components/site/SiteFooter';
import { SiteHeader } from '@/components/site/SiteHeader';

const LINKS = [
  { href: '/docs', label: 'Documentation' },
  { href: '/docs/quick-start', label: 'Quick start' },
  { href: '/downloads', label: 'Downloads' },
  { href: '/changelog', label: 'Changelog' },
];

interface StatusPageProps {
  code: string;
  title: string;
  description: string;
}

/** Shared layout for the 404 and error pages. */
export function StatusPage({ code, title, description }: StatusPageProps) {
  return (
    <div className="flex min-h-screen flex-col bg-ground text-fg">
      <SiteHeader />
      <main className="mx-auto w-full max-w-[1200px] grow px-6 py-24">
        <p className="m-0 font-mono text-sm text-brand">{code}</p>
        <h1 className="mt-4 mb-0 text-[clamp(36px,6vw,56px)] leading-[1.05] font-bold tracking-[-0.035em]">
          {title}
        </h1>
        <p className="mt-5 mb-0 max-w-xl text-lg text-fg-secondary">{description}</p>
        <div className="mt-8">
          <ButtonLink href="/" variant="primary" size="lg">
            Back to home
          </ButtonLink>
        </div>
        <ul className="m-0 mt-12 flex list-none flex-wrap gap-x-6 p-0 text-sm">
          {LINKS.map((link) => (
            <li key={link.href}>
              <Link
                href={link.href}
                className="inline-flex h-11 items-center text-fg-secondary transition-colors hover:text-fg"
              >
                {link.label}
              </Link>
            </li>
          ))}
        </ul>
      </main>
      <SiteFooter />
    </div>
  );
}
