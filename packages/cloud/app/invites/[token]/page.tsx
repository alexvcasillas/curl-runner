import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { InviteCard } from '@/components/InviteCard';
import { PublicShell } from '@/components/PublicShell';
import { getInvite, getViewer } from '@/lib/public-data';
import { expiresIn } from '@/lib/time';
import { isInviteToken } from '@/lib/tokens';
import { acceptInvite } from './actions';

// Never the token: titles end up in history and shares.
export const metadata: Metadata = {
  title: 'Join a team',
  robots: { index: false, follow: false },
};

export default async function InvitePage({ params }: { params: Promise<{ token: string }> }) {
  const { token } = await params;
  if (!isInviteToken(token)) {
    notFound();
  }
  const invite = await getInvite(token);
  if (!invite) {
    notFound();
  }
  const viewer = await getViewer();

  return (
    <PublicShell cloud width="invite">
      <InviteCard
        teamName={invite.team_name}
        inviter={invite.inviter_name}
        joinAs={invite.role}
        email={invite.email}
        expires={expiresIn(invite.expires_at)}
        viewer={viewer ? { email: viewer } : null}
        returnTo={`/invites/${token}`}
        onJoin={acceptInvite.bind(null, token)}
      />
    </PublicShell>
  );
}
