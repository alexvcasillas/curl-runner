'use client';

import { useRouter } from 'next/navigation';
import { routes } from '@/lib/routes';
import { Select } from './Select';

/** "Compare with v13": picks the older version to diff against. */
export function CompareSelect({
  team,
  slug,
  n,
  options,
  value,
}: {
  team: string;
  slug: string;
  /** The version being viewed. */
  n: number;
  /** Older versions, newest first. */
  options: number[];
  value: number;
}) {
  const router = useRouter();
  return (
    <div className="flex items-center gap-2 text-[12px] text-fg-muted">
      <span aria-hidden="true">Compare with</span>
      <Select
        aria-label="Compare with"
        value={String(value)}
        onChange={(e) => router.push(routes.version(team, slug, n, Number(e.target.value)))}
        options={options.map((m) => ({ value: String(m), label: `v${m}` }))}
        className="w-24 [&_select]:font-mono [&_select]:text-[12px]"
      />
    </div>
  );
}
