import { teamLetter } from '@/lib/initials';
import { external } from '@/lib/routes';
import { CopyField } from './CopyField';
import { LockIcon } from './icons';
import { OpenInStudio } from './OpenInStudio';
import { Pill } from './Pill';

export type ShareLandingProps = {
  team: string;
  name: string;
  description: string | null;
  access: 'public' | 'restricted';
  requests: number;
  version: number;
  /** "2 hours ago". */
  updated: string;
  studioHref: string;
  /** `curl-runner pull <share link>`. */
  command: string;
};

function Stat({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex min-w-[90px] flex-1 flex-col gap-0.5 border-r border-hairline px-4 py-3 last:border-r-0 max-sm:px-3">
      <span className="text-[12px] text-fg-subtle">{label}</span>
      <span className="font-medium tabular-nums">{children}</span>
    </div>
  );
}

/**
 * What a customer sees at a share link they can open: the collection and the
 * ways to get it.
 */
export function ShareLanding({
  team,
  name,
  description,
  access,
  requests,
  version,
  updated,
  studioHref,
  command,
}: ShareLandingProps) {
  return (
    <article className="min-w-0 overflow-hidden rounded-[12px] border border-hairline bg-bg">
      <div className="flex flex-col gap-3.5 px-7 pt-7 pb-5 max-sm:px-4 max-sm:pt-6">
        <div className="flex items-center gap-2.5">
          <span
            aria-hidden="true"
            className="grid size-8 shrink-0 place-items-center rounded-[8px] bg-surface-300 font-semibold"
          >
            {teamLetter(team)}
          </span>
          <span className="min-w-0 text-fg-muted [overflow-wrap:anywhere]">
            {team} shared a collection with you
          </span>
        </div>
        <div className="flex flex-col gap-1">
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em] [overflow-wrap:anywhere] max-sm:text-[22px] max-sm:leading-7">
              {name}
            </h1>
            <Pill>{access === 'public' ? 'Public link' : 'Restricted'}</Pill>
          </div>
          {description ? (
            <p className="m-0 text-fg-muted [overflow-wrap:anywhere]">{description}</p>
          ) : null}
        </div>
      </div>

      <div className="flex flex-wrap border-y border-hairline">
        <Stat label="Requests">{requests}</Stat>
        <Stat label="Version">
          <span className="font-mono">v{version}</span>
        </Stat>
        <Stat label="Updated">{updated}</Stat>
      </div>

      <div className="flex flex-col gap-3 px-7 pt-5 pb-6 max-sm:px-4">
        <OpenInStudio href={studioHref} downloadHref={external.download} />
        <div className="mt-1 flex items-center gap-3 text-[12px] text-fg-subtle">
          <span className="h-px flex-1 bg-hairline" />
          Not on a Mac?
          <span className="h-px flex-1 bg-hairline" />
        </div>
        <CopyField value={command} label="the pull command" />
        <span className="text-[12px] text-fg-subtle">
          Installs with <span className="font-mono text-fg-muted">npm i -g curl-runner</span> or
          Homebrew. Pulling again later fetches the newest version.
        </span>
      </div>

      <div className="flex items-start gap-2.5 border-t border-hairline bg-raised px-7 py-3 text-[12px] leading-[18px] text-fg-subtle max-sm:px-4">
        <LockIcon size={13} className="mt-0.5 shrink-0" />
        <span>
          You fill in your own values, like API keys. Secrets stay on your machine, and {team} never
          sees them.
        </span>
      </div>
    </article>
  );
}
