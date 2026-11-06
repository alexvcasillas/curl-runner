import { cx } from '@/lib/cx';
import { StatTile, type StatTone } from './StatTile';

/** A labelled hairline box, the way the admin boards draw a section. */
export function AdminCard({
  label,
  children,
  className,
}: {
  label: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <section
      aria-label={label}
      className={cx(
        'min-w-0 overflow-hidden rounded-[8px] border border-hairline bg-bg',
        className,
      )}
    >
      {children}
    </section>
  );
}

export function CardHead({
  title,
  id,
  aside,
}: {
  title: string;
  id?: string;
  aside?: React.ReactNode;
}) {
  return (
    <div className="flex min-h-11 flex-wrap items-center gap-2 border-b border-hairline px-4 py-1.5">
      <h2 id={id} className="m-0 mr-auto text-[14px] font-semibold">
        {title}
      </h2>
      {aside}
    </div>
  );
}

export function CardFoot({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex min-h-10 flex-wrap items-center justify-between gap-3 border-t border-hairline bg-raised px-4 py-1.5 text-[12px] text-fg-muted">
      {children}
    </div>
  );
}

const LG_SPAN = ['', '', 'lg:col-span-2', 'lg:col-span-3'];

/** The last tile stretches over the cells a short final row leaves empty, so the grid fills evenly. */
function lastTileSpan(n: number): string {
  const cols = n % 4 === 0 ? 4 : 3;
  const rem = n % cols;
  return cx(n % 2 === 1 && 'sm:col-span-2', rem > 0 && LG_SPAN[cols - rem + 1]);
}

export type Tile = { label: string; value: string; sub?: string; tone?: StatTone };

/** The "Key numbers" strip: stat tiles in a card. */
export function StatGrid({ tiles }: { tiles: Tile[] }) {
  return (
    <section
      aria-label="Key numbers"
      className="overflow-hidden rounded-[8px] border border-hairline bg-bg"
    >
      <div
        data-tiles={tiles.length}
        className={cx(
          '-mr-px -mb-px grid grid-cols-1 sm:grid-cols-2',
          tiles.length % 4 === 0 ? 'lg:grid-cols-4' : 'lg:grid-cols-3',
        )}
      >
        {tiles.map((t, i) => (
          <StatTile
            key={t.label}
            {...t}
            className={i === tiles.length - 1 ? lastTileSpan(tiles.length) : undefined}
          />
        ))}
      </div>
    </section>
  );
}
