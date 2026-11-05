import type { Metadata } from 'next';
import { MembersView } from '@/components/MembersView';
import { getInfo, getMe } from '@/lib/app-data';
import { requireTeam } from '@/lib/collections-data';
import { seatCostNote } from '@/lib/members';
import { getBillingSummary, getPrices, getTeam } from '@/lib/team-data';
import { expiresIn } from '@/lib/time';
import { cancelInvite, changeRole, inviteMember, leaveTeam, removeMember } from './actions';

export const metadata: Metadata = { title: 'Members' };

export default async function MembersPage({ params }: { params: Promise<{ team: string }> }) {
  const { team } = await params;
  const [current, info, me] = await Promise.all([requireTeam(team), getInfo(), getMe()]);
  const detail = await getTeam(team);
  const now = new Date();

  // Prices and the subscription only exist with billing on; off, both are a 404.
  const [prices, summary] =
    owner(detail.role) && info.billing && detail.plan === 'team'
      ? await Promise.all([getPrices(), getBillingSummary(team)])
      : [null, null];

  return (
    <MembersView
      team={team}
      teamName={current.name}
      me={{ id: me.user.id, email: me.user.email }}
      owner={owner(detail.role)}
      plan={detail.plan}
      billing={info.billing}
      members={detail.members}
      invites={detail.pending_invites.map((i) => ({
        id: i.id,
        email: i.email,
        role: i.role,
        expires: expiresIn(i.expires_at, now),
      }))}
      costNote={seatCostNote(info.billing, detail.plan, prices, summary)}
      onInvite={inviteMember.bind(null, team)}
      onRole={changeRole.bind(null, team)}
      onRemove={removeMember.bind(null, team)}
      onCancel={cancelInvite.bind(null, team)}
      onLeave={leaveTeam.bind(null, team)}
    />
  );
}

function owner(role: string): boolean {
  return role === 'owner';
}
