import type { Metadata } from 'next';
import { type TokenRow, TokensView } from '@/components/TokensView';
import { requireTeam } from '@/lib/collections-data';
import { getTokens } from '@/lib/team-data';
import { dateThisYear, relativeTime, tokenExpiry } from '@/lib/time';
import { createToken, revokeToken } from './actions';

export const metadata: Metadata = { title: 'Tokens' };

export default async function TokensPage({ params }: { params: Promise<{ team: string }> }) {
  const { team } = await params;
  const current = await requireTeam(team);
  const tokens = await getTokens(team);
  const now = new Date();

  const rows: TokenRow[] = tokens.map((t) => {
    const expiry = tokenExpiry(t.expires_at, now);
    return {
      id: t.id,
      name: t.name,
      by: t.created_by,
      created: dateThisYear(t.created_at, now),
      expires: expiry.label,
      expiresSoon: expiry.soon,
      lastUsed: t.last_used_at ? relativeTime(t.last_used_at, now) : null,
    };
  });

  return (
    <TokensView
      team={team}
      teamName={current.name}
      tokens={rows}
      onCreate={createToken.bind(null, team)}
      onRevoke={revokeToken.bind(null, team)}
    />
  );
}
