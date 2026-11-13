import Link from 'next/link';
import { GITHUB_URL } from '@/components/site/SiteHeader';
import { CLOUD_SIGN_IN_URL } from '@/lib/site';

const COLUMNS: { title: string; links: { label: string; href: string }[] }[] = [
  {
    title: 'Product',
    links: [
      { label: 'Features', href: '/features/parallel-execution' },
      { label: 'Collections', href: '/collections' },
      { label: 'Downloads', href: '/downloads' },
      { label: 'Changelog', href: '/changelog' },
      { label: 'Sign in to Cloud', href: CLOUD_SIGN_IN_URL },
    ],
  },
  {
    title: 'Resources',
    links: [
      { label: 'Docs', href: '/docs' },
      { label: 'Alternatives', href: '/alternatives' },
      { label: 'Tools database', href: '/tools-database' },
    ],
  },
  {
    title: 'Community',
    links: [
      { label: 'GitHub', href: GITHUB_URL },
      { label: 'Issues', href: `${GITHUB_URL}/issues` },
    ],
  },
  {
    title: 'Legal',
    links: [
      { label: 'Privacy', href: '/privacy' },
      { label: 'Terms', href: '/terms' },
    ],
  },
];

// The ::before extends each 20px line to a 44px touch target.
const LINK_CLASS =
  "relative inline-flex items-center text-fg-secondary transition-colors before:absolute before:inset-x-0 before:-inset-y-3 before:content-[''] hover:text-fg";

export function SiteFooter() {
  return (
    <footer className="border-t border-hairline">
      <div className="mx-auto flex max-w-[1200px] flex-wrap gap-x-16 gap-y-8 px-6 py-12 text-sm text-fg-secondary">
        <div className="flex flex-[1_1_240px] flex-col gap-2.5">
          <span className="font-semibold text-fg">curl-runner</span>
          <span>MIT licensed. Built on libcurl.</span>
        </div>
        {COLUMNS.map((column) => (
          <div key={column.title} className="flex flex-col gap-2.5">
            <h2 className="text-sm font-normal text-fg">{column.title}</h2>
            <ul className="flex flex-col gap-2.5">
              {column.links.map((link) => (
                <li key={link.href}>
                  {link.href.startsWith('http') ? (
                    <a href={link.href} className={LINK_CLASS}>
                      {link.label}
                    </a>
                  ) : (
                    <Link href={link.href} className={LINK_CLASS}>
                      {link.label}
                    </Link>
                  )}
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </footer>
  );
}
