import type { Plan } from '@/lib/types';
import { EyeOffIcon, GitIcon, GlobeIcon, LockIcon } from './icons';
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

/** Where a collection is edited: a git repo through CI, or the cloud from the studio. */
export function SourceBadge({ source }: { source: 'git' | 'cloud' }) {
  return <Pill icon={<GitIcon size={12} />}>{source === 'git' ? 'git' : 'Cloud'}</Pill>;
}
