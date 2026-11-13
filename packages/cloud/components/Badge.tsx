import { gitSummary, sourceKind } from '@/lib/collections';
import type { GitMeta, Plan, Source } from '@/lib/types';
import { CloudIcon, EyeOffIcon, FolderIcon, GitIcon, GlobeIcon, LockIcon } from './icons';
import { Pill } from './Pill';

const PLAN_NAMES: Record<Plan, string> = { free: 'Free', team: 'Team' };

/** A team's plan. Render it only when billing is on. */
export function PlanBadge({ plan, size = 'sm' }: { plan: Plan; size?: 'sm' | 'md' }) {
  return <Pill size={size}>{PLAN_NAMES[plan]}</Pill>;
}

export type Access = 'public' | 'restricted' | 'off';

const ACCESS = {
  public: { label: 'Public', Icon: GlobeIcon },
  restricted: { label: 'Restricted', Icon: LockIcon },
  off: { label: 'Off', Icon: EyeOffIcon },
} as const;

/** Who can open a collection's share link. */
export function AccessBadge({ access }: { access: Access }) {
  const { label, Icon } = ACCESS[access];
  return (
    <Pill tone={access === 'off' ? 'muted' : 'strong'} icon={<Icon size={12} />}>
      {label}
    </Pill>
  );
}

const SOURCES = {
  git: { label: 'Git', Icon: GitIcon },
  files: { label: 'Local files', Icon: FolderIcon },
  cloud: { label: 'Cloud', Icon: CloudIcon },
} as const;

/**
 * Where a collection's versions come from: `Git` (published from files that
 * came with git details), `Local files` (published from files without), or
 * `Cloud` (synced from the studio). The hover says which repo and commit.
 */
export function SourceBadge({ source, git }: { source: Source; git?: GitMeta | null }) {
  const kind = sourceKind(source, git);
  const { label, Icon } = SOURCES[kind];
  const title = kind === 'git' ? (gitSummary(git) ?? undefined) : undefined;
  return (
    <span title={title} className="inline-flex">
      <Pill icon={<Icon size={12} />}>{label}</Pill>
    </span>
  );
}
