import type { Metadata } from 'next';
import { TeamSettings } from '@/components/TeamSettings';
import { getInfo, getMe } from '@/lib/app-data';
import { getCollections, requireTeam } from '@/lib/collections-data';
import { env } from '@/lib/env';
import { getBillingSummary, getTeam, getTokens } from '@/lib/team-data';
import { leaveTeam } from '../members/actions';
import { deleteTeam, renameTeam } from './actions';

export const metadata: Metadata = { title: 'Settings' };

export default async function SettingsPage({ params }: { params: Promise<{ team: string }> }) {
  const { team } = await params;
  const [current, info, me] = await Promise.all([requireTeam(team), getInfo(), getMe()]);
  const isOwner = current.role === 'owner';
  // What a delete takes with it. Only owners can delete, so only they need the counts.
  const [detail, collections, tokens, summary] = await Promise.all([
    getTeam(team),
    isOwner ? getCollections(team) : [],
    isOwner ? getTokens(team) : [],
    isOwner && info.billing ? getBillingSummary(team) : null,
  ]);
  const owners = detail.members.filter((m) => m.role === 'owner');

  return (
    <TeamSettings
      team={team}
      name={detail.name}
      owner={isOwner}
      me={{ id: me.user.id, email: me.user.email }}
      soleOwner={owners.length === 1 && owners[0].user_id === me.user.id}
      slugHost={new URL(env().PUBLIC_APP_URL).host}
      collections={collections.length}
      tokens={tokens.length}
      // A live subscription, whoever the plan comes from: a grant doesn't stop its charges.
      subscription={summary?.has_subscription === true}
      onRename={renameTeam.bind(null, team)}
      onDelete={deleteTeam.bind(null, team)}
      onLeave={leaveTeam.bind(null, team)}
    />
  );
}
