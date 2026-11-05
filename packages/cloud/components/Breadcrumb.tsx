import Link from 'next/link';

export type Crumb = { label: string; href?: string };

/** "Collections / Payments API": every step but the last links back. */
export function Breadcrumb({ items }: { items: Crumb[] }) {
  return (
    <nav aria-label="Breadcrumb" className="flex flex-wrap gap-1.5 text-[12px] text-fg-subtle">
      {items.map((item, i) => (
        <span key={item.label} className="flex gap-1.5">
          {i > 0 ? <span aria-hidden="true">/</span> : null}
          {item.href ? (
            <Link href={item.href} className="text-fg-muted hover:text-fg">
              {item.label}
            </Link>
          ) : (
            <span aria-current="page" className="text-fg">
              {item.label}
            </span>
          )}
        </span>
      ))}
    </nav>
  );
}
