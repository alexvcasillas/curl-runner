import Link from 'next/link';
import { cx } from '@/lib/cx';
import { teamLetter } from '@/lib/initials';
import { external, routes } from '@/lib/routes';
import { buttonClasses } from './button-styles';
import { Card, CardFooter } from './Card';
import { CopyField } from './CopyField';
import { EmptyState } from './EmptyState';
import { FolderIcon } from './icons';
import { PageHeader } from './PageHeader';
import { Pill } from './Pill';

export type SharedRow = {
  key: string;
  name: string;
  /** The team's display name. */
  team: string;
  access: 'public' | 'restricted' | 'off';
  version: number;
  /** "2 hours ago". */
  updated: string;
  /** `curl-runner pull <share link>`. */
  command: string;
  /** The `curl-runner://` link. */
  studioHref: string;
};

/** Collections other teams shared with you, or the empty state. */
export function SharedView({ email, items }: { email: string; items: SharedRow[] }) {
  const domain = email.split('@')[1] ?? '';
  return (
    <div className="mx-auto flex w-full max-w-[960px] flex-col gap-6">
      <PageHeader title="Shared with me">
        Collections teams shared with {email} or with everyone at {domain}, plus public links you’ve
        opened while signed in.
      </PageHeader>

      {items.length === 0 ? (
        <EmptyState
          icon={<FolderIcon size={18} />}
          title="Nothing shared with you yet"
          className="py-14"
          actions={
            <Link href={routes.newTeam} className="text-[12px]">
              Publishing your own collections? Create a team
            </Link>
          }
        >
          When a team shares a collection with {email}, it shows up here. Got a link already? Open
          it, and it’ll be here next time.
        </EmptyState>
      ) : (
        <Card aria-label="Shared collections">
          {items.map((c) => (
            <div
              key={c.key}
              className="flex flex-col gap-3 border-b border-hairline last:border-b-0 px-6 py-[18px] max-sm:px-4"
            >
              <div className="flex flex-wrap items-start gap-3">
                <span
                  aria-hidden="true"
                  className="grid size-9 shrink-0 place-items-center rounded-[8px] bg-surface-300 font-semibold"
                >
                  {teamLetter(c.team)}
                </span>
                <span className="flex min-w-[200px] flex-1 flex-col gap-0.5">
                  <span className="flex flex-wrap items-center gap-2">
                    <span className="text-[14px] font-medium [overflow-wrap:anywhere]">
                      {c.name}
                    </span>
                    <Pill size="sm">{c.access === 'public' ? 'Public' : 'Restricted'}</Pill>
                  </span>
                  <span className="text-[12px] text-fg-muted">
                    Shared by {c.team} · <span className="font-mono">v{c.version}</span> · updated{' '}
                    {c.updated}
                  </span>
                </span>
                <a href={c.studioHref} className={cx(buttonClasses('primary', 'sm'))}>
                  Open in studio
                </a>
              </div>
              <div className="flex flex-wrap items-center gap-2 pl-12 max-sm:pl-0">
                <span className="text-[12px] text-fg-subtle">Not on a Mac?</span>
                <CopyField
                  value={c.command}
                  label={`the pull command for ${c.name}`}
                  className="min-w-0 flex-1 max-sm:basis-full"
                />
              </div>
            </div>
          ))}
          <CardFooter>
            <span className="min-w-0 flex-1">
              The studio checks for new versions every 15 minutes and asks before it updates your
              copy.
            </span>
            <a href={external.download}>Download the studio</a>
          </CardFooter>
        </Card>
      )}
    </div>
  );
}
