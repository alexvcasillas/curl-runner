import type { Metadata } from 'next';
import { AccountView, type ProviderRow, type SessionRow } from '@/components/AccountView';
import { getIdentities, getSessions } from '@/lib/account-data';
import { getInfo, getMe } from '@/lib/app-data';
import { PROVIDER_NAMES } from '@/lib/sign-in';
import { formatDate, relativeTimeInline } from '@/lib/time';
import { leaveTeam } from '../t/[team]/members/actions';
import { revokeSession } from './actions';

export const metadata: Metadata = { title: 'Account' };

export default async function AccountPage() {
  const [me, info, identities, sessions] = await Promise.all([
    getMe(),
    getInfo(),
    getIdentities(),
    getSessions(),
  ]);
  const now = new Date();

  const linked = new Map(identities.map((i) => [i.provider, i]));
  const providers: ProviderRow[] = [{ provider: 'email', detail: me.user.email, linked: true }];
  for (const p of ['github', 'google'] as const) {
    const found = linked.get(p);
    if (found) {
      providers.push({
        provider: p,
        detail: `Linked ${formatDate(found.created_at)} with ${found.email}`,
        linked: true,
      });
    } else if (info.sign_in.includes(p)) {
      providers.push({
        provider: p,
        detail: `Not linked. Sign in with ${PROVIDER_NAMES[p]} once, using ${me.user.email}, to link it.`,
        linked: false,
      });
    }
  }

  const rows: SessionRow[] = sessions.map((s) => ({
    id: s.id,
    kind: s.kind,
    current: s.current,
    label: s.label || (s.kind === 'web' ? 'Browser' : 'Device'),
    meta: `${
      s.current ? 'Active now' : `Last used ${relativeTimeInline(s.last_used_at, now)}`
    } · expires ${formatDate(s.expires_at)}`,
  }));

  return (
    <AccountView
      email={me.user.email}
      name={me.user.name}
      providers={providers}
      sessions={rows}
      teams={me.teams}
      showPlan={info.billing}
      onRevoke={revokeSession}
      onLeave={leaveTeam}
    />
  );
}
